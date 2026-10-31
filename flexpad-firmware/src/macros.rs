use core::fmt::{self, Display, Write};

use crate::json::{self, JsonError, JsonStr, Parser};
use crate::keys::{Key, key_from_name, media_usage_from_name};

// Constants

pub const MAX_DELAY_MS: u32 = 60_000;
pub const MAX_MACRO_SIZE: usize = 512;
const TAG_PRESS_MODIFIER: u8 = 1;
const TAG_PRESS_USAGE: u8 = 2;
const TAG_RELEASE_MODIFIER: u8 = 3;
const TAG_RELEASE_USAGE: u8 = 4;
const TAG_DELAY: u8 = 5;
const TAG_MEDIA: u8 = 6;
const TAG_TEXT: u8 = 7;

// Types

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action<'a> {
    Press(Key),
    Release(Key),
    Delay(u32),
    Media(u16),
    Text(&'a str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MacroError<'a> {
    InvalidJson(usize),
    InvalidStructure(&'static str),
    UnknownKey(&'a str),
    UnknownMedia(&'a str),
    InvalidDelay(&'a str),
    InvalidText,
    Unsupported(&'a str),
    UnknownAction(&'a str),
    TooLarge,
}

impl Display for MacroError<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MacroError::InvalidJson(position) => write!(f, "invalid_json:{position}"),
            MacroError::InvalidStructure(what) => write!(f, "invalid_structure:{what}"),
            MacroError::UnknownKey(name) => write!(f, "unknown_key:{name}"),
            MacroError::UnknownMedia(name) => write!(f, "unknown_media:{name}"),
            MacroError::InvalidDelay(value) => write!(f, "invalid_delay:{value}"),
            MacroError::InvalidText => f.write_str("invalid_text"),
            MacroError::Unsupported(kind) => write!(f, "unsupported:{kind}"),
            MacroError::UnknownAction(kind) => write!(f, "unknown_action:{kind}"),
            MacroError::TooLarge => f.write_str("too_large"),
        }
    }
}

impl From<JsonError> for MacroError<'_> {
    fn from(error: JsonError) -> Self {
        MacroError::InvalidJson(error.position)
    }
}

// Writing

pub struct MacroWriter<'a> {
    buffer: &'a mut [u8],
    length: usize,
    overflowed: bool,
}

impl<'a> MacroWriter<'a> {
    pub fn new(buffer: &'a mut [u8]) -> Self {
        Self {
            buffer,
            length: 0,
            overflowed: false,
        }
    }

    pub fn len(&self) -> usize {
        self.length
    }

    pub fn is_empty(&self) -> bool {
        self.length == 0
    }

    fn push(&mut self, bytes: &[u8]) -> Result<(), MacroError<'static>> {
        let end = self.length + bytes.len();
        let Some(target) = self.buffer.get_mut(self.length..end) else {
            self.overflowed = true;
            return Err(MacroError::TooLarge);
        };
        target.copy_from_slice(bytes);
        self.length = end;
        Ok(())
    }

    fn key(&mut self, press: bool, key: Key) -> Result<(), MacroError<'static>> {
        let (tag, code) = match (press, key) {
            (true, Key::Modifier(bits)) => (TAG_PRESS_MODIFIER, bits),
            (true, Key::Usage(usage)) => (TAG_PRESS_USAGE, usage),
            (false, Key::Modifier(bits)) => (TAG_RELEASE_MODIFIER, bits),
            (false, Key::Usage(usage)) => (TAG_RELEASE_USAGE, usage),
        };
        self.push(&[tag, code])
    }

    fn value(&mut self, tag: u8, value: u16) -> Result<(), MacroError<'static>> {
        let [low, high] = value.to_le_bytes();
        self.push(&[tag, low, high])
    }

    fn text(
        &mut self,
        write: impl FnOnce(&mut Self) -> fmt::Result,
    ) -> Result<(), MacroError<'static>> {
        let start = self.length;
        self.push(&[TAG_TEXT, 0, 0])?;
        if write(self).is_err() {
            return Err(if self.overflowed {
                MacroError::TooLarge
            } else {
                MacroError::InvalidText
            });
        }
        let length = u16::try_from(self.length - start - 3).map_err(|_| MacroError::TooLarge)?;
        self.buffer[start + 1..start + 3].copy_from_slice(&length.to_le_bytes());
        Ok(())
    }
}

