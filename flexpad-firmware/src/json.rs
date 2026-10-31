use core::fmt::{self, Write};

// Constants

const MAX_DEPTH: u8 = 16;

// Errors

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JsonError {
    pub position: usize,
}

pub type JsonResult<T> = Result<T, JsonError>;

// Strings

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JsonStr<'a> {
    raw: &'a [u8],
}

impl<'a> JsonStr<'a> {
    pub fn raw(&self) -> &'a [u8] {
        self.raw
    }

    pub fn as_str(&self) -> &'a str {
        core::str::from_utf8(self.raw).unwrap_or_default()
    }

    pub fn is_empty(&self) -> bool {
        self.raw.is_empty()
    }

    pub fn tokens(&self, separator: u8) -> impl Iterator<Item = JsonStr<'a>> + use<'a> {
        self.raw
            .split(move |&byte| byte == separator)
            .map(|raw| JsonStr {
                raw: raw.trim_ascii(),
            })
            .filter(|token| !token.is_empty())
    }

    pub fn split_once(&self, separator: u8) -> Option<(JsonStr<'a>, JsonStr<'a>)> {
        let index = self.raw.iter().position(|&byte| byte == separator)?;
        Some((
            JsonStr {
                raw: &self.raw[..index],
            },
            JsonStr {
                raw: &self.raw[index + 1..],
            },
        ))
    }

    pub fn eq_str(&self, text: &str) -> bool {
        !self.raw.contains(&b'\\') && self.raw == text.as_bytes()
    }

    pub fn unescape_into(&self, out: &mut impl Write) -> fmt::Result {
        let mut index = 0;
        while index < self.raw.len() {
            let byte = self.raw[index];
            if byte != b'\\' {
                let end = self.raw[index..]
                    .iter()
                    .position(|&b| b == b'\\')
                    .map_or(self.raw.len(), |offset| index + offset);
                out.write_str(
                    core::str::from_utf8(&self.raw[index..end]).map_err(|_| fmt::Error)?,
                )?;
                index = end;
                continue;
            }
            let escape = *self.raw.get(index + 1).ok_or(fmt::Error)?;
            index += 2;
            let character = match escape {
                b'"' => '"',
                b'\\' => '\\',
                b'/' => '/',
                b'b' => '\u{8}',
                b'f' => '\u{c}',
                b'n' => '\n',
                b'r' => '\r',
                b't' => '\t',
                b'u' => {
                    let high = hex4(self.raw, index).ok_or(fmt::Error)?;
                    index += 4;
                    let code = if (0xD800..0xDC00).contains(&high) {
                        if self.raw.get(index..index + 2) != Some(b"\\u") {
                            return Err(fmt::Error);
                        }
                        let low = hex4(self.raw, index + 2).ok_or(fmt::Error)?;
                        if !(0xDC00..0xE000).contains(&low) {
                            return Err(fmt::Error);
                        }
                        index += 6;
                        0x10000 + ((high - 0xD800) << 10) + (low - 0xDC00)
                    } else {
                        high
                    };
                    char::from_u32(code).ok_or(fmt::Error)?
                }
                _ => return Err(fmt::Error),
            };
            out.write_char(character)?;
        }
        Ok(())
    }
}

fn hex4(bytes: &[u8], start: usize) -> Option<u32> {
    let digits = bytes.get(start..start + 4)?;
    digits.iter().try_fold(0u32, |value, &digit| {
        let nibble = (digit as char).to_digit(16)?;
        Some(value * 16 + nibble)
    })
}

// Parser

pub struct Parser<'a> {
    bytes: &'a [u8],
    position: usize,
    depth: u8,
}

