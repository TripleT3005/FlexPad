use core::fmt::{self, Display, Write};

use crate::buttons::{DEFAULT_DEBOUNCE_MS, MAX_DEBOUNCE_MS};
use crate::encoder::{DEFAULT_STEPS_PER_DETENT, Direction};
use crate::json::{JsonError, JsonStr, Parser};
use crate::macros::{self, MacroError};

// Constants

pub const INPUT_PINS: [u8; 25] = [
    1, 2, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 21, 38, 39, 40, 41, 42, 47, 48,
];
pub const MAX_CONFIG_SIZE: usize = 16 * 1024;
pub const MACRO_STORAGE_SIZE: usize = 8 * 1024;
pub const MAX_ENCODERS: usize = INPUT_PINS.len() / 2;
pub const SUPPORTED_FORMAT: u8 = 2;
const MAX_MACROS: usize = INPUT_PINS.len() + MAX_ENCODERS * 2;
const MAX_TILE_ENTRIES: usize = 4;

// Errors

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigError {
    TooLarge,
    InvalidJson(usize),
    InvalidStructure(&'static str),
    UnsupportedFormat,
    UnknownKind,
    UnknownRole,
    UnknownFunction,
    InvalidParameter(&'static str),
    InvalidPin,
    UnusablePin(u8),
    DuplicatePin(u8),
    InvalidMacro(u8),
    MacrosTooLarge,
}

impl Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::TooLarge => f.write_str("too_large"),
            ConfigError::InvalidJson(position) => write!(f, "invalid_json:{position}"),
            ConfigError::InvalidStructure(what) => write!(f, "invalid_structure:{what}"),
            ConfigError::UnsupportedFormat => f.write_str("unsupported_format"),
            ConfigError::UnknownKind => f.write_str("unknown_kind"),
            ConfigError::UnknownRole => f.write_str("unknown_role"),
            ConfigError::UnknownFunction => f.write_str("unknown_function"),
            ConfigError::InvalidParameter(name) => write!(f, "invalid_parameter:{name}"),
            ConfigError::InvalidPin => f.write_str("invalid_pin"),
            ConfigError::UnusablePin(pin) => write!(f, "unusable_pin:{pin}"),
            ConfigError::DuplicatePin(pin) => write!(f, "duplicate_pin:{pin}"),
            ConfigError::InvalidMacro(pin) => write!(f, "invalid_macro:{pin}"),
            ConfigError::MacrosTooLarge => f.write_str("macros_too_large"),
        }
    }
}

impl From<JsonError> for ConfigError {
    fn from(error: JsonError) -> Self {
        ConfigError::InvalidJson(error.position)
    }
}

