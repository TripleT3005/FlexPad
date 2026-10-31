// Types

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Modifier(u8),
    Usage(u8),
}

// Lookup

pub fn key_from_name(name: &str) -> Option<Key> {
    let name = name.trim();
    letter_key(name)
        .or_else(|| digit_key(name))
        .or_else(|| function_key(name))
        .or_else(|| numpad_digit_key(name))
        .or_else(|| {
            NAMED_KEYS
                .iter()
                .find(|(names, _)| {
                    names
                        .iter()
                        .any(|candidate| candidate.eq_ignore_ascii_case(name))
                })
                .map(|(_, key)| *key)
        })
}

pub fn media_usage_from_name(name: &str) -> Option<u16> {
    let name = name.trim();
    MEDIA_KEYS
        .iter()
        .find(|(names, _)| {
            names
                .iter()
                .any(|candidate| candidate.eq_ignore_ascii_case(name))
        })
        .map(|(_, usage)| *usage)
}

fn strip_prefix_ignore_case<'a>(name: &'a str, prefix: &str) -> Option<&'a str> {
    let head = name.get(..prefix.len())?;
    head.eq_ignore_ascii_case(prefix)
        .then(|| &name[prefix.len()..])
}

fn single_char(name: &str, prefix: &str) -> Option<u8> {
    let rest = strip_prefix_ignore_case(name, prefix).unwrap_or(name);
    match rest.as_bytes() {
        [byte] => Some(byte.to_ascii_lowercase()),
        _ => None,
    }
}

fn letter_key(name: &str) -> Option<Key> {
    single_char(name, "Key")
        .filter(u8::is_ascii_lowercase)
        .map(|letter| Key::Usage(0x04 + (letter - b'a')))
}

fn digit_key(name: &str) -> Option<Key> {
    single_char(name, "Digit")
        .filter(u8::is_ascii_digit)
        .map(|digit| Key::Usage(digit_usage(digit, 0x1E, 0x27)))
}

fn numpad_digit_key(name: &str) -> Option<Key> {
    let digit = ["Numpad", "kp_"].iter().find_map(|prefix| {
        match strip_prefix_ignore_case(name, prefix)?.as_bytes() {
            [digit] => Some(*digit),
            _ => None,
        }
    })?;
    digit
        .is_ascii_digit()
        .then(|| Key::Usage(digit_usage(digit, 0x59, 0x62)))
}

fn digit_usage(digit: u8, one: u8, zero: u8) -> u8 {
    if digit == b'0' {
        zero
    } else {
        one + (digit - b'1')
    }
}

fn function_key(name: &str) -> Option<Key> {
    let digits = strip_prefix_ignore_case(name, "F")?;
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let number: u8 = digits.parse().ok()?;
    match number {
        1..=12 => Some(Key::Usage(0x3A + number - 1)),
        13..=24 => Some(Key::Usage(0x68 + number - 13)),
        _ => None,
    }
}

// Tables

const NAMED_KEYS: &[(&[&str], Key)] = &[
    (&["ControlLeft", "lctrl", "ctrl"], Key::Modifier(0x01)),
    (&["ShiftLeft", "lshift", "shift"], Key::Modifier(0x02)),
    (&["AltLeft", "lalt", "alt"], Key::Modifier(0x04)),
    (&["MetaLeft", "lwin", "win", "gui"], Key::Modifier(0x08)),
    (&["ControlRight", "rctrl"], Key::Modifier(0x10)),
    (&["ShiftRight", "rshift"], Key::Modifier(0x20)),
    (&["AltRight", "ralt", "altgr"], Key::Modifier(0x40)),
    (&["MetaRight", "rwin"], Key::Modifier(0x80)),
    (&["Enter", "return"], Key::Usage(0x28)),
    (&["Escape", "esc"], Key::Usage(0x29)),
    (&["Backspace"], Key::Usage(0x2A)),
    (&["Tab"], Key::Usage(0x2B)),
    (&["Space"], Key::Usage(0x2C)),
    (&["Minus", "sz"], Key::Usage(0x2D)),
    (&["Equal"], Key::Usage(0x2E)),
    (
        &["BracketLeft", "left_brace", "bracket_left", "uel"],
        Key::Usage(0x2F),
    ),
    (
        &["BracketRight", "right_brace", "bracket_right"],
        Key::Usage(0x30),
    ),
    (&["Backslash"], Key::Usage(0x31)),
    (&["IntlHash"], Key::Usage(0x32)),
    (&["Semicolon", "oel"], Key::Usage(0x33)),
    (&["Quote", "ael"], Key::Usage(0x34)),
    (&["Backquote"], Key::Usage(0x35)),
    (&["Comma"], Key::Usage(0x36)),
    (&["Period"], Key::Usage(0x37)),
    (&["Slash"], Key::Usage(0x38)),
    (&["CapsLock"], Key::Usage(0x39)),
    (&["PrintScreen"], Key::Usage(0x46)),
    (&["ScrollLock"], Key::Usage(0x47)),
    (&["Pause"], Key::Usage(0x48)),
    (&["Insert"], Key::Usage(0x49)),
    (&["Home"], Key::Usage(0x4A)),
    (&["PageUp"], Key::Usage(0x4B)),
    (&["Delete", "del"], Key::Usage(0x4C)),
    (&["End"], Key::Usage(0x4D)),
    (&["PageDown"], Key::Usage(0x4E)),
    (&["ArrowRight", "right"], Key::Usage(0x4F)),
    (&["ArrowLeft", "left"], Key::Usage(0x50)),
    (&["ArrowDown", "down"], Key::Usage(0x51)),
    (&["ArrowUp", "up"], Key::Usage(0x52)),
    (&["NumLock"], Key::Usage(0x53)),
    (&["NumpadDivide", "kp_slash"], Key::Usage(0x54)),
    (&["NumpadMultiply", "kp_asterisk"], Key::Usage(0x55)),
    (&["NumpadSubtract", "kp_minus"], Key::Usage(0x56)),
    (&["NumpadAdd", "kp_plus"], Key::Usage(0x57)),
    (&["NumpadEnter", "kp_enter"], Key::Usage(0x58)),
    (&["NumpadDecimal"], Key::Usage(0x63)),
    (&["IntlBackslash"], Key::Usage(0x64)),
    (&["ContextMenu"], Key::Usage(0x65)),
];