impl Write for MacroWriter<'_> {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        self.push(text.as_bytes()).map_err(|_| fmt::Error)
    }
}

struct LegacyText<'w, 'a> {
    out: &'w mut MacroWriter<'a>,
    backslash: bool,
}

impl Write for LegacyText<'_, '_> {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        for character in text.chars() {
            match (self.backslash, character) {
                (true, 'n') => {
                    self.backslash = false;
                    self.out.write_char('\n')?;
                }
                (true, '\\') => self.out.write_char('\\')?,
                (true, other) => {
                    self.backslash = false;
                    self.out.write_char('\\')?;
                    self.out.write_char(other)?;
                }
                (false, '\\') => self.backslash = true,
                (false, other) => self.out.write_char(other)?,
            }
        }
        Ok(())
    }
}

// Compiling

pub fn compile_function<'a>(
    json: &'a [u8],
    out: &mut MacroWriter<'_>,
) -> Result<(), MacroError<'a>> {
    let mut parser = Parser::new(json);
    if parser.peek() != Some(b'{') {
        return Err(MacroError::InvalidStructure("function"));
    }
    let mut error = None;
    parser
        .object(|parser, key| {
            let result = if key.eq_str("actions") {
                compile_actions(parser, out)
            } else {
                parser.skip_value().map_err(MacroError::from)
            };
            result.map_err(|macro_error| json::remember(&mut error, macro_error, parser))
        })
        .map_err(|json_error| error.unwrap_or(MacroError::from(json_error)))?;
    parser.finish()?;
    Ok(())
}

fn compile_actions<'a>(
    parser: &mut Parser<'a>,
    out: &mut MacroWriter<'_>,
) -> Result<(), MacroError<'a>> {
    if parser.peek() != Some(b'[') {
        return Err(MacroError::InvalidStructure("actions"));
    }
    let mut error = None;
    parser
        .array(|parser| {
            compile_action(parser, out)
                .map_err(|macro_error| json::remember(&mut error, macro_error, parser))
        })
        .map_err(|json_error| error.unwrap_or(MacroError::from(json_error)))
}

fn compile_action<'a>(
    parser: &mut Parser<'a>,
    out: &mut MacroWriter<'_>,
) -> Result<(), MacroError<'a>> {
    if parser.peek() != Some(b'{') {
        return Err(MacroError::InvalidStructure("action"));
    }
    let mut members = 0;
    let mut error = None;
    parser
        .object(|parser, kind| {
            members += 1;
            let result = if members > 1 {
                Err(MacroError::InvalidStructure("action"))
            } else {
                compile_member(parser, kind, out)
            };
            result.map_err(|macro_error| json::remember(&mut error, macro_error, parser))
        })
        .map_err(|json_error| error.unwrap_or(MacroError::from(json_error)))?;
    if members == 0 {
        return Err(MacroError::InvalidStructure("action"));
    }
    Ok(())
}