// Layout

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PinRole {
    Unused,
    Button { debounce_ms: u8 },
    EncoderA(u8),
    EncoderB(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EncoderSetup {
    pub a: u8,
    pub b: u8,
    pub steps_per_detent: u8,
    pub reverse: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Layout {
    pub roles: [PinRole; INPUT_PINS.len()],
    pub encoders: [EncoderSetup; MAX_ENCODERS],
    pub encoder_count: usize,
}

impl Layout {
    pub const fn new() -> Self {
        Self {
            roles: [PinRole::Unused; INPUT_PINS.len()],
            encoders: [EncoderSetup {
                a: 0,
                b: 0,
                steps_per_detent: DEFAULT_STEPS_PER_DETENT,
                reverse: false,
            }; MAX_ENCODERS],
            encoder_count: 0,
        }
    }

    pub fn role(&self, pin: u8) -> PinRole {
        pin_index(pin).map_or(PinRole::Unused, |index| self.roles[index])
    }

    pub fn encoders(&self) -> &[EncoderSetup] {
        &self.encoders[..self.encoder_count]
    }
}

impl Default for Layout {
    fn default() -> Self {
        Self::new()
    }
}

pub fn pin_index(pin: u8) -> Option<usize> {
    INPUT_PINS.iter().position(|&candidate| candidate == pin)
}

// Bindings

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trigger {
    Press(u8),
    Turn(u8, Direction),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MacroEntry {
    trigger: Trigger,
    start: usize,
    end: usize,
}

pub struct Bindings {
    layout: Layout,
    entries: [MacroEntry; MAX_MACROS],
    count: usize,
    text: [u8; MACRO_STORAGE_SIZE],
    used: usize,
}

impl Bindings {
    pub const fn new() -> Self {
        Self {
            layout: Layout::new(),
            entries: [MacroEntry {
                trigger: Trigger::Press(0),
                start: 0,
                end: 0,
            }; MAX_MACROS],
            count: 0,
            text: [0; MACRO_STORAGE_SIZE],
            used: 0,
        }
    }

    pub fn clear(&mut self) {
        self.layout = Layout::new();
        self.count = 0;
        self.used = 0;
    }

    pub fn layout(&self) -> &Layout {
        &self.layout
    }

    pub fn len(&self) -> usize {
        self.count
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    pub fn macro_for(&self, trigger: Trigger) -> Option<&str> {
        self.entries[..self.count]
            .iter()
            .find(|entry| entry.trigger == trigger)
            .map(|entry| {
                core::str::from_utf8(&self.text[entry.start..entry.end]).unwrap_or_default()
            })
    }

    pub fn macro_for_pin(&self, pin: u8) -> Option<&str> {
        self.macro_for(Trigger::Press(pin))
    }

    fn claim_pin(&mut self, pin: u8, role: PinRole) -> Result<(), ConfigError> {
        let index = pin_index(pin).ok_or(ConfigError::UnusablePin(pin))?;
        if self.layout.roles[index] != PinRole::Unused {
            return Err(ConfigError::DuplicatePin(pin));
        }
        self.layout.roles[index] = role;
        Ok(())
    }

    fn add_encoder(&mut self, setup: EncoderSetup) -> Result<u8, ConfigError> {
        let index = self.layout.encoder_count;
        let number = u8::try_from(index).map_err(|_| ConfigError::InvalidStructure("encoders"))?;
        self.claim_pin(setup.a, PinRole::EncoderA(number))?;
        self.claim_pin(setup.b, PinRole::EncoderB(number))?;
        self.layout.encoders[index] = setup;
        self.layout.encoder_count += 1;
        Ok(number)
    }

    fn check_macro(
        &mut self,
        text: JsonStr<'_>,
        error_pin: u8,
    ) -> Result<(usize, usize), ConfigError> {
        let start = self.used;
        let mut writer = SliceWriter {
            buffer: &mut self.text[start..],
            length: 0,
        };
        text.unescape_into(&mut writer)
            .map_err(|_| ConfigError::MacrosTooLarge)?;
        let end = start + writer.length;
        let unescaped = core::str::from_utf8(&self.text[start..end]).unwrap_or_default();
        let valid = macros::actions(unescaped)
            .all(|action| matches!(action, Ok(_) | Err(MacroError::Unsupported(_))));
        if valid {
            Ok((start, end))
        } else {
            Err(ConfigError::InvalidMacro(error_pin))
        }
    }

    fn add_macro(
        &mut self,
        trigger: Option<Trigger>,
        text: JsonStr<'_>,
        error_pin: u8,
    ) -> Result<(), ConfigError> {
        let (start, end) = self.check_macro(text, error_pin)?;
        let Some(trigger) = trigger else {
            return Ok(());
        };
        if start == end {
            return Ok(());
        }
        self.entries[self.count] = MacroEntry {
            trigger,
            start,
            end,
        };
        self.count += 1;
        self.used = end;
        Ok(())
    }
}

impl Default for Bindings {
    fn default() -> Self {
        Self::new()
    }
}

struct SliceWriter<'a> {
    buffer: &'a mut [u8],
    length: usize,
}

impl Write for SliceWriter<'_> {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        let end = self.length + text.len();
        let target = self.buffer.get_mut(self.length..end).ok_or(fmt::Error)?;
        target.copy_from_slice(text.as_bytes());
        self.length = end;
        Ok(())
    }
}

// Tiles

#[derive(Clone, Copy)]
struct Entries<T: Copy, const N: usize> {
    items: [Option<T>; N],
    count: usize,
}

impl<T: Copy, const N: usize> Entries<T, N> {
    const fn new() -> Self {
        Self {
            items: [None; N],
            count: 0,
        }
    }

    fn push(&mut self, item: T) -> Result<(), ConfigError> {
        let slot = self
            .items
            .get_mut(self.count)
            .ok_or(ConfigError::InvalidStructure("tile"))?;
        *slot = Some(item);
        self.count += 1;
        Ok(())
    }

    fn iter(&self) -> impl Iterator<Item = T> + '_ {
        self.items[..self.count].iter().flatten().copied()
    }
}

struct Tile<'a> {
    kind: Option<JsonStr<'a>>,
    pins: Entries<(JsonStr<'a>, Option<u8>), MAX_TILE_ENTRIES>,
    functions: Entries<(JsonStr<'a>, JsonStr<'a>), MAX_TILE_ENTRIES>,
    legacy_slots: Entries<(Option<u8>, JsonStr<'a>), MAX_TILE_ENTRIES>,
    debounce_ms: u8,
    steps_per_detent: u8,
    reverse: bool,
}

impl<'a> Tile<'a> {
    fn new() -> Self {
        Self {
            kind: None,
            pins: Entries::new(),
            functions: Entries::new(),
            legacy_slots: Entries::new(),
            debounce_ms: DEFAULT_DEBOUNCE_MS,
            steps_per_detent: DEFAULT_STEPS_PER_DETENT,
            reverse: false,
        }
    }

    fn pin(&self, role: &str) -> Option<u8> {
        self.pins
            .iter()
            .find(|(name, _)| name.eq_str(role))
            .and_then(|(_, pin)| pin)
    }

    fn check_names(&self, roles: &[&str], functions: &[&str]) -> Result<(), ConfigError> {
        if self
            .pins
            .iter()
            .any(|(name, _)| !roles.iter().any(|role| name.eq_str(role)))
        {
            return Err(ConfigError::UnknownRole);
        }
        if self
            .functions
            .iter()
            .any(|(name, _)| !functions.iter().any(|function| name.eq_str(function)))
        {
            return Err(ConfigError::UnknownFunction);
        }
        Ok(())
    }

    fn apply(&self, bindings: &mut Bindings) -> Result<(), ConfigError> {
        match self.kind {
            None => self.apply_legacy(bindings),
            Some(kind) if kind.eq_str("button") => self.apply_button(bindings),
            Some(kind) if kind.eq_str("encoder") => self.apply_encoder(bindings),
            Some(_) => Err(ConfigError::UnknownKind),
        }
    }

    fn apply_legacy(&self, bindings: &mut Bindings) -> Result<(), ConfigError> {
        for (pin, text) in self.legacy_slots.iter() {
            if let Some(pin) = pin {
                bindings.claim_pin(
                    pin,
                    PinRole::Button {
                        debounce_ms: DEFAULT_DEBOUNCE_MS,
                    },
                )?;
            }
            bindings.add_macro(pin.map(Trigger::Press), text, pin.unwrap_or(0))?;
        }
        Ok(())
    }

    fn apply_button(&self, bindings: &mut Bindings) -> Result<(), ConfigError> {
        self.check_names(&["switch"], &["press"])?;
        let pin = self.pin("switch");
        if let Some(pin) = pin {
            bindings.claim_pin(
                pin,
                PinRole::Button {
                    debounce_ms: self.debounce_ms,
                },
            )?;
        }
        for (_, text) in self.functions.iter() {
            bindings.add_macro(pin.map(Trigger::Press), text, pin.unwrap_or(0))?;
        }
        Ok(())
    }

    fn apply_encoder(&self, bindings: &mut Bindings) -> Result<(), ConfigError> {
        self.check_names(&["a", "b", "switch"], &["turn_right", "turn_left", "press"])?;
        let encoder = match (self.pin("a"), self.pin("b")) {
            (Some(a), Some(b)) => Some(bindings.add_encoder(EncoderSetup {
                a,
                b,
                steps_per_detent: self.steps_per_detent,
                reverse: self.reverse,
            })?),
            _ => None,
        };
        let switch = self.pin("switch");
        if let Some(pin) = switch {
            bindings.claim_pin(
                pin,
                PinRole::Button {
                    debounce_ms: self.debounce_ms,
                },
            )?;
        }
        let error_pin = self.pin("a").or(switch).unwrap_or(0);
        for (name, text) in self.functions.iter() {
            let trigger = if name.eq_str("turn_right") {
                encoder.map(|index| Trigger::Turn(index, Direction::Right))
            } else if name.eq_str("turn_left") {
                encoder.map(|index| Trigger::Turn(index, Direction::Left))
            } else {
                switch.map(Trigger::Press)
            };
            bindings.add_macro(trigger, text, error_pin)?;
        }
        Ok(())
    }
}

// Parsing

pub fn parse_config(json: &[u8], bindings: &mut Bindings) -> Result<(), ConfigError> {
    bindings.clear();
    if json.len() > MAX_CONFIG_SIZE {
        return Err(ConfigError::TooLarge);
    }
    let result = parse_root(json, bindings);
    if result.is_err() {
        bindings.clear();
    }
    result
}

fn remember(slot: &mut Option<ConfigError>, error: ConfigError, parser: &Parser<'_>) -> JsonError {
    slot.get_or_insert(error);
    parser.error()
}

fn parse_root(json: &[u8], bindings: &mut Bindings) -> Result<(), ConfigError> {
    let mut parser = Parser::new(json);
    if parser.peek() != Some(b'{') {
        return Err(ConfigError::InvalidStructure("root"));
    }
    let mut error = None;
    parser
        .object(|parser, key| {
            let result = if key.eq_str("format") {
                parse_format(parser)
            } else if key.eq_str("tiles") {
                parse_tiles(parser, bindings)
            } else if key.eq_str("pins") {
                parse_legacy_pins(parser, bindings)
            } else {
                parser.skip_value().map_err(ConfigError::from)
            };
            result.map_err(|config_error| remember(&mut error, config_error, parser))
        })
        .map_err(|json_error| error.unwrap_or(ConfigError::from(json_error)))?;
    parser.finish()?;
    Ok(())
}

fn parse_format(parser: &mut Parser<'_>) -> Result<(), ConfigError> {
    match parse_small_number(parser)? {
        Some(1..=SUPPORTED_FORMAT) => Ok(()),
        _ => Err(ConfigError::UnsupportedFormat),
    }
}

fn parse_tiles(parser: &mut Parser<'_>, bindings: &mut Bindings) -> Result<(), ConfigError> {
    if parser.peek() != Some(b'[') {
        return Err(ConfigError::InvalidStructure("tiles"));
    }
    let mut error = None;
    parser
        .array(|parser| {
            parse_tile(parser, bindings)
                .map_err(|config_error| remember(&mut error, config_error, parser))
        })
        .map_err(|json_error| error.unwrap_or(ConfigError::from(json_error)))
}

fn parse_tile(parser: &mut Parser<'_>, bindings: &mut Bindings) -> Result<(), ConfigError> {
    if parser.peek() != Some(b'{') {
        return Err(ConfigError::InvalidStructure("tile"));
    }
    let mut tile = Tile::new();
    let mut error = None;
    parser
        .object(|parser, key| {
            let result = if key.eq_str("kind") {
                parser
                    .string()
                    .map(|kind| tile.kind = Some(kind))
                    .map_err(ConfigError::from)
            } else if key.eq_str("pins") {
                parse_tile_pins(parser, &mut tile)
            } else if key.eq_str("functions") {
                parse_tile_functions(parser, &mut tile)
            } else if key.eq_str("parameters") {
                parse_tile_parameters(parser, &mut tile)
            } else {
                parser.skip_value().map_err(ConfigError::from)
            };
            result.map_err(|config_error| remember(&mut error, config_error, parser))
        })
        .map_err(|json_error| error.unwrap_or(ConfigError::from(json_error)))?;
    tile.apply(bindings)
}

fn parse_tile_pins<'a>(parser: &mut Parser<'a>, tile: &mut Tile<'a>) -> Result<(), ConfigError> {
    if parser.peek() != Some(b'{') {
        return Err(ConfigError::InvalidStructure("pins"));
    }
    let mut error = None;
    parser
        .object(|parser, role| {
            parse_pin(parser)
                .and_then(|pin| tile.pins.push((role, pin)))
                .map_err(|config_error| remember(&mut error, config_error, parser))
        })
        .map_err(|json_error| error.unwrap_or(ConfigError::from(json_error)))
}

fn parse_tile_functions<'a>(
    parser: &mut Parser<'a>,
    tile: &mut Tile<'a>,
) -> Result<(), ConfigError> {
    if parser.peek() != Some(b'{') {
        return Err(ConfigError::InvalidStructure("functions"));
    }
    let mut error = None;
    parser
        .object(|parser, name| {
            let result = match parser.peek() {
                Some(b'"') => parser
                    .string()
                    .map_err(ConfigError::from)
                    .and_then(|text| tile.functions.push((name, text))),
                Some(b'{') => {
                    parse_legacy_slot(parser).and_then(|slot| tile.legacy_slots.push(slot))
                }
                _ => Err(ConfigError::InvalidStructure("function")),
            };
            result.map_err(|config_error| remember(&mut error, config_error, parser))
        })
        .map_err(|json_error| error.unwrap_or(ConfigError::from(json_error)))
}