const MEDIA_KEYS: &[(&[&str], u16)] = &[
    (&["AudioVolumeUp", "vol_up"], 0x00E9),
    (&["AudioVolumeDown", "vol_down"], 0x00EA),
    (&["AudioVolumeMute", "mute"], 0x00E2),
    (&["MediaTrackNext", "next"], 0x00B5),
    (&["MediaTrackPrevious", "prev"], 0x00B6),
    (&["MediaPlayPause", "play"], 0x00CD),
    (&["MediaStop", "stop"], 0x00B7),
];

#[cfg(test)]
mod tests {
    use super::*;

    // Keys

    #[test]
    fn letters_use_us_positions() {
        assert_eq!(key_from_name("KeyA"), Some(Key::Usage(0x04)));
        assert_eq!(key_from_name("a"), Some(Key::Usage(0x04)));
        assert_eq!(key_from_name("KeyZ"), Some(Key::Usage(0x1D)));
        assert_eq!(key_from_name("Z"), Some(Key::Usage(0x1D)));
    }

    #[test]
    fn digits_and_numpad_digits() {
        assert_eq!(key_from_name("Digit1"), Some(Key::Usage(0x1E)));
        assert_eq!(key_from_name("0"), Some(Key::Usage(0x27)));
        assert_eq!(key_from_name("Numpad1"), Some(Key::Usage(0x59)));
        assert_eq!(key_from_name("kp_0"), Some(Key::Usage(0x62)));
    }

    #[test]
    fn function_keys_up_to_f24() {
        assert_eq!(key_from_name("F1"), Some(Key::Usage(0x3A)));
        assert_eq!(key_from_name("f12"), Some(Key::Usage(0x45)));
        assert_eq!(key_from_name("F13"), Some(Key::Usage(0x68)));
        assert_eq!(key_from_name("F24"), Some(Key::Usage(0x73)));
        assert_eq!(key_from_name("F25"), None);
        assert_eq!(key_from_name("F0"), None);
    }

    #[test]
    fn modifiers_are_bits() {
        assert_eq!(key_from_name("ControlLeft"), Some(Key::Modifier(0x01)));
        assert_eq!(key_from_name("lctrl"), Some(Key::Modifier(0x01)));
        assert_eq!(key_from_name("altgr"), Some(Key::Modifier(0x40)));
        assert_eq!(key_from_name("MetaLeft"), Some(Key::Modifier(0x08)));
    }

    #[test]
    fn legacy_app_names_map_to_positions() {
        assert_eq!(key_from_name("ael"), key_from_name("Quote"));
        assert_eq!(key_from_name("oel"), key_from_name("Semicolon"));
        assert_eq!(key_from_name("uel"), key_from_name("BracketLeft"));
        assert_eq!(key_from_name("sz"), key_from_name("Minus"));
        assert_eq!(key_from_name("left_brace"), key_from_name("bracket_left"));
        assert_eq!(key_from_name("kp_slash"), key_from_name("NumpadDivide"));
    }

    #[test]
    fn every_app_key_name_is_known() {
        let app_names = [
            "lctrl",
            "rctrl",
            "lshift",
            "rshift",
            "lalt",
            "ralt",
            "lwin",
            "rwin",
            "enter",
            "esc",
            "backspace",
            "tab",
            "space",
            "up",
            "down",
            "left",
            "right",
            "ael",
            "oel",
            "uel",
            "sz",
            "minus",
            "equal",
            "left_brace",
            "right_brace",
            "semicolon",
            "quote",
            "backslash",
            "comma",
            "period",
            "slash",
            "kp_enter",
            "kp_plus",
            "kp_minus",
            "kp_asterisk",
            "kp_slash",
        ];
        for name in app_names {
            assert!(key_from_name(name).is_some(), "{name}");
        }
    }

    #[test]
    fn unknown_names_are_rejected() {
        assert_eq!(key_from_name(""), None);
        assert_eq!(key_from_name("KeyAB"), None);
        assert_eq!(key_from_name("ö"), None);
        assert_eq!(key_from_name("Numpaö"), None);
        assert_eq!(key_from_name("äF1"), None);
        assert_eq!(key_from_name("F+1"), None);
        assert_eq!(key_from_name("Hyper"), None);
    }

    // Media

    #[test]
    fn media_names_map_to_consumer_usages() {
        assert_eq!(media_usage_from_name("vol_up"), Some(0x00E9));
        assert_eq!(media_usage_from_name("MediaPlayPause"), Some(0x00CD));
        assert_eq!(media_usage_from_name("shuffle"), None);
    }
}
