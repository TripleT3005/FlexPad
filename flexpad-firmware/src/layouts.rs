// Types

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct KeyboardLayout(u8);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stroke {
    pub modifiers: u8,
    pub usage: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Keystrokes {
    pub dead_key: Option<Stroke>,
    pub key: Stroke,
}

struct Layout {
    code: &'static str,
    name: &'static str,
    labels: &'static [(&'static str, &'static str)],
    characters: &'static [(char, Keystrokes)],
}

const fn stroke(modifiers: u8, usage: u8) -> Stroke {
    Stroke { modifiers, usage }
}

include!(concat!(env!("OUT_DIR"), "/layouts.rs"));

// Layouts

impl KeyboardLayout {
    pub const DEFAULT: KeyboardLayout = DEFAULT;
    pub const ALL: &'static [KeyboardLayout] = ALL;

    pub fn from_name(name: &str) -> Option<Self> {
        ALL.iter()
            .copied()
            .find(|layout| layout.code().eq_ignore_ascii_case(name.trim()))
    }

    fn layout(self) -> &'static Layout {
        &LAYOUTS[usize::from(self.0)]
    }

    pub fn code(self) -> &'static str {
        self.layout().code
    }

    pub fn name(self) -> &'static str {
        self.layout().name
    }

    pub fn label(self, key: &str) -> Option<&'static str> {
        self.layout()
            .labels
            .iter()
            .find(|(known, _)| *known == key)
            .map(|(_, label)| *label)
    }

    pub fn characters(self) -> impl Iterator<Item = (char, Keystrokes)> {
        self.layout().characters.iter().copied()
    }
}

impl Default for KeyboardLayout {
    fn default() -> Self {
        DEFAULT
    }
}

impl core::fmt::Debug for KeyboardLayout {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.code())
    }
}

// Lookup

pub fn keystrokes(layout: KeyboardLayout, character: char) -> Option<Keystrokes> {
    let characters = layout.layout().characters;
    characters
        .binary_search_by_key(&character, |(known, _)| *known)
        .ok()
        .map(|index| characters[index].1)
}

pub fn can_type(layout: KeyboardLayout, text: &str) -> bool {
    text.chars()
        .all(|character| keystrokes(layout, character).is_some())
}

#[cfg(test)]
mod tests {
    use super::*;

    const NONE: u8 = 0x00;
    const SHIFT: u8 = 0x02;
    const ALT_GR: u8 = 0x40;
    const ENTER: u8 = 0x28;
    const TAB: u8 = 0x2B;
    const SPACE: u8 = 0x2C;
    const MINUS: u8 = 0x2D;
    const EQUAL: u8 = 0x2E;
    const BACKSLASH: u8 = 0x31;
    const INTL_HASH: u8 = 0x32;
    const SEMICOLON: u8 = 0x33;
    const QUOTE: u8 = 0x34;
    const BACKQUOTE: u8 = 0x35;
    const SLASH: u8 = 0x38;
    const INTL_BACKSLASH: u8 = 0x64;

    fn layout(code: &str) -> KeyboardLayout {
        KeyboardLayout::from_name(code).unwrap()
    }

    fn key(code: &str, character: char) -> Option<(u8, u8)> {
        keystrokes(layout(code), character)
            .filter(|strokes| strokes.dead_key.is_none())
            .map(|strokes| (strokes.key.modifiers, strokes.key.usage))
    }

    // Layouts

    #[test]
    fn layout_names() {
        assert_eq!(KeyboardLayout::from_name("us").unwrap().code(), "us");
        assert_eq!(KeyboardLayout::from_name("de").unwrap().name(), "German");
        assert_eq!(KeyboardLayout::from_name("DE-ch").unwrap().code(), "de-CH");
        assert_eq!(KeyboardLayout::from_name("xx"), None);
        assert_eq!(KeyboardLayout::default().code(), "us");
        assert_eq!(KeyboardLayout::DEFAULT, KeyboardLayout::default());
    }

    #[test]
    fn characters_are_sorted_for_the_lookup() {
        for layout in KeyboardLayout::ALL {
            let characters: Vec<char> = layout.characters().map(|(c, _)| c).collect();
            assert!(
                characters.windows(2).all(|pair| pair[0] < pair[1]),
                "{layout:?}"
            );
        }
    }

    #[test]
    fn no_two_characters_share_their_keystrokes() {
        for layout in KeyboardLayout::ALL {
            let strokes: Vec<Keystrokes> = layout.characters().map(|(_, s)| s).collect();
            for (index, stroke) in strokes.iter().enumerate() {
                assert!(
                    !strokes[index + 1..].contains(stroke),
                    "{layout:?} {stroke:?}"
                );
            }
        }
    }

    // Common keys

    #[test]
    fn letters_digits_and_whitespace() {
        for code in ["us", "de"] {
            assert_eq!(key(code, 'a'), Some((NONE, 0x04)));
            assert_eq!(key(code, 'C'), Some((SHIFT, 0x06)));
            assert_eq!(key(code, '1'), Some((NONE, 0x1E)));
            assert_eq!(key(code, '0'), Some((NONE, 0x27)));
        }
        for &layout in KeyboardLayout::ALL {
            let key =
                |character| keystrokes(layout, character).map(|s| (s.key.modifiers, s.key.usage));
            assert_eq!(key(' '), Some((NONE, SPACE)), "{layout:?}");
            assert_eq!(key('\n'), Some((NONE, ENTER)), "{layout:?}");
            assert_eq!(key('\t'), Some((NONE, TAB)), "{layout:?}");
        }
    }

