use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::i18n;
use crate::texts;

#[cfg(test)]
pub(crate) use crate::layout_list::firmware_keys;

#[allow(dead_code)]
#[path = "../../flexpad-firmware/src/layouts.rs"]
mod firmware_layouts;

#[cfg(test)]
pub(crate) use firmware_layouts::keystrokes as firmware_keystrokes;

// Layouts

pub use firmware_layouts::KeyboardLayout;

const LAYOUT_TEXT_PREFIX: &str = "layout.";

impl KeyboardLayout {
    pub fn by_name() -> Vec<KeyboardLayout> {
        let mut layouts = KeyboardLayout::ALL.to_vec();
        layouts.sort_by_cached_key(|layout| layout.to_string().to_lowercase());
        layouts
    }

    pub fn text_key(self) -> String {
        format!("{LAYOUT_TEXT_PREFIX}{}", self.code())
    }

    pub fn untypeable_characters(self, text: &str) -> String {
        let mut missing = String::new();
        for character in text.chars() {
            if firmware_layouts::keystrokes(self, character).is_none()
                && !missing.contains(character)
            {
                missing.push(character);
            }
        }
        missing
    }
}

impl std::fmt::Display for KeyboardLayout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match i18n::optional_text(&self.text_key()) {
            Some(name) => f.write_str(&name),
            None => f.write_str(self.name()),
        }
    }
}

impl Serialize for KeyboardLayout {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.code())
    }
}

impl<'de> Deserialize<'de> for KeyboardLayout {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let code = String::deserialize(deserializer)?;
        KeyboardLayout::from_name(&code)
            .ok_or_else(|| D::Error::custom(format!("unknown keyboard layout {code:?}")))
    }
}

