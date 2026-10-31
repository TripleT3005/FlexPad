use serde::{Deserialize, Serialize};

#[cfg(test)]
#[allow(dead_code)]
#[path = "../../flexpad-firmware/src/keys.rs"]
pub(crate) mod firmware_keys;

// Layouts

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KeyboardLayout {
    #[default]
    Us,
    De,
}

impl KeyboardLayout {
    pub const ALL: &[KeyboardLayout] = &[KeyboardLayout::Us, KeyboardLayout::De];
}

impl std::fmt::Display for KeyboardLayout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            KeyboardLayout::Us => "English (US)",
            KeyboardLayout::De => "German",
        })
    }
}

// Keys

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Key {
    pub code: &'static str,
    us: &'static str,
    de: &'static str,
}

impl Key {
    pub fn label(&self, layout: KeyboardLayout) -> &'static str {
        match layout {
            KeyboardLayout::Us => self.us,
            KeyboardLayout::De => self.de,
        }
    }
}

const fn key(code: &'static str, label: &'static str) -> Key {
    Key {
        code,
        us: label,
        de: label,
    }
}

const fn layout_key(code: &'static str, us: &'static str, de: &'static str) -> Key {
    Key { code, us, de }
}

pub const KEYS: &[Key] = &[
    key("ControlLeft", "Ctrl (left)"),
    key("ControlRight", "Ctrl (right)"),
    key("ShiftLeft", "Shift (left)"),
    key("ShiftRight", "Shift (right)"),
    key("AltLeft", "Alt (left)"),
    layout_key("AltRight", "Alt (right)", "AltGr"),
    key("MetaLeft", "Win / Cmd (left)"),
    key("MetaRight", "Win / Cmd (right)"),
    key("Enter", "Enter"),
    key("Escape", "Escape"),
    key("Backspace", "Backspace"),
    key("Tab", "Tab"),
    key("Space", "Space"),
    key("CapsLock", "Caps Lock"),
    key("ArrowUp", "Arrow up"),
    key("ArrowDown", "Arrow down"),
    key("ArrowLeft", "Arrow left"),
    key("ArrowRight", "Arrow right"),
    key("Insert", "Insert"),
    key("Delete", "Delete"),
    key("Home", "Home"),
    key("End", "End"),
    key("PageUp", "Page up"),
    key("PageDown", "Page down"),
    key("PrintScreen", "Print Screen"),
    key("ScrollLock", "Scroll Lock"),
    key("Pause", "Pause"),
    key("ContextMenu", "Menu"),
    key("Digit1", "1"),
    key("Digit2", "2"),
    key("Digit3", "3"),
    key("Digit4", "4"),
    key("Digit5", "5"),
    key("Digit6", "6"),
    key("Digit7", "7"),
    key("Digit8", "8"),
    key("Digit9", "9"),
    key("Digit0", "0"),
    key("KeyA", "A"),
    key("KeyB", "B"),
    key("KeyC", "C"),
    key("KeyD", "D"),
    key("KeyE", "E"),
    key("KeyF", "F"),
    key("KeyG", "G"),
    key("KeyH", "H"),
    key("KeyI", "I"),
    key("KeyJ", "J"),
    key("KeyK", "K"),
    key("KeyL", "L"),
    key("KeyM", "M"),
    key("KeyN", "N"),
    key("KeyO", "O"),
    key("KeyP", "P"),
    key("KeyQ", "Q"),
    key("KeyR", "R"),
    key("KeyS", "S"),
    key("KeyT", "T"),
    key("KeyU", "U"),
    key("KeyV", "V"),
    key("KeyW", "W"),
    key("KeyX", "X"),
    layout_key("KeyY", "Y", "Z"),
    layout_key("KeyZ", "Z", "Y"),
    layout_key("Minus", "-", "ß"),
    layout_key("Equal", "=", "´"),
    layout_key("BracketLeft", "[", "Ü"),
    layout_key("BracketRight", "]", "+"),
    layout_key("Backslash", "\\", "#"),
    layout_key("Semicolon", ";", "Ö"),
    layout_key("Quote", "'", "Ä"),
    layout_key("Backquote", "`", "^"),
    key("Comma", ","),
    key("Period", "."),
    layout_key("Slash", "/", "-"),
    layout_key("IntlBackslash", "Extra ISO key (\\ |)", "<"),
    key("NumLock", "Num Lock"),
    key("Numpad1", "Numpad 1"),
    key("Numpad2", "Numpad 2"),
    key("Numpad3", "Numpad 3"),
    key("Numpad4", "Numpad 4"),
    key("Numpad5", "Numpad 5"),
    key("Numpad6", "Numpad 6"),
    key("Numpad7", "Numpad 7"),
    key("Numpad8", "Numpad 8"),
    key("Numpad9", "Numpad 9"),
    key("Numpad0", "Numpad 0"),
    key("NumpadDivide", "Numpad /"),
    key("NumpadMultiply", "Numpad *"),
    key("NumpadSubtract", "Numpad -"),
    key("NumpadAdd", "Numpad +"),
    key("NumpadEnter", "Numpad Enter"),
    layout_key("NumpadDecimal", "Numpad .", "Numpad ,"),
    key("F1", "F1"),
    key("F2", "F2"),
    key("F3", "F3"),
    key("F4", "F4"),
    key("F5", "F5"),
    key("F6", "F6"),
    key("F7", "F7"),
    key("F8", "F8"),
    key("F9", "F9"),
    key("F10", "F10"),
    key("F11", "F11"),
    key("F12", "F12"),
    key("F13", "F13"),
    key("F14", "F14"),
    key("F15", "F15"),
    key("F16", "F16"),
    key("F17", "F17"),
    key("F18", "F18"),
    key("F19", "F19"),
    key("F20", "F20"),
    key("F21", "F21"),
    key("F22", "F22"),
    key("F23", "F23"),
    key("F24", "F24"),
];