impl<'a> Parser<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            position: 0,
            depth: 0,
        }
    }

    pub fn finish(mut self) -> JsonResult<()> {
        self.skip_whitespace();
        if self.position == self.bytes.len() {
            Ok(())
        } else {
            Err(self.error())
        }
    }

    pub fn object(
        &mut self,
        mut on_member: impl FnMut(&mut Self, JsonStr<'a>) -> JsonResult<()>,
    ) -> JsonResult<()> {
        self.enter(b'{')?;
        self.skip_whitespace();
        if self.consume(b'}') {
            self.depth -= 1;
            return Ok(());
        }
        loop {
            self.skip_whitespace();
            let key = self.string()?;
            self.skip_whitespace();
            self.expect(b':')?;
            on_member(self, key)?;
            self.skip_whitespace();
            if self.consume(b',') {
                continue;
            }
            self.expect(b'}')?;
            self.depth -= 1;
            return Ok(());
        }
    }

    pub fn array(
        &mut self,
        mut on_item: impl FnMut(&mut Self) -> JsonResult<()>,
    ) -> JsonResult<()> {
        self.enter(b'[')?;
        self.skip_whitespace();
        if self.consume(b']') {
            self.depth -= 1;
            return Ok(());
        }
        loop {
            on_item(self)?;
            self.skip_whitespace();
            if self.consume(b',') {
                continue;
            }
            self.expect(b']')?;
            self.depth -= 1;
            return Ok(());
        }
    }

    pub fn string(&mut self) -> JsonResult<JsonStr<'a>> {
        self.skip_whitespace();
        self.expect(b'"')?;
        let start = self.position;
        loop {
            match self.bytes.get(self.position) {
                None => return Err(self.error()),
                Some(b'"') => break,
                Some(b'\\') => match self.bytes.get(self.position + 1) {
                    Some(b'"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r' | b't') => {
                        self.position += 2
                    }
                    Some(b'u') if hex4(self.bytes, self.position + 2).is_some() => {
                        self.position += 6
                    }
                    _ => return Err(self.error()),
                },
                Some(&byte) if byte < 0x20 => return Err(self.error()),
                Some(_) => self.position += 1,
            }
        }
        let raw = &self.bytes[start..self.position];
        self.position += 1;
        if core::str::from_utf8(raw).is_err() {
            return Err(JsonError { position: start });
        }
        Ok(JsonStr { raw })
    }

    pub fn peek(&mut self) -> Option<u8> {
        self.skip_whitespace();
        self.bytes.get(self.position).copied()
    }

    pub fn number(&mut self) -> JsonResult<&'a str> {
        self.skip_whitespace();
        let start = self.position;
        self.consume(b'-');
        if !self.consume(b'0') {
            self.digits(1)?;
        }
        if self.consume(b'.') {
            self.digits(1)?;
        }
        if self.consume(b'e') || self.consume(b'E') {
            if !self.consume(b'+') {
                self.consume(b'-');
            }
            self.digits(1)?;
        }
        Ok(core::str::from_utf8(&self.bytes[start..self.position]).unwrap_or_default())
    }

    pub fn raw_value(&mut self) -> JsonResult<&'a [u8]> {
        self.skip_whitespace();
        let start = self.position;
        self.skip_value()?;
        Ok(&self.bytes[start..self.position])
    }

    pub fn skip_value(&mut self) -> JsonResult<()> {
        match self.peek() {
            Some(b'{') => self.object(|parser, _| parser.skip_value()),
            Some(b'[') => self.array(Self::skip_value),
            Some(b'"') => self.string().map(|_| ()),
            Some(b't') => self.literal(b"true"),
            Some(b'f') => self.literal(b"false"),
            Some(b'n') => self.literal(b"null"),
            Some(b'-' | b'0'..=b'9') => self.number().map(|_| ()),
            _ => Err(self.error()),
        }
    }

    pub fn error(&self) -> JsonError {
        JsonError {
            position: self.position,
        }
    }

    fn enter(&mut self, open: u8) -> JsonResult<()> {
        self.skip_whitespace();
        if self.depth == MAX_DEPTH {
            return Err(self.error());
        }
        self.expect(open)?;
        self.depth += 1;
        Ok(())
    }

    fn literal(&mut self, word: &[u8]) -> JsonResult<()> {
        if self.bytes[self.position..].starts_with(word) {
            self.position += word.len();
            Ok(())
        } else {
            Err(self.error())
        }
    }

    fn digits(&mut self, minimum: usize) -> JsonResult<()> {
        let start = self.position;
        while self
            .bytes
            .get(self.position)
            .is_some_and(u8::is_ascii_digit)
        {
            self.position += 1;
        }
        if self.position - start < minimum {
            Err(self.error())
        } else {
            Ok(())
        }
    }

    fn skip_whitespace(&mut self) {
        while matches!(
            self.bytes.get(self.position),
            Some(b' ' | b'\t' | b'\n' | b'\r')
        ) {
            self.position += 1;
        }
    }

    fn consume(&mut self, byte: u8) -> bool {
        if self.bytes.get(self.position) == Some(&byte) {
            self.position += 1;
            true
        } else {
            false
        }
    }

    fn expect(&mut self, byte: u8) -> JsonResult<()> {
        if self.consume(byte) {
            Ok(())
        } else {
            Err(self.error())
        }
    }
}

