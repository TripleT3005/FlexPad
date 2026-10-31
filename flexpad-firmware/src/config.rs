use core::fmt::{self, Display, Write};

use crate::json::{JsonError, JsonStr, Parser};
use crate::macros::{self, MacroError};

// Constants

pub const INPUT_PINS: [u8; 25] = [
    1, 2, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 21, 38, 39, 40, 41, 42, 47, 48,
];
pub const MAX_CONFIG_SIZE: usize = 16 * 1024;
pub const MACRO_STORAGE_SIZE: usize = 8 * 1024;

// Errors

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigError {
    TooLarge,
    InvalidJson(usize),
    InvalidStructure(&'static str),
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

// Bindings

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Binding {
    pin: u8,
    start: usize,
    end: usize,
}

pub struct Bindings {
    items: [Binding; INPUT_PINS.len()],
    count: usize,
    macros: [u8; MACRO_STORAGE_SIZE],
    used: usize,
}

impl Bindings {
    pub const fn new() -> Self {
        Self {
            items: [Binding {
                pin: 0,
                start: 0,
                end: 0,
            }; INPUT_PINS.len()],
            count: 0,
            macros: [0; MACRO_STORAGE_SIZE],
            used: 0,
        }
    }

    pub fn clear(&mut self) {
        self.count = 0;
        self.used = 0;
    }

    pub fn len(&self) -> usize {
        self.count
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    pub fn macro_for_pin(&self, pin: u8) -> Option<&str> {
        self.items[..self.count]
            .iter()
            .find(|binding| binding.pin == pin)
            .map(|binding| self.text(binding))
    }

    pub fn pins(&self) -> impl Iterator<Item = u8> + '_ {
        self.items[..self.count].iter().map(|binding| binding.pin)
    }

    fn text(&self, binding: &Binding) -> &str {
        core::str::from_utf8(&self.macros[binding.start..binding.end]).unwrap_or_default()
    }

    fn add(&mut self, pin: u8, macro_text: JsonStr<'_>) -> Result<(), ConfigError> {
        if !INPUT_PINS.contains(&pin) {
            return Err(ConfigError::UnusablePin(pin));
        }
        if self.macro_for_pin(pin).is_some() {
            return Err(ConfigError::DuplicatePin(pin));
        }
        let start = self.used;
        let mut writer = SliceWriter {
            buffer: &mut self.macros[start..],
            length: 0,
        };
        macro_text
            .unescape_into(&mut writer)
            .map_err(|_| ConfigError::MacrosTooLarge)?;
        let end = start + writer.length;
        let binding = Binding { pin, start, end };
        let text = self.text(&binding);
        let valid = macros::actions(text)
            .all(|action| matches!(action, Ok(_) | Err(MacroError::Unsupported(_))));
        if !valid {
            return Err(ConfigError::InvalidMacro(pin));
        }
        self.items[self.count] = binding;
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

fn parse_root(json: &[u8], bindings: &mut Bindings) -> Result<(), ConfigError> {
    let mut parser = Parser::new(json);
    if parser.peek() != Some(b'{') {
        return Err(ConfigError::InvalidStructure("root"));
    }
    let mut error = None;
    parser
        .object(|parser, key| {
            let result = if key.eq_str("tiles") {
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

fn remember(slot: &mut Option<ConfigError>, error: ConfigError, parser: &Parser<'_>) -> JsonError {
    slot.get_or_insert(error);
    parser.error()
}

fn parse_tiles(parser: &mut Parser<'_>, bindings: &mut Bindings) -> Result<(), ConfigError> {
    if parser.peek() != Some(b'[') {
        return Err(ConfigError::InvalidStructure("tiles"));
    }
    let mut error = None;
    parser
        .array(|parser| {
            if parser.peek() != Some(b'{') {
                return Err(remember(
                    &mut error,
                    ConfigError::InvalidStructure("tile"),
                    parser,
                ));
            }
            parser.object(|parser, key| {
                let result = if key.eq_str("functions") {
                    parse_functions(parser, bindings)
                } else {
                    parser.skip_value().map_err(ConfigError::from)
                };
                result.map_err(|config_error| remember(&mut error, config_error, parser))
            })
        })
        .map_err(|json_error| error.unwrap_or(ConfigError::from(json_error)))
}

fn parse_functions(parser: &mut Parser<'_>, bindings: &mut Bindings) -> Result<(), ConfigError> {
    if parser.peek() != Some(b'{') {
        return Err(ConfigError::InvalidStructure("functions"));
    }
    let mut error = None;
    parser
        .object(|parser, _slot| {
            parse_function(parser, bindings)
                .map_err(|config_error| remember(&mut error, config_error, parser))
        })
        .map_err(|json_error| error.unwrap_or(ConfigError::from(json_error)))
}

fn parse_function(parser: &mut Parser<'_>, bindings: &mut Bindings) -> Result<(), ConfigError> {
    if parser.peek() != Some(b'{') {
        return Err(ConfigError::InvalidStructure("function"));
    }
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
    match (pin, macro_text) {
        (Some(pin), Some(text)) => bindings.add(pin, text),
        _ => Ok(()),
    }
}

fn parse_pin(parser: &mut Parser<'_>) -> Result<Option<u8>, ConfigError> {
    match parser.peek() {
        Some(b'"') => {
            let text = parser.string()?;
            if text.is_empty() || text.eq_str("-") {
                return Ok(None);
            }
            parse_pin_number(text.raw()).map(Some)
        }
        Some(b'-' | b'0'..=b'9') => parse_pin_number(parser.number()?.as_bytes()).map(Some),
        Some(b'n') => parser.skip_value().map(|_| None).map_err(ConfigError::from),
        _ => Err(ConfigError::InvalidPin),
    }
}

fn parse_pin_number(digits: &[u8]) -> Result<u8, ConfigError> {
    if digits.is_empty() || digits.len() > 3 || !digits.iter().all(u8::is_ascii_digit) {
        return Err(ConfigError::InvalidPin);
    }
    let value = digits
        .iter()
        .fold(0u16, |value, digit| value * 10 + u16::from(digit - b'0'));
    u8::try_from(value).map_err(|_| ConfigError::InvalidPin)
}

fn parse_legacy_pins(parser: &mut Parser<'_>, bindings: &mut Bindings) -> Result<(), ConfigError> {
    if parser.peek() != Some(b'{') {
        return Err(ConfigError::InvalidStructure("pins"));
    }
    let mut error = None;
    parser
        .object(|parser, key| {
            let result = parse_pin_number(key.raw()).and_then(|pin| {
                let text = parser.string()?;
                bindings.add(pin, text)
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

    // App format

    const APP_CONFIG: &str = r#"{
        "profile_name": "Default_Profile",
        "tiles": [
            {
                "id": "tile_1",
                "display_name": "Copy",
                "tile_type": "Button",
                "functions": { "button": { "pin": "4", "macro_str": "P:lctrl,P:c,D:50,R:c,R:lctrl" } },
                "x": 50.0,
                "y": 120.5
            },
            {
                "id": "tile_2",
                "display_name": "Volume",
                "tile_type": "EC11",
                "functions": {
                    "push": { "pin": "17", "macro_str": "MEDIA:mute" },
                    "rot_a": { "pin": "15", "macro_str": "MEDIA:vol_up" },
                    "rot_b": { "pin": "", "macro_str": "" }
                },
                "x": 200,
                "y": -3e1
            }
        ]
    }"#;

    #[test]
    fn reads_bindings_from_app_config() {
        let bindings = parse(APP_CONFIG).unwrap();

        assert_eq!(bindings.len(), 3);
        assert_eq!(
            bindings.macro_for_pin(4),
            Some("P:lctrl,P:c,D:50,R:c,R:lctrl")
        );
        assert_eq!(bindings.macro_for_pin(17), Some("MEDIA:mute"));
        assert_eq!(bindings.macro_for_pin(15), Some("MEDIA:vol_up"));
        assert_eq!(bindings.macro_for_pin(5), None);
        assert_eq!(bindings.pins().collect::<Vec<_>>(), vec![4, 17, 15]);
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
        let bindings =
            parse(r#"{"tiles":[{"functions":{"b":{"pin":"7","macro_str":"TEXT:Hallo Welt"}}}]}"#);

        assert_eq!(bindings.unwrap().macro_for_pin(7), Some("TEXT:Hallo Welt"));
    }

    #[test]
    fn rejects_unescaped_commas_in_text() {
        let bindings =
            parse(r#"{"tiles":[{"functions":{"b":{"pin":"7","macro_str":"TEXT:Hallo, Welt"}}}]}"#);

        assert_eq!(bindings.err(), Some(ConfigError::InvalidMacro(7)));
    }

    // Legacy format

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
        let truncated = &APP_CONFIG[..APP_CONFIG.len() / 2];

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
        parse_config(APP_CONFIG.as_bytes(), &mut bindings).unwrap();

        let result = parse_config(
            br#"{"tiles":[{"functions":{"b":{"pin":"35","macro_str":""}}}]}"#,
            &mut bindings,
        );

        assert!(result.is_err());
        assert!(bindings.is_empty());
    }

    #[test]
    fn errors_have_protocol_text() {
        assert_eq!(ConfigError::InvalidJson(12).to_string(), "invalid_json:12");
        assert_eq!(ConfigError::UnusablePin(35).to_string(), "unusable_pin:35");
    }
}