const LEGACY_NAMES: &[(&str, &str)] = &[
    ("lctrl", "ControlLeft"),
    ("ctrl", "ControlLeft"),
    ("rctrl", "ControlRight"),
    ("lshift", "ShiftLeft"),
    ("shift", "ShiftLeft"),
    ("rshift", "ShiftRight"),
    ("lalt", "AltLeft"),
    ("alt", "AltLeft"),
    ("ralt", "AltRight"),
    ("altgr", "AltRight"),
    ("lwin", "MetaLeft"),
    ("win", "MetaLeft"),
    ("gui", "MetaLeft"),
    ("rwin", "MetaRight"),
    ("return", "Enter"),
    ("esc", "Escape"),
    ("del", "Delete"),
    ("up", "ArrowUp"),
    ("down", "ArrowDown"),
    ("left", "ArrowLeft"),
    ("right", "ArrowRight"),
    ("ael", "Quote"),
    ("oel", "Semicolon"),
    ("uel", "BracketLeft"),
    ("sz", "Minus"),
    ("left_brace", "BracketLeft"),
    ("bracket_left", "BracketLeft"),
    ("right_brace", "BracketRight"),
    ("bracket_right", "BracketRight"),
    ("kp_enter", "NumpadEnter"),
    ("kp_plus", "NumpadAdd"),
    ("kp_minus", "NumpadSubtract"),
    ("kp_asterisk", "NumpadMultiply"),
    ("kp_slash", "NumpadDivide"),
];

pub fn find(code: &str) -> Option<&'static Key> {
    KEYS.iter().find(|key| key.code.eq_ignore_ascii_case(code))
}

pub fn code_for(name: &str) -> Option<&'static str> {
    let name = name.trim();
    if let Some(key) = find(name) {
        return Some(key.code);
    }
    if let Some((_, code)) = LEGACY_NAMES
        .iter()
        .find(|(legacy, _)| legacy.eq_ignore_ascii_case(name))
    {
        return Some(code);
    }
    let wanted = match name.as_bytes() {
        [letter] if letter.is_ascii_alphabetic() => {
            format!("Key{}", letter.to_ascii_uppercase() as char)
        }
        [digit] if digit.is_ascii_digit() => format!("Digit{}", *digit as char),
        _ => match name.get(..3) {
            Some(prefix) if prefix.eq_ignore_ascii_case("kp_") => format!("Numpad{}", &name[3..]),
            _ => return None,
        },
    };
    find(&wanted).map(|key| key.code)
}

pub fn label_for(name: &str, layout: KeyboardLayout) -> String {
    code_for(name)
        .and_then(find)
        .map_or_else(|| name.to_string(), |key| key.label(layout).to_string())
}

// Macros

