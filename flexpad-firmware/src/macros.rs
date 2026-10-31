use core::fmt::{self, Display};

use crate::keys::{Key, key_from_name, media_usage_from_name};

// Constants

pub const MAX_DELAY_MS: u32 = 60_000;

// Types

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Press(Key),
    Release(Key),
    Delay(u32),
    Media(u16),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MacroError<'a> {
    UnknownKey(&'a str),
    UnknownMedia(&'a str),
    InvalidDelay(&'a str),
    Unsupported(&'a str),
    UnknownAction(&'a str),
}

impl Display for MacroError<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MacroError::UnknownKey(name) => write!(f, "unknown_key:{name}"),
            MacroError::UnknownMedia(name) => write!(f, "unknown_media:{name}"),
            MacroError::InvalidDelay(value) => write!(f, "invalid_delay:{value}"),
            MacroError::Unsupported(token) => write!(f, "unsupported:{token}"),
            MacroError::UnknownAction(token) => write!(f, "unknown_action:{token}"),
        }
    }
}

// Parsing

pub fn parse_action(token: &str) -> Result<Action, MacroError<'_>> {
    let (kind, value) = token
        .split_once(':')
        .ok_or(MacroError::UnknownAction(token))?;
    match kind {
        "P" => key_from_name(value)
            .map(Action::Press)
            .ok_or(MacroError::UnknownKey(value)),
        "R" => key_from_name(value)
            .map(Action::Release)
            .ok_or(MacroError::UnknownKey(value)),
        "D" => value
            .trim()
            .parse()
            .ok()
            .filter(|&ms| ms <= MAX_DELAY_MS)
            .map(Action::Delay)
            .ok_or(MacroError::InvalidDelay(value)),
        "MEDIA" => media_usage_from_name(value)
            .map(Action::Media)
            .ok_or(MacroError::UnknownMedia(value)),
        "TEXT" => Err(MacroError::Unsupported(kind)),
        _ => Err(MacroError::UnknownAction(token)),
    }
}

pub fn actions(macro_text: &str) -> impl Iterator<Item = Result<Action, MacroError<'_>>> {
    macro_text
        .split(',')
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .map(parse_action)
}

pub fn validate(macro_text: &str) -> Result<usize, MacroError<'_>> {
    actions(macro_text).try_fold(0, |count, action| action.map(|_| count + 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Parsing

    #[test]
    fn parses_copy_macro() {
        let parsed: Result<Vec<_>, _> = actions("P:lctrl,P:c,D:50,R:c,R:lctrl").collect();

        assert_eq!(
            parsed,
            Ok(vec![
                Action::Press(Key::Modifier(0x01)),
                Action::Press(Key::Usage(0x06)),
                Action::Delay(50),
                Action::Release(Key::Usage(0x06)),
                Action::Release(Key::Modifier(0x01)),
            ])
        );
    }

    #[test]
    fn parses_media_and_new_key_names() {
        assert_eq!(parse_action("MEDIA:vol_up"), Ok(Action::Media(0x00E9)));
        assert_eq!(
            parse_action("P:ControlLeft"),
            Ok(Action::Press(Key::Modifier(0x01)))
        );
        assert_eq!(
            parse_action("R:KeyV"),
            Ok(Action::Release(Key::Usage(0x19)))
        );
    }

    #[test]
    fn skips_empty_tokens_and_spaces() {
        assert_eq!(validate(" P:a , ,R:a,"), Ok(2));
        assert_eq!(validate(""), Ok(0));
    }

    #[test]
    fn reports_the_first_invalid_token() {
        assert_eq!(
            validate("P:a,P:hyper,R:a"),
            Err(MacroError::UnknownKey("hyper"))
        );
        assert_eq!(validate("D:abc"), Err(MacroError::InvalidDelay("abc")));
        assert_eq!(validate("D:-5"), Err(MacroError::InvalidDelay("-5")));
        assert_eq!(validate("D:60001"), Err(MacroError::InvalidDelay("60001")));
        assert_eq!(
            validate("MEDIA:shuffle"),
            Err(MacroError::UnknownMedia("shuffle"))
        );
        assert_eq!(validate("TEXT:hello"), Err(MacroError::Unsupported("TEXT")));
        assert_eq!(validate("X:1"), Err(MacroError::UnknownAction("X:1")));
        assert_eq!(
            validate("garbage"),
            Err(MacroError::UnknownAction("garbage"))
        );
    }

    #[test]
    fn errors_have_protocol_text() {
        assert_eq!(
            MacroError::UnknownKey("hyper").to_string(),
            "unknown_key:hyper"
        );
        assert_eq!(
            MacroError::Unsupported("TEXT").to_string(),
            "unsupported:TEXT"
        );
    }
}