fn compile_member<'a>(
    parser: &mut Parser<'a>,
    kind: JsonStr<'a>,
    out: &mut MacroWriter<'_>,
) -> Result<(), MacroError<'a>> {
    let string = |parser: &mut Parser<'a>| match parser.peek() {
        Some(b'"') => parser.string().map_err(MacroError::from),
        _ => Err(MacroError::InvalidStructure("action")),
    };
    if kind.eq_str("press") || kind.eq_str("release") {
        let name = string(parser)?.as_str();
        let key = key_from_name(name).ok_or(MacroError::UnknownKey(name))?;
        Ok(out.key(kind.eq_str("press"), key)?)
    } else if kind.eq_str("delay") {
        let raw = core::str::from_utf8(parser.raw_value()?).unwrap_or_default();
        let ms = delay(raw).ok_or(MacroError::InvalidDelay(raw))?;
        Ok(out.value(TAG_DELAY, ms)?)
    } else if kind.eq_str("media") {
        let name = string(parser)?.as_str();
        let usage = media_usage_from_name(name).ok_or(MacroError::UnknownMedia(name))?;
        Ok(out.value(TAG_MEDIA, usage)?)
    } else if kind.eq_str("text") {
        let text = string(parser)?;
        Ok(out.text(|out| text.unescape_into(out))?)
    } else {
        Err(MacroError::UnknownAction(kind.as_str()))
    }
}

pub fn compile_legacy<'a>(
    macro_text: JsonStr<'a>,
    out: &mut MacroWriter<'_>,
) -> Result<(), MacroError<'a>> {
    for token in macro_text.tokens(b',') {
        let (kind, value) = token
            .split_once(b':')
            .ok_or(MacroError::UnknownAction(token.as_str()))?;
        let name = value.as_str();
        match kind.as_str() {
            "P" | "R" => {
                let key = key_from_name(name).ok_or(MacroError::UnknownKey(name))?;
                out.key(kind.eq_str("P"), key)?;
            }
            "D" => {
                let ms = delay(name.trim()).ok_or(MacroError::InvalidDelay(name))?;
                out.value(TAG_DELAY, ms)?;
            }
            "MEDIA" => {
                let usage = media_usage_from_name(name).ok_or(MacroError::UnknownMedia(name))?;
                out.value(TAG_MEDIA, usage)?;
            }
            "TEXT" => out.text(|out| {
                let mut text = LegacyText {
                    out,
                    backslash: false,
                };
                value.unescape_into(&mut text)?;
                if text.backslash {
                    text.out.write_char('\\')?;
                }
                Ok(())
            })?,
            _ => return Err(MacroError::UnknownAction(token.as_str())),
        }
    }
    Ok(())
}

fn delay(text: &str) -> Option<u16> {
    let ms: u32 = text.parse().ok()?;
    if ms > MAX_DELAY_MS {
        return None;
    }
    u16::try_from(ms).ok()
}

// Reading

pub struct Actions<'a> {
    code: &'a [u8],
}

impl<'a> Iterator for Actions<'a> {
    type Item = Action<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let (&tag, rest) = self.code.split_first()?;
        let (action, length) = match (tag, rest) {
            (TAG_PRESS_MODIFIER, [bits, ..]) => (Action::Press(Key::Modifier(*bits)), 1),
            (TAG_PRESS_USAGE, [usage, ..]) => (Action::Press(Key::Usage(*usage)), 1),
            (TAG_RELEASE_MODIFIER, [bits, ..]) => (Action::Release(Key::Modifier(*bits)), 1),
            (TAG_RELEASE_USAGE, [usage, ..]) => (Action::Release(Key::Usage(*usage)), 1),
            (TAG_DELAY, [low, high, ..]) => (
                Action::Delay(u32::from(u16::from_le_bytes([*low, *high]))),
                2,
            ),
            (TAG_MEDIA, [low, high, ..]) => (Action::Media(u16::from_le_bytes([*low, *high])), 2),
            (TAG_TEXT, [low, high, text @ ..]) => {
                let length = usize::from(u16::from_le_bytes([*low, *high]));
                let text = core::str::from_utf8(text.get(..length)?).ok()?;
                (Action::Text(text), 2 + length)
            }
            _ => {
                self.code = &[];
                return None;
            }
        };
        self.code = &rest[length..];
        Some(action)
    }
}

pub fn actions(code: &[u8]) -> Actions<'_> {
    Actions { code }
}

// Buffer

pub struct MacroBuffer {
    code: [u8; MAX_MACRO_SIZE],
    length: usize,
}