pub fn migrate_macro(macro_str: &str) -> String {
    if !macro_str.contains("P:") && !macro_str.contains("R:") {
        return macro_str.to_string();
    }
    macro_str
        .split(',')
        .map(|token| {
            let trimmed = token.trim();
            let Some((prefix, name)) = trimmed
                .strip_prefix("P:")
                .map(|name| ("P:", name))
                .or_else(|| trimmed.strip_prefix("R:").map(|name| ("R:", name)))
            else {
                return token.to_string();
            };
            match code_for(name) {
                Some(code) if code != name => format!("{prefix}{code}"),
                _ => token.to_string(),
            }
        })
        .collect::<Vec<_>>()
        .join(",")
}

#[cfg(test)]
mod tests {
    use super::*;

    // Table

    #[test]
    fn codes_are_unique() {
        for (index, key) in KEYS.iter().enumerate() {
            assert!(
                KEYS[index + 1..].iter().all(|other| other.code != key.code),
                "{}",
                key.code
            );
        }
    }

    #[test]
    fn labels_are_unique_per_layout() {
        for &layout in KeyboardLayout::ALL {
            for (index, key) in KEYS.iter().enumerate() {
                assert!(
                    KEYS[index + 1..]
                        .iter()
                        .all(|other| other.label(layout) != key.label(layout)),
                    "{layout}: {}",
                    key.label(layout)
                );
            }
        }
    }

    #[test]
    fn every_code_is_known_to_the_firmware() {
        for key in KEYS {
            assert!(
                firmware_keys::key_from_name(key.code).is_some(),
                "{}",
                key.code
            );
        }
    }

    #[test]
    fn german_labels_follow_the_printed_keys() {
        assert_eq!(label_for("KeyY", KeyboardLayout::De), "Z");
        assert_eq!(label_for("KeyZ", KeyboardLayout::De), "Y");
        assert_eq!(label_for("Semicolon", KeyboardLayout::De), "Ö");
        assert_eq!(label_for("IntlBackslash", KeyboardLayout::De), "<");
        assert_eq!(label_for("KeyY", KeyboardLayout::Us), "Y");
        assert_eq!(label_for("Semicolon", KeyboardLayout::Us), ";");
    }

    // Legacy names

    #[test]
    fn legacy_names_map_to_positions() {
        let cases = [
            ("lctrl", "ControlLeft"),
            ("ctrl", "ControlLeft"),
            ("lwin", "MetaLeft"),
            ("esc", "Escape"),
            ("c", "KeyC"),
            ("Z", "KeyZ"),
            ("1", "Digit1"),
            ("f12", "F12"),
            ("kp_7", "Numpad7"),
            ("kp_enter", "NumpadEnter"),
            ("ael", "Quote"),
            ("oel", "Semicolon"),
            ("uel", "BracketLeft"),
            ("sz", "Minus"),
            ("left_brace", "BracketLeft"),
            ("period", "Period"),
            ("ControlLeft", "ControlLeft"),
            ("keya", "KeyA"),
        ];
        for (name, code) in cases {
            assert_eq!(code_for(name), Some(code), "{name}");
        }
        assert_eq!(code_for("hyper"), None);
        assert_eq!(code_for("kp_"), None);
        assert_eq!(code_for(""), None);
        assert_eq!(code_for("ö"), None);
    }

    #[test]
    fn unknown_names_are_shown_as_they_are() {
        assert_eq!(label_for("hyper", KeyboardLayout::De), "hyper");
    }

    // Macros

    #[test]
    fn migrate_macro_renames_only_keys() {
        assert_eq!(
            migrate_macro("P:lctrl,P:c,D:50,R:c,R:lctrl,MEDIA:mute"),
            "P:ControlLeft,P:KeyC,D:50,R:KeyC,R:ControlLeft,MEDIA:mute"
        );
        assert_eq!(
            migrate_macro("P:lwin,P:period,D:50,R:period,R:lwin"),
            "P:MetaLeft,P:Period,D:50,R:Period,R:MetaLeft"
        );
    }

    #[test]
    fn migrate_macro_keeps_text_and_unknown_tokens() {
        assert_eq!(migrate_macro("TEXT:Hallo, Welt"), "TEXT:Hallo, Welt");
        assert_eq!(
            migrate_macro("P:a,TEXT:x, P:y ,P:hyper"),
            "P:KeyA,TEXT:x,P:KeyY,P:hyper"
        );
        assert_eq!(migrate_macro(""), "");
        assert_eq!(migrate_macro("P:KeyA,R:KeyA"), "P:KeyA,R:KeyA");
    }
}