fn parse_legacy_slot<'a>(
    parser: &mut Parser<'a>,
) -> Result<(Option<u8>, JsonStr<'a>), ConfigError> {
    let mut pin = None;
    let mut macro_text = None;
    let mut error = None;
    parser
        .object(|parser, key| {
            let result = if key.eq_str("pin") {
                parse_pin(parser).map(|value| pin = value)
            } else if key.eq_str("macro_str") {
                parser
                    .string()
                    .map(|text| macro_text = Some(text))
                    .map_err(ConfigError::from)
            } else {
                parser.skip_value().map_err(ConfigError::from)
            };
            result.map_err(|config_error| remember(&mut error, config_error, parser))
        })
        .map_err(|json_error| error.unwrap_or(ConfigError::from(json_error)))?;
    let macro_text = macro_text.ok_or(ConfigError::InvalidStructure("function"))?;
    Ok((pin, macro_text))
}

fn parse_tile_parameters(parser: &mut Parser<'_>, tile: &mut Tile<'_>) -> Result<(), ConfigError> {
    if parser.peek() != Some(b'{') {
        return Err(ConfigError::InvalidStructure("parameters"));
    }
    let mut error = None;
    parser
        .object(|parser, name| {
            let result = if name.eq_str("debounce_ms") {
                match parse_small_number(parser) {
                    Ok(Some(value)) if value <= MAX_DEBOUNCE_MS => {
                        tile.debounce_ms = value;
                        Ok(())
                    }
                    _ => Err(ConfigError::InvalidParameter("debounce_ms")),
                }
            } else if name.eq_str("steps_per_detent") {
                match parse_small_number(parser) {
                    Ok(Some(value @ (1 | 2 | 4))) => {
                        tile.steps_per_detent = value;
                        Ok(())
                    }
                    _ => Err(ConfigError::InvalidParameter("steps_per_detent")),
                }
            } else if name.eq_str("reverse") {
                parse_bool(parser)
                    .map(|value| tile.reverse = value)
                    .ok_or(ConfigError::InvalidParameter("reverse"))
            } else {
                parser.skip_value().map_err(ConfigError::from)
            };
            result.map_err(|config_error| remember(&mut error, config_error, parser))
        })
        .map_err(|json_error| error.unwrap_or(ConfigError::from(json_error)))
}