pub fn remember<E>(slot: &mut Option<E>, error: E, parser: &Parser<'_>) -> JsonError {
    slot.get_or_insert(error);
    parser.error()
}

pub fn validate(bytes: &[u8]) -> JsonResult<()> {
    let mut parser = Parser::new(bytes);
    parser.skip_value()?;
    parser.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Validation

    #[test]
    fn accepts_valid_documents() {
        for document in [
            r#"{}"#,
            r#"[]"#,
            r#"{"a":[1,-2.5,3e4,true,false,null,"x"],"b":{"c":{}}}"#,
            " { \"a\" : 0 } \r\n",
            r#""ä😀\n""#,
        ] {
            assert_eq!(validate(document.as_bytes()), Ok(()), "{document}");
        }
    }

    #[test]
    fn rejects_invalid_documents() {
        for document in [
            "",
            "{",
            r#"{"a":1"#,
            r#"{"a":1,}"#,
            r#"[1,]"#,
            r#"{"a" 1}"#,
            r#"{'a':1}"#,
            "01",
            "1.",
            "-",
            "tru",
            r#""\x""#,
            "\"line\nbreak\"",
            "{} {}",
            r#"{"profile_name":"Default","tiles":[{"id":"t"#,
        ] {
            assert!(validate(document.as_bytes()).is_err(), "{document}");
        }
    }

    #[test]
    fn rejects_too_deep_nesting() {
        let deep = "[".repeat(17) + &"]".repeat(17);
        let allowed = "[".repeat(16) + &"]".repeat(16);

        assert!(validate(deep.as_bytes()).is_err());
        assert_eq!(validate(allowed.as_bytes()), Ok(()));
    }

    #[test]
    fn reports_error_position() {
        assert_eq!(validate(br#"{"a":x}"#), Err(JsonError { position: 5 }));
    }

    // Strings

    fn unescape(document: &str) -> String {
        let mut parser = Parser::new(document.as_bytes());
        let mut out = String::new();
        parser.string().unwrap().unescape_into(&mut out).unwrap();
        out
    }

    #[test]
    fn unescapes_strings() {
        assert_eq!(unescape(r#""plain""#), "plain");
        assert_eq!(unescape(r#""a\"b\\c\/d\n""#), "a\"b\\c/d\n");
        assert_eq!(unescape(r#""Größe""#), "Größe");
        assert_eq!(unescape(r#""😀""#), "😀");
        assert_eq!(unescape("\"Größe\""), "Größe");
    }

    #[test]
    fn rejects_lone_surrogates() {
        let mut parser = Parser::new(br#""\ud83d""#);
        let mut out = String::new();
        assert!(parser.string().unwrap().unescape_into(&mut out).is_err());
    }

    #[test]
    fn splits_raw_strings_into_tokens() {
        let mut parser = Parser::new(br#"" P:a , ,TEXT:x\\ny,""#);
        let text = parser.string().unwrap();

        let tokens: Vec<&str> = text.tokens(b',').map(|token| token.as_str()).collect();
        let (kind, value) = text.tokens(b',').nth(1).unwrap().split_once(b':').unwrap();

        assert_eq!(tokens, ["P:a", r"TEXT:x\\ny"]);
        assert_eq!((kind.as_str(), value.as_str()), ("TEXT", r"x\\ny"));
    }

    #[test]
    fn returns_raw_values() {
        let mut parser = Parser::new(br#" {"a":[1, "x"]} "#);

        assert_eq!(parser.raw_value(), Ok(&br#"{"a":[1, "x"]}"#[..]));
        assert_eq!(parser.finish(), Ok(()));
    }

    #[test]
    fn compares_keys_without_escapes() {
        let mut parser = Parser::new(br#""tiles""#);
        let key = parser.string().unwrap();
        assert!(key.eq_str("tiles"));
        assert!(!key.eq_str("tile"));
    }
}