// Keys

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Label {
    Plain(&'static str),
    Text(&'static str),
    Numpad(&'static str),
    Layout,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Key {
    pub code: &'static str,
    label: Label,
}

impl Key {
    pub fn label(&self, layout: KeyboardLayout) -> String {
        let label = layout.label(self.code);
        match (self.label, label) {
            (Label::Numpad(default), label) => {
                i18n::format_text(texts::KEY_NUMPAD, &[("key", label.unwrap_or(default))])
            }
            (_, Some(label)) => label.to_string(),
            (Label::Plain(label), None) => label.to_string(),
            (Label::Text(key), None) => i18n::text(key),
            (Label::Layout, None) => self.code.to_string(),
        }
    }
}

const fn key(code: &'static str, label: &'static str) -> Key {
    Key {
        code,
        label: Label::Plain(label),
    }
}

const fn named_key(code: &'static str, text: &'static str) -> Key {
    Key {
        code,
        label: Label::Text(text),
    }
}

const fn layout_key(code: &'static str) -> Key {
    Key {
        code,
        label: Label::Layout,
    }
}

const fn numpad_key(code: &'static str, label: &'static str) -> Key {
    Key {
        code,
        label: Label::Numpad(label),
    }
}

pub const KEYS: &[Key] = &[
    named_key("ControlLeft", texts::KEY_CONTROL_LEFT),
    named_key("ControlRight", texts::KEY_CONTROL_RIGHT),
    named_key("ShiftLeft", texts::KEY_SHIFT_LEFT),
    named_key("ShiftRight", texts::KEY_SHIFT_RIGHT),
    named_key("AltLeft", texts::KEY_ALT_LEFT),
    named_key("AltRight", texts::KEY_ALT_RIGHT),
    named_key("MetaLeft", texts::KEY_META_LEFT),
    named_key("MetaRight", texts::KEY_META_RIGHT),
    named_key("Enter", texts::KEY_ENTER),
    named_key("Escape", texts::KEY_ESCAPE),
    named_key("Backspace", texts::KEY_BACKSPACE),
    named_key("Tab", texts::KEY_TAB),
    named_key("Space", texts::KEY_SPACE),
    named_key("CapsLock", texts::KEY_CAPS_LOCK),
    named_key("ArrowUp", texts::KEY_ARROW_UP),
    named_key("ArrowDown", texts::KEY_ARROW_DOWN),
    named_key("ArrowLeft", texts::KEY_ARROW_LEFT),
    named_key("ArrowRight", texts::KEY_ARROW_RIGHT),
    named_key("Insert", texts::KEY_INSERT),
    named_key("Delete", texts::KEY_DELETE),
    named_key("Home", texts::KEY_HOME),
    named_key("End", texts::KEY_END),
    named_key("PageUp", texts::KEY_PAGE_UP),
    named_key("PageDown", texts::KEY_PAGE_DOWN),
    named_key("PrintScreen", texts::KEY_PRINT_SCREEN),
    named_key("ScrollLock", texts::KEY_SCROLL_LOCK),
    named_key("Pause", texts::KEY_PAUSE),
    named_key("ContextMenu", texts::KEY_CONTEXT_MENU),
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
    key("KeyY", "Y"),
    key("KeyZ", "Z"),
    layout_key("Minus"),
    layout_key("Equal"),
    layout_key("BracketLeft"),
    layout_key("BracketRight"),
    layout_key("Backslash"),
    layout_key("Semicolon"),
    layout_key("Quote"),
    layout_key("Backquote"),
    layout_key("Comma"),
    layout_key("Period"),
    layout_key("Slash"),
    named_key("IntlBackslash", texts::KEY_INTL_BACKSLASH),
    named_key("NumLock", texts::KEY_NUM_LOCK),
    numpad_key("Numpad1", "1"),
    numpad_key("Numpad2", "2"),
    numpad_key("Numpad3", "3"),
    numpad_key("Numpad4", "4"),
    numpad_key("Numpad5", "5"),
    numpad_key("Numpad6", "6"),
    numpad_key("Numpad7", "7"),
    numpad_key("Numpad8", "8"),
    numpad_key("Numpad9", "9"),
    numpad_key("Numpad0", "0"),
    numpad_key("NumpadDivide", "/"),
    numpad_key("NumpadMultiply", "*"),
    numpad_key("NumpadSubtract", "-"),
    numpad_key("NumpadAdd", "+"),
    numpad_key("NumpadEnter", "Enter"),
    numpad_key("NumpadDecimal", "."),
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

pub fn picker_keys(layout: KeyboardLayout) -> Vec<&'static Key> {
    let is_letter = |key: &Key| {
        let label = key.label(layout);
        let mut chars = label.chars();
        matches!((chars.next(), chars.next()), (Some(c), None) if c.is_alphabetic())
    };
    let letters_start = KEYS.iter().position(&is_letter).unwrap_or(KEYS.len());
    let mut keys: Vec<(usize, &'static Key)> = KEYS.iter().enumerate().collect();
    keys.sort_by_key(|&(index, key)| {
        if is_letter(key) {
            (letters_start, Some(key.label(layout)))
        } else {
            (index, None)
        }
    });
    keys.into_iter().map(|(_, key)| key).collect()
}

pub fn label_for(name: &str, layout: KeyboardLayout) -> String {
    code_for(name)
        .and_then(find)
        .map_or_else(|| name.to_string(), |key| key.label(layout).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout(code: &str) -> KeyboardLayout {
        KeyboardLayout::from_name(code).unwrap()
    }

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
    fn every_layout_labels_its_keys() {
        for &layout in KeyboardLayout::ALL {
            for key in KEYS.iter().filter(|key| key.label == Label::Layout) {
                assert!(
                    layout.label(key.code).is_some(),
                    "{layout:?} has no label for {}",
                    key.code
                );
            }
        }
    }

    #[test]
    fn layout_labels_belong_to_known_keys() {
        for path in crate::layout_list::layout_files() {
            let file = crate::layout_list::read(&path);
            for code in file.labels.keys() {
                assert!(
                    find(code).is_some_and(|key| key.code == code),
                    "{}: unknown key {code}",
                    file.code
                );
            }
        }
    }

    #[test]
    fn layouts_serialize_as_their_code() {
        let json = serde_json::to_string(&layout("de-CH")).unwrap();
        assert_eq!(json, r#""de-CH""#);
        assert_eq!(
            serde_json::from_str::<KeyboardLayout>(r#""DE""#).unwrap(),
            layout("de")
        );
        assert!(serde_json::from_str::<KeyboardLayout>(r#""xx""#).is_err());
    }

    #[test]
    fn layout_names_come_from_the_files() {
        assert_eq!(layout("us").name(), "English (US)");
        assert_eq!(layout("en-GB").name(), "English (UK)");
        let names: Vec<String> = KeyboardLayout::by_name()
            .iter()
            .map(ToString::to_string)
            .collect();
        let mut sorted = names.clone();
        sorted.sort_by_key(|name| name.to_lowercase());
        assert_eq!(names, sorted);
        assert_eq!(names.len(), KeyboardLayout::ALL.len());
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
        assert_eq!(label_for("KeyY", layout("de")), "Z");
        assert_eq!(label_for("KeyZ", layout("de")), "Y");
        assert_eq!(label_for("Semicolon", layout("de")), "Ö");
        assert_eq!(label_for("IntlBackslash", layout("de")), "<");
        assert_eq!(label_for("KeyY", layout("us")), "Y");
        assert_eq!(label_for("Semicolon", layout("us")), ";");
    }

    // Picker order

    fn picker_labels(layout: KeyboardLayout) -> Vec<String> {
        picker_keys(layout)
            .iter()
            .map(|k| k.label(layout))
            .collect()
    }

    #[test]
    fn picker_offers_every_key_once() {
        for &layout in KeyboardLayout::ALL {
            let mut codes: Vec<_> = picker_keys(layout).iter().map(|k| k.code).collect();
            codes.sort_unstable();
            let mut expected: Vec<_> = KEYS.iter().map(|k| k.code).collect();
            expected.sort_unstable();
            assert_eq!(codes, expected);
        }
    }

    #[test]
    fn picker_keeps_the_us_order() {
        let codes: Vec<_> = picker_keys(layout("us")).iter().map(|k| k.code).collect();
        let expected: Vec<_> = KEYS.iter().map(|k| k.code).collect();
        assert_eq!(codes, expected);
    }

    #[test]
    fn picker_sorts_german_letters_by_label() {
        let labels = picker_labels(layout("de"));
        let start = labels.iter().position(|l| l == "A").unwrap();
        let letters: Vec<_> = labels[start..start + 30].to_vec();
        let mut expected: Vec<_> = ('A'..='Z').map(|c| c.to_string()).collect();
        expected.extend(["Ä", "Ö", "Ü", "ß"].map(String::from));
        assert_eq!(letters, expected);
        assert_eq!(labels[start - 1], "0");
        assert_eq!(labels[start + 30], "´");
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

    // Text

    #[test]
    fn lists_characters_the_layout_cannot_type() {
        assert_eq!(layout("us").untypeable_characters("Größe ä"), "ößä");
        assert_eq!(layout("de").untypeable_characters("Größe ä"), "");
        assert_eq!(layout("de").untypeable_characters("😀 ok 😀"), "😀");
    }

    #[test]
    fn unknown_names_are_shown_as_they_are() {
        assert_eq!(label_for("hyper", layout("de")), "hyper");
    }
}