fn parse_bool(parser: &mut Parser<'_>) -> Option<bool> {
    let value = match parser.peek()? {
        b't' => true,
        b'f' => false,
        _ => return None,
    };
    parser.skip_value().ok().map(|_| value)
}

fn parse_small_number(parser: &mut Parser<'_>) -> Result<Option<u8>, ConfigError> {
    if !matches!(parser.peek(), Some(b'-' | b'0'..=b'9')) {
        parser.skip_value()?;
        return Ok(None);
    }
    let digits = parser.number()?;
    Ok(parse_digits(digits.as_bytes()))
}

fn parse_pin(parser: &mut Parser<'_>) -> Result<Option<u8>, ConfigError> {
    match parser.peek() {
        Some(b'"') => {
            let text = parser.string()?;
            if text.is_empty() || text.eq_str("-") {
                return Ok(None);
            }
            parse_digits(text.raw())
                .map(Some)
                .ok_or(ConfigError::InvalidPin)
        }
        Some(b'-' | b'0'..=b'9') => parse_digits(parser.number()?.as_bytes())
            .map(Some)
            .ok_or(ConfigError::InvalidPin),
        Some(b'n') => parser.skip_value().map(|_| None).map_err(ConfigError::from),
        _ => Err(ConfigError::InvalidPin),
    }
}