impl MacroBuffer {
    pub const fn new() -> Self {
        Self {
            code: [0; MAX_MACRO_SIZE],
            length: 0,
        }
    }

    pub fn from_code(code: &[u8]) -> Option<Self> {
        let mut buffer = Self::new();
        buffer.code.get_mut(..code.len())?.copy_from_slice(code);
        buffer.length = code.len();
        Some(buffer)
    }

    pub fn compile(json: &[u8]) -> Result<Self, MacroError<'_>> {
        let mut buffer = Self::new();
        let mut writer = MacroWriter::new(&mut buffer.code);
        compile_function(json, &mut writer)?;
        buffer.length = writer.len();
        Ok(buffer)
    }

    pub fn code(&self) -> &[u8] {
        &self.code[..self.length]
    }

    pub fn actions(&self) -> Actions<'_> {
        actions(self.code())
    }
}

impl Default for MacroBuffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn compiled(json: &str) -> Result<Vec<Action<'static>>, MacroError<'static>> {
        let json: &'static str = Box::leak(json.to_string().into_boxed_str());
        let code: &'static mut [u8] = Box::leak(vec![0; MAX_MACRO_SIZE].into_boxed_slice());
        let mut writer = MacroWriter::new(code);
        compile_function(json.as_bytes(), &mut writer)?;
        let length = writer.len();
        Ok(actions(&code[..length]).collect())
    }

    fn compiled_legacy(text: &str) -> Result<Vec<Action<'static>>, MacroError<'static>> {
        let json: &'static str = Box::leak(format!("\"{text}\"").into_boxed_str());
        let raw = Parser::new(json.as_bytes()).string().unwrap();
        let code: &'static mut [u8] = Box::leak(vec![0; MAX_MACRO_SIZE].into_boxed_slice());
        let mut writer = MacroWriter::new(code);
        compile_legacy(raw, &mut writer)?;
        let length = writer.len();
        Ok(actions(&code[..length]).collect())
    }

    // Function objects

    #[test]
    fn compiles_all_action_types() {
        let parsed = compiled(
            r#"{"actions":[{"press":"ControlLeft"},{"press":"KeyC"},{"delay":50},{"release":"KeyC"},{"release":"ControlLeft"},{"media":"vol_up"},{"text":"Hallo, Welt\n\"ä\" \\"}]}"#,
        );

        assert_eq!(
            parsed,
            Ok(vec![
                Action::Press(Key::Modifier(0x01)),
                Action::Press(Key::Usage(0x06)),
                Action::Delay(50),
                Action::Release(Key::Usage(0x06)),
                Action::Release(Key::Modifier(0x01)),
                Action::Media(0x00E9),
                Action::Text("Hallo, Welt\n\"ä\" \\"),
            ])
        );
    }

    #[test]
    fn ignores_unknown_function_members_and_accepts_empty_lists() {
        assert_eq!(compiled(r#"{"actions":[]}"#), Ok(vec![]));
        assert_eq!(compiled(r#"{}"#), Ok(vec![]));
        assert_eq!(
            compiled(r#"{"value":3,"actions":[{"delay":0}]}"#),
            Ok(vec![Action::Delay(0)])
        );
    }

    #[test]
    fn rejects_invalid_actions() {
        let cases = [
            (r#"[]"#, MacroError::InvalidStructure("function")),
            (r#"{"actions":{}}"#, MacroError::InvalidStructure("actions")),
            (
                r#"{"actions":["P:a"]}"#,
                MacroError::InvalidStructure("action"),
            ),
            (
                r#"{"actions":[{}]}"#,
                MacroError::InvalidStructure("action"),
            ),
            (
                r#"{"actions":[{"press":"KeyA","release":"KeyA"}]}"#,
                MacroError::InvalidStructure("action"),
            ),
            (
                r#"{"actions":[{"press":4}]}"#,
                MacroError::InvalidStructure("action"),
            ),
            (
                r#"{"actions":[{"press":"hyper"}]}"#,
                MacroError::UnknownKey("hyper"),
            ),
            (
                r#"{"actions":[{"delay":60001}]}"#,
                MacroError::InvalidDelay("60001"),
            ),
            (
                r#"{"actions":[{"delay":-5}]}"#,
                MacroError::InvalidDelay("-5"),
            ),
            (
                r#"{"actions":[{"delay":"50"}]}"#,
                MacroError::InvalidDelay("\"50\""),
            ),
            (
                r#"{"actions":[{"media":"shuffle"}]}"#,
                MacroError::UnknownMedia("shuffle"),
            ),
            (
                r#"{"actions":[{"text":"\ud83d"}]}"#,
                MacroError::InvalidText,
            ),
            (
                r#"{"actions":[{"jump":1}]}"#,
                MacroError::UnknownAction("jump"),
            ),
            (r#"{"actions":[{"delay":5}"#, MacroError::InvalidJson(23)),
        ];
        for (json, expected) in cases {
            assert_eq!(compiled(json), Err(expected), "{json}");
        }
    }

    #[test]
    fn reports_full_buffers() {
        let mut code = [0; 8];
        let mut writer = MacroWriter::new(&mut code);

        let result = compile_function(br#"{"actions":[{"text":"too long"}]}"#, &mut writer);

        assert_eq!(result, Err(MacroError::TooLarge));
    }

    #[test]
    fn buffers_compile_and_copy_code() {
        let buffer = MacroBuffer::compile(br#"{"actions":[{"media":"mute"}]}"#).unwrap();
        let copy = MacroBuffer::from_code(buffer.code()).unwrap();

        assert_eq!(copy.actions().collect::<Vec<_>>(), [Action::Media(0x00E2)]);
        assert!(MacroBuffer::from_code(&[0; MAX_MACRO_SIZE + 1]).is_none());
    }

    // Legacy strings

    #[test]
    fn compiles_legacy_strings() {
        assert_eq!(
            compiled_legacy(" P:lctrl , ,P:c,D:50,R:c,R:lctrl,MEDIA:mute,"),
            Ok(vec![
                Action::Press(Key::Modifier(0x01)),
                Action::Press(Key::Usage(0x06)),
                Action::Delay(50),
                Action::Release(Key::Usage(0x06)),
                Action::Release(Key::Modifier(0x01)),
                Action::Media(0x00E2),
            ])
        );
        assert_eq!(compiled_legacy(""), Ok(vec![]));
    }

    #[test]
    fn decodes_line_breaks_in_legacy_text() {
        assert_eq!(
            compiled_legacy(r"TEXT:first\\nsecond\\\\x\\"),
            Ok(vec![Action::Text("first\nsecond\\\\x\\")])
        );
    }

    #[test]
    fn reports_the_first_invalid_legacy_token() {
        let cases = [
            ("P:a,P:hyper,R:a", MacroError::UnknownKey("hyper")),
            ("D:abc", MacroError::InvalidDelay("abc")),
            ("D:60001", MacroError::InvalidDelay("60001")),
            ("MEDIA:shuffle", MacroError::UnknownMedia("shuffle")),
            ("X:1", MacroError::UnknownAction("X:1")),
            ("garbage", MacroError::UnknownAction("garbage")),
            ("TEXT:Hallo, Welt", MacroError::UnknownAction("Welt")),
        ];
        for (text, expected) in cases {
            assert_eq!(compiled_legacy(text), Err(expected), "{text}");
        }
    }

    // Errors

    #[test]
    fn errors_have_protocol_text() {
        assert_eq!(
            MacroError::UnknownKey("hyper").to_string(),
            "unknown_key:hyper"
        );
        assert_eq!(
            MacroError::InvalidStructure("action").to_string(),
            "invalid_structure:action"
        );
        assert_eq!(
            MacroError::Unsupported("text").to_string(),
            "unsupported:text"
        );
        assert_eq!(MacroError::InvalidJson(3).to_string(), "invalid_json:3");
    }
}