    #[test]
    fn german_layout_swaps_y_and_z() {
        assert_eq!(key("us", 'y'), Some((NONE, 0x1C)));
        assert_eq!(key("us", 'z'), Some((NONE, 0x1D)));
        assert_eq!(key("de", 'y'), Some((NONE, 0x1D)));
        assert_eq!(key("de", 'Z'), Some((SHIFT, 0x1C)));
    }

    // Symbols

    #[test]
    fn us_symbols() {
        assert_eq!(key("us", '@'), Some((SHIFT, 0x1F)));
        assert_eq!(key("us", '/'), Some((NONE, SLASH)));
        assert_eq!(key("us", '"'), Some((SHIFT, QUOTE)));
        assert_eq!(key("us", '\\'), Some((NONE, BACKSLASH)));
        assert_eq!(key("us", '^'), Some((SHIFT, 0x23)));
        assert_eq!(keystrokes(layout("us"), 'ä'), None);
        assert_eq!(keystrokes(layout("us"), '€'), None);
    }

    #[test]
    fn german_symbols_and_umlauts() {
        assert_eq!(key("de", 'ä'), Some((NONE, QUOTE)));
        assert_eq!(key("de", 'Ö'), Some((SHIFT, SEMICOLON)));
        assert_eq!(key("de", 'ß'), Some((NONE, MINUS)));
        assert_eq!(key("de", '@'), Some((ALT_GR, 0x14)));
        assert_eq!(key("de", '€'), Some((ALT_GR, 0x08)));
        assert_eq!(key("de", '/'), Some((SHIFT, 0x24)));
        assert_eq!(key("de", '-'), Some((NONE, SLASH)));
        assert_eq!(key("de", '<'), Some((NONE, INTL_BACKSLASH)));
        assert_eq!(key("de", '|'), Some((ALT_GR, INTL_BACKSLASH)));
        assert_eq!(key("de", '\\'), Some((ALT_GR, MINUS)));
        assert_eq!(key("de", '#'), Some((NONE, INTL_HASH)));
    }

    #[test]
    fn german_dead_keys() {
        let de = layout("de");
        assert_eq!(
            keystrokes(de, '^'),
            Some(Keystrokes {
                dead_key: Some(stroke(NONE, BACKQUOTE)),
                key: stroke(NONE, SPACE),
            })
        );
        assert_eq!(
            keystrokes(de, 'é'),
            Some(Keystrokes {
                dead_key: Some(stroke(NONE, EQUAL)),
                key: stroke(NONE, 0x08),
            })
        );
        assert_eq!(
            keystrokes(de, 'À'),
            Some(Keystrokes {
                dead_key: Some(stroke(SHIFT, EQUAL)),
                key: stroke(SHIFT, 0x04),
            })
        );
        assert_eq!(
            keystrokes(de, 'Û'),
            Some(Keystrokes {
                dead_key: Some(stroke(NONE, BACKQUOTE)),
                key: stroke(SHIFT, 0x18),
            })
        );
    }

    #[test]
    fn other_layouts() {
        assert_eq!(key("fr", 'a'), Some((NONE, 0x14)));
        assert_eq!(key("fr", 'M'), Some((SHIFT, SEMICOLON)));
        assert_eq!(key("fr", '1'), Some((SHIFT, 0x1E)));
        assert_eq!(key("fr", 'é'), Some((NONE, 0x1F)));
        assert_eq!(key("fr", '@'), Some((ALT_GR, 0x27)));
        assert_eq!(key("de-CH", 'è'), Some((SHIFT, 0x2F)));
        assert_eq!(key("de-CH", '@'), Some((ALT_GR, 0x1F)));
        assert_eq!(key("de-CH", 'z'), Some((NONE, 0x1C)));
        assert_eq!(key("en-GB", '£'), Some((SHIFT, 0x20)));
        assert_eq!(key("en-GB", '@'), Some((SHIFT, QUOTE)));
        assert_eq!(key("en-GB", '#'), Some((NONE, INTL_HASH)));
        assert_eq!(key("en-GB", '\\'), Some((NONE, INTL_BACKSLASH)));
    }

    #[test]
    fn every_printable_ascii_character_can_be_typed() {
        for &layout in KeyboardLayout::ALL {
            for byte in 0x20u8..0x7F {
                assert!(
                    keystrokes(layout, byte as char).is_some(),
                    "{layout:?} {:?}",
                    byte as char
                );
            }
        }
    }

    #[test]
    fn can_type_checks_every_character() {
        assert!(can_type(layout("de"), "Größe: 5 €"));
        assert!(!can_type(layout("us"), "Größe"));
        assert!(!can_type(layout("de"), "😀"));
        assert!(can_type(layout("us"), ""));
    }
}