fn parse_digits(digits: &[u8]) -> Option<u8> {
    if digits.is_empty() || digits.len() > 3 || !digits.iter().all(u8::is_ascii_digit) {
        return None;
    }
    let value = digits
        .iter()
        .fold(0u16, |value, digit| value * 10 + u16::from(digit - b'0'));
    u8::try_from(value).ok()
}

fn parse_legacy_pins(parser: &mut Parser<'_>, bindings: &mut Bindings) -> Result<(), ConfigError> {
    if parser.peek() != Some(b'{') {
        return Err(ConfigError::InvalidStructure("pins"));
    }
    let mut error = None;
    parser
        .object(|parser, key| {
            let result = parse_digits(key.raw())
                .ok_or(ConfigError::InvalidPin)
                .and_then(|pin| {
                    let text = parser.string()?;
                    bindings.claim_pin(
                        pin,
                        PinRole::Button {
                            debounce_ms: DEFAULT_DEBOUNCE_MS,
                        },
                    )?;
                    bindings.add_macro(Some(Trigger::Press(pin)), text, pin)
                });
            result.map_err(|config_error| remember(&mut error, config_error, parser))
        })
        .map_err(|json_error| error.unwrap_or(ConfigError::from(json_error)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(json: &str) -> Result<Bindings, ConfigError> {
        let mut bindings = Bindings::new();
        parse_config(json.as_bytes(), &mut bindings).map(|_| bindings)
    }

    // Format 2

    const PROFILE: &str = r#"{
        "format": 2,
        "profile_name": "Desk",
        "tiles": [
            {
                "id": "tile_1",
                "display_name": "Copy",
                "component": "button",
                "kind": "button",
                "pins": { "switch": 4 },
                "parameters": { "debounce_ms": 25 },
                "functions": { "press": "P:lctrl,P:c,D:50,R:c,R:lctrl" },
                "x": 50.0,
                "y": 120.5
            },
            {
                "id": "tile_2",
                "display_name": "Volume",
                "component": "ec11",
                "kind": "encoder",
                "pins": { "a": 15, "b": 16, "switch": 17 },
                "parameters": { "steps_per_detent": 4, "reverse": true, "debounce_ms": 15, "future": [1, 2] },
                "functions": { "turn_right": "MEDIA:vol_up", "turn_left": "MEDIA:vol_down", "press": "MEDIA:mute" },
                "x": 200,
                "y": -3e1
            }
        ]
    }"#;

    #[test]
    fn reads_buttons_and_encoders() {
        let bindings = parse(PROFILE).unwrap();
        let layout = bindings.layout();

        assert_eq!(layout.role(4), PinRole::Button { debounce_ms: 25 });
        assert_eq!(layout.role(15), PinRole::EncoderA(0));
        assert_eq!(layout.role(16), PinRole::EncoderB(0));
        assert_eq!(layout.role(17), PinRole::Button { debounce_ms: 15 });
        assert_eq!(layout.role(5), PinRole::Unused);
        assert_eq!(
            layout.encoders(),
            &[EncoderSetup {
                a: 15,
                b: 16,
                steps_per_detent: 4,
                reverse: true
            }]
        );
    }

    #[test]
    fn maps_functions_to_triggers() {
        let bindings = parse(PROFILE).unwrap();

        assert_eq!(bindings.len(), 4);
        assert_eq!(
            bindings.macro_for(Trigger::Press(4)),
            Some("P:lctrl,P:c,D:50,R:c,R:lctrl")
        );
        assert_eq!(
            bindings.macro_for(Trigger::Turn(0, Direction::Right)),
            Some("MEDIA:vol_up")
        );
        assert_eq!(
            bindings.macro_for(Trigger::Turn(0, Direction::Left)),
            Some("MEDIA:vol_down")
        );
        assert_eq!(bindings.macro_for(Trigger::Press(17)), Some("MEDIA:mute"));
        assert_eq!(bindings.macro_for(Trigger::Press(15)), None);
    }

    #[test]
    fn uses_defaults_for_missing_parameters() {
        let bindings = parse(
            r#"{"format":2,"tiles":[
                {"kind":"button","pins":{"switch":4},"functions":{"press":"P:a"}},
                {"kind":"encoder","pins":{"a":15,"b":16},"functions":{}}
            ]}"#,
        )
        .unwrap();

        assert_eq!(
            bindings.layout().role(4),
            PinRole::Button {
                debounce_ms: DEFAULT_DEBOUNCE_MS
            }
        );
        assert_eq!(
            bindings.layout().encoders()[0].steps_per_detent,
            DEFAULT_STEPS_PER_DETENT
        );
        assert!(!bindings.layout().encoders()[0].reverse);
    }

    #[test]
    fn unassigned_pins_are_allowed() {
        let bindings = parse(
            r#"{"format":2,"tiles":[
                {"kind":"button","pins":{},"functions":{"press":"P:a"}},
                {"kind":"encoder","pins":{"a":15},"functions":{"turn_right":"P:b"}}
            ]}"#,
        )
        .unwrap();

        assert!(bindings.is_empty());
        assert!(bindings.layout().encoders().is_empty());
        assert_eq!(bindings.layout().role(15), PinRole::Unused);
    }

    #[test]
    fn empty_macros_are_not_stored() {
        let bindings =
            parse(r#"{"format":2,"tiles":[{"kind":"button","pins":{"switch":4},"functions":{"press":""}}]}"#)
                .unwrap();

        assert!(bindings.is_empty());
        assert_eq!(
            bindings.layout().role(4),
            PinRole::Button { debounce_ms: 10 }
        );
    }

    #[test]
    fn rejects_invalid_format_2_tiles() {
        let cases = [
            (r#"{"format":3}"#, ConfigError::UnsupportedFormat),
            (r#"{"format":"2"}"#, ConfigError::UnsupportedFormat),
            (
                r#"{"tiles":[{"kind":"joystick"}]}"#,
                ConfigError::UnknownKind,
            ),
            (
                r#"{"tiles":[{"kind":"button","pins":{"a":4}}]}"#,
                ConfigError::UnknownRole,
            ),
            (
                r#"{"tiles":[{"kind":"button","functions":{"turn_right":"P:a"}}]}"#,
                ConfigError::UnknownFunction,
            ),
            (
                r#"{"tiles":[{"kind":"button","pins":{"switch":4},"parameters":{"debounce_ms":201}}]}"#,
                ConfigError::InvalidParameter("debounce_ms"),
            ),
            (
                r#"{"tiles":[{"kind":"encoder","parameters":{"steps_per_detent":3}}]}"#,
                ConfigError::InvalidParameter("steps_per_detent"),
            ),
            (
                r#"{"tiles":[{"kind":"encoder","parameters":{"reverse":"yes"}}]}"#,
                ConfigError::InvalidParameter("reverse"),
            ),
            (
                r#"{"tiles":[{"kind":"encoder","pins":{"a":15,"b":15}}]}"#,
                ConfigError::DuplicatePin(15),
            ),
            (
                r#"{"tiles":[{"kind":"button","pins":{"switch":15}},{"kind":"encoder","pins":{"a":15,"b":16}}]}"#,
                ConfigError::DuplicatePin(15),
            ),
            (
                r#"{"tiles":[{"kind":"encoder","pins":{"a":35,"b":16}}]}"#,
                ConfigError::UnusablePin(35),
            ),
            (
                r#"{"tiles":[{"kind":"encoder","pins":{"a":15,"b":16},"functions":{"turn_left":"P:hyper"}}]}"#,
                ConfigError::InvalidMacro(15),
            ),
            (
                r#"{"tiles":[{"kind":"button","pins":{},"functions":{"press":"P:hyper"}}]}"#,
                ConfigError::InvalidMacro(0),
            ),
        ];
        for (json, expected) in cases {
            assert_eq!(parse(json).err(), Some(expected), "{json}");
        }
    }

    #[test]
    fn supports_one_encoder_per_two_pins() {
        let tiles: Vec<String> = INPUT_PINS
            .chunks(2)
            .filter(|pair| pair.len() == 2)
            .map(|pair| format!(r#"{{"kind":"encoder","pins":{{"a":{},"b":{}}},"functions":{{"turn_right":"P:a","turn_left":"P:b"}}}}"#, pair[0], pair[1]))
            .collect();
        let json = format!(r#"{{"format":2,"tiles":[{}]}}"#, tiles.join(","));

        let bindings = parse(&json).unwrap();

        assert_eq!(bindings.layout().encoders().len(), MAX_ENCODERS);
        assert_eq!(bindings.len(), MAX_ENCODERS * 2);
    }

    // Format 1

    const LEGACY_PROFILE: &str = r#"{
        "profile_name": "Default_Profile",
        "tiles": [
            {
                "id": "tile_1",
                "tile_type": "Button",
                "functions": { "button": { "pin": "4", "macro_str": "P:lctrl,P:c,D:50,R:c,R:lctrl" } },
                "x": 50.0,
                "y": 120.5
            },
            {
                "id": "tile_2",
                "tile_type": "EC11",
                "functions": {
                    "push": { "pin": "17", "macro_str": "MEDIA:mute" },
                    "rot_a": { "pin": "15", "macro_str": "MEDIA:vol_up" },
                    "rot_b": { "pin": "", "macro_str": "" }
                }
            }
        ]
    }"#;

    #[test]
    fn reads_format_1_slots_as_buttons() {
        let bindings = parse(LEGACY_PROFILE).unwrap();

        assert_eq!(bindings.len(), 3);
        assert_eq!(
            bindings.macro_for_pin(4),
            Some("P:lctrl,P:c,D:50,R:c,R:lctrl")
        );
        assert_eq!(bindings.macro_for_pin(17), Some("MEDIA:mute"));
        assert_eq!(bindings.macro_for_pin(15), Some("MEDIA:vol_up"));
        assert_eq!(
            bindings.layout().role(15),
            PinRole::Button {
                debounce_ms: DEFAULT_DEBOUNCE_MS
            }
        );
    }

    #[test]
    fn reads_legacy_arduino_format() {
        let bindings =
            parse(r#"{"pins":{"15":"MEDIA:mute","5":"P:ctrl,P:c,D:20,R:c,R:ctrl"}}"#).unwrap();

        assert_eq!(bindings.macro_for_pin(15), Some("MEDIA:mute"));
        assert_eq!(
            bindings.macro_for_pin(5),
            Some("P:ctrl,P:c,D:20,R:c,R:ctrl")
        );
    }

    #[test]
    fn accepts_empty_configs() {
        assert!(parse("{}").unwrap().is_empty());
        assert!(
            parse(r#"{"profile_name":"x","tiles":[]}"#)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn accepts_numeric_pins_and_escaped_macros() {
        let bindings =
            parse(r#"{"tiles":[{"functions":{"b":{"pin":6,"macro_str":"P:KeyA,R:KeyA"}}}]}"#)
                .unwrap();

        assert_eq!(bindings.macro_for_pin(6), Some("P:KeyA,R:KeyA"));
    }

    #[test]
    fn keeps_text_actions_for_later() {
        let bindings = parse(
            r#"{"format":2,"tiles":[{"kind":"button","pins":{"switch":7},"functions":{"press":"TEXT:Hallo Welt"}}]}"#,
        );

        assert_eq!(bindings.unwrap().macro_for_pin(7), Some("TEXT:Hallo Welt"));
    }

    #[test]
    fn rejects_unescaped_commas_in_text() {
        let bindings =
            parse(r#"{"tiles":[{"functions":{"b":{"pin":"7","macro_str":"TEXT:Hallo, Welt"}}}]}"#);

        assert_eq!(bindings.err(), Some(ConfigError::InvalidMacro(7)));
    }

    // Errors

    #[test]
    fn rejects_invalid_configs() {
        let cases = [
            ("", ConfigError::InvalidStructure("root")),
            ("[]", ConfigError::InvalidStructure("root")),
            (r#"{"tiles":{}}"#, ConfigError::InvalidStructure("tiles")),
            (r#"{"tiles":[1]}"#, ConfigError::InvalidStructure("tile")),
            (
                r#"{"tiles":[{"functions":[]}]}"#,
                ConfigError::InvalidStructure("functions"),
            ),
            (
                r#"{"tiles":[{"functions":{"b":{"pin":"35","macro_str":""}}}]}"#,
                ConfigError::UnusablePin(35),
            ),
            (
                r#"{"tiles":[{"functions":{"b":{"pin":"x4","macro_str":""}}}]}"#,
                ConfigError::InvalidPin,
            ),
            (
                r#"{"tiles":[{"functions":{"b":{"pin":"999","macro_str":""}}}]}"#,
                ConfigError::InvalidPin,
            ),
            (
                r#"{"tiles":[{"functions":{"a":{"pin":"4","macro_str":""},"b":{"pin":"4","macro_str":""}}}]}"#,
                ConfigError::DuplicatePin(4),
            ),
            (
                r#"{"tiles":[{"functions":{"b":{"pin":"4","macro_str":"P:hyper"}}}]}"#,
                ConfigError::InvalidMacro(4),
            ),
            (r#"{"pins":{"99":"P:a"}}"#, ConfigError::UnusablePin(99)),
        ];
        for (json, expected) in cases {
            assert_eq!(parse(json).err(), Some(expected), "{json}");
        }
    }

    #[test]
    fn rejects_truncated_json() {
        let truncated = &PROFILE[..PROFILE.len() / 2];

        assert!(matches!(parse(truncated), Err(ConfigError::InvalidJson(_))));
    }

    #[test]
    fn rejects_oversized_configs() {
        let large = format!(r#"{{"profile_name":"{}"}}"#, "x".repeat(MAX_CONFIG_SIZE));

        assert_eq!(parse(&large).err(), Some(ConfigError::TooLarge));
    }

    #[test]
    fn failed_parse_leaves_no_bindings() {
        let mut bindings = Bindings::new();
        parse_config(PROFILE.as_bytes(), &mut bindings).unwrap();

        let result = parse_config(
            br#"{"tiles":[{"kind":"button","pins":{"switch":35}}]}"#,
            &mut bindings,
        );

        assert!(result.is_err());
        assert!(bindings.is_empty());
        assert!(bindings.layout().encoders().is_empty());
    }

    #[test]
    fn errors_have_protocol_text() {
        assert_eq!(ConfigError::InvalidJson(12).to_string(), "invalid_json:12");
        assert_eq!(ConfigError::UnusablePin(35).to_string(), "unusable_pin:35");
        assert_eq!(
            ConfigError::InvalidParameter("debounce_ms").to_string(),
            "invalid_parameter:debounce_ms"
        );
    }
}
