// Constants

const NONE: u8 = 0x00;
const SHIFT: u8 = 0x02;
const ALT_GR: u8 = 0x40;

const ENTER: u8 = 0x28;
const TAB: u8 = 0x2B;
const SPACE: u8 = 0x2C;
const MINUS: u8 = 0x2D;
const EQUAL: u8 = 0x2E;
const BRACKET_LEFT: u8 = 0x2F;
const BRACKET_RIGHT: u8 = 0x30;
const BACKSLASH: u8 = 0x31;
const INTL_HASH: u8 = 0x32;
const SEMICOLON: u8 = 0x33;
const QUOTE: u8 = 0x34;
const BACKQUOTE: u8 = 0x35;
const COMMA: u8 = 0x36;
const PERIOD: u8 = 0x37;
const SLASH: u8 = 0x38;
const INTL_BACKSLASH: u8 = 0x64;

// Types

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum KeyboardLayout {
    #[default]
    Us,
    De,
}

impl KeyboardLayout {
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "us" => Some(KeyboardLayout::Us),
            "de" => Some(KeyboardLayout::De),
            _ => None,
        }
    }
}

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

const fn stroke(modifiers: u8, usage: u8) -> Stroke {
    Stroke { modifiers, usage }
}

const fn single(modifiers: u8, usage: u8) -> Keystrokes {
    Keystrokes {
        dead_key: None,
        key: stroke(modifiers, usage),
    }
}

// Lookup

pub fn keystrokes(layout: KeyboardLayout, character: char) -> Option<Keystrokes> {
    common(layout, character).or_else(|| match layout {
        KeyboardLayout::Us => {
            us_symbol(character).map(|(modifiers, usage)| single(modifiers, usage))
        }
        KeyboardLayout::De => de_symbol(character)
            .map(|(modifiers, usage)| single(modifiers, usage))
            .or_else(|| de_dead_key(character)),
    })
}

pub fn can_type(layout: KeyboardLayout, text: &str) -> bool {
    text.chars()
        .all(|character| keystrokes(layout, character).is_some())
}

fn common(layout: KeyboardLayout, character: char) -> Option<Keystrokes> {
    let (modifiers, usage) = match character {
        'a'..='z' => (NONE, letter_usage(layout, character)),
        'A'..='Z' => (SHIFT, letter_usage(layout, character.to_ascii_lowercase())),
        '1'..='9' => (NONE, 0x1E + (character as u8 - b'1')),
        '0' => (NONE, 0x27),
        ' ' => (NONE, SPACE),
        '\n' => (NONE, ENTER),
        '\t' => (NONE, TAB),
        _ => return None,
    };
    Some(single(modifiers, usage))
}

fn letter_usage(layout: KeyboardLayout, letter: char) -> u8 {
    let letter = match (layout, letter) {
        (KeyboardLayout::De, 'y') => 'z',
        (KeyboardLayout::De, 'z') => 'y',
        (_, other) => other,
    };
    0x04 + (letter as u8 - b'a')
}

fn digit(number: u8) -> u8 {
    if number == 0 { 0x27 } else { 0x1E + number - 1 }
}

fn us_symbol(character: char) -> Option<(u8, u8)> {
    Some(match character {
        '!' => (SHIFT, digit(1)),
        '@' => (SHIFT, digit(2)),
        '#' => (SHIFT, digit(3)),
        '$' => (SHIFT, digit(4)),
        '%' => (SHIFT, digit(5)),
        '^' => (SHIFT, digit(6)),
        '&' => (SHIFT, digit(7)),
        '*' => (SHIFT, digit(8)),
        '(' => (SHIFT, digit(9)),
        ')' => (SHIFT, digit(0)),
        '-' => (NONE, MINUS),
        '_' => (SHIFT, MINUS),
        '=' => (NONE, EQUAL),
        '+' => (SHIFT, EQUAL),
        '[' => (NONE, BRACKET_LEFT),
        '{' => (SHIFT, BRACKET_LEFT),
        ']' => (NONE, BRACKET_RIGHT),
        '}' => (SHIFT, BRACKET_RIGHT),
        '\\' => (NONE, BACKSLASH),
        '|' => (SHIFT, BACKSLASH),
        ';' => (NONE, SEMICOLON),
        ':' => (SHIFT, SEMICOLON),
        '\'' => (NONE, QUOTE),
        '"' => (SHIFT, QUOTE),
        '`' => (NONE, BACKQUOTE),
        '~' => (SHIFT, BACKQUOTE),
        ',' => (NONE, COMMA),
        '<' => (SHIFT, COMMA),
        '.' => (NONE, PERIOD),
        '>' => (SHIFT, PERIOD),
        '/' => (NONE, SLASH),
        '?' => (SHIFT, SLASH),
        _ => return None,
    })
}

fn de_symbol(character: char) -> Option<(u8, u8)> {
    Some(match character {
        '!' => (SHIFT, digit(1)),
        '"' => (SHIFT, digit(2)),
        '§' => (SHIFT, digit(3)),
        '$' => (SHIFT, digit(4)),
        '%' => (SHIFT, digit(5)),
        '&' => (SHIFT, digit(6)),
        '/' => (SHIFT, digit(7)),
        '(' => (SHIFT, digit(8)),
        ')' => (SHIFT, digit(9)),
        '=' => (SHIFT, digit(0)),
        '²' => (ALT_GR, digit(2)),
        '³' => (ALT_GR, digit(3)),
        '{' => (ALT_GR, digit(7)),
        '[' => (ALT_GR, digit(8)),
        ']' => (ALT_GR, digit(9)),
        '}' => (ALT_GR, digit(0)),
        'ß' => (NONE, MINUS),
        '?' => (SHIFT, MINUS),
        '\\' => (ALT_GR, MINUS),
        'ü' => (NONE, BRACKET_LEFT),
        'Ü' => (SHIFT, BRACKET_LEFT),
        '+' => (NONE, BRACKET_RIGHT),
        '*' => (SHIFT, BRACKET_RIGHT),
        '~' => (ALT_GR, BRACKET_RIGHT),
        '#' => (NONE, INTL_HASH),
        '\'' => (SHIFT, INTL_HASH),
        'ö' => (NONE, SEMICOLON),
        'Ö' => (SHIFT, SEMICOLON),
        'ä' => (NONE, QUOTE),
        'Ä' => (SHIFT, QUOTE),
        '°' => (SHIFT, BACKQUOTE),
        ',' => (NONE, COMMA),
        ';' => (SHIFT, COMMA),
        '.' => (NONE, PERIOD),
        ':' => (SHIFT, PERIOD),
        '-' => (NONE, SLASH),
        '_' => (SHIFT, SLASH),
        '<' => (NONE, INTL_BACKSLASH),
        '>' => (SHIFT, INTL_BACKSLASH),
        '|' => (ALT_GR, INTL_BACKSLASH),
        '@' => (ALT_GR, 0x14),
        '€' => (ALT_GR, 0x08),
        'µ' => (ALT_GR, 0x10),
        _ => return None,
    })
}

fn de_dead_key(character: char) -> Option<Keystrokes> {
    const CIRCUMFLEX: Stroke = stroke(NONE, BACKQUOTE);
    const ACUTE: Stroke = stroke(NONE, EQUAL);
    const GRAVE: Stroke = stroke(SHIFT, EQUAL);
    let (dead_key, base) = match character {
        '^' => (CIRCUMFLEX, ' '),
        '´' => (ACUTE, ' '),
        '`' => (GRAVE, ' '),
        'â' | 'ê' | 'î' | 'ô' | 'û' | 'Â' | 'Ê' | 'Î' | 'Ô' | 'Û' => {
            (CIRCUMFLEX, base_letter(character)?)
        }
        'á' | 'é' | 'í' | 'ó' | 'ú' | 'Á' | 'É' | 'Í' | 'Ó' | 'Ú' => {
            (ACUTE, base_letter(character)?)
        }
        'à' | 'è' | 'ì' | 'ò' | 'ù' | 'À' | 'È' | 'Ì' | 'Ò' | 'Ù' => {
            (GRAVE, base_letter(character)?)
        }
        _ => return None,
    };
    Some(Keystrokes {
        dead_key: Some(dead_key),
        key: common(KeyboardLayout::De, base)?.key,
    })
}

fn base_letter(character: char) -> Option<char> {
    const ACCENTED: [(&str, char); 5] = [
        ("âáàÂÁÀ", 'a'),
        ("êéèÊÉÈ", 'e'),
        ("îíìÎÍÌ", 'i'),
        ("ôóòÔÓÒ", 'o'),
        ("ûúùÛÚÙ", 'u'),
    ];
    let (variants, base) = ACCENTED
        .iter()
        .find(|(variants, _)| variants.contains(character))?;
    let upper = variants.chars().skip(3).any(|variant| variant == character);
    Some(if upper {
        base.to_ascii_uppercase()
    } else {
        *base
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(layout: KeyboardLayout, character: char) -> Option<(u8, u8)> {
        keystrokes(layout, character)
            .filter(|strokes| strokes.dead_key.is_none())
            .map(|strokes| (strokes.key.modifiers, strokes.key.usage))
    }

    // Common keys

    #[test]
    fn letters_digits_and_whitespace() {
        for layout in [KeyboardLayout::Us, KeyboardLayout::De] {
            assert_eq!(key(layout, 'a'), Some((NONE, 0x04)));
            assert_eq!(key(layout, 'C'), Some((SHIFT, 0x06)));
            assert_eq!(key(layout, '1'), Some((NONE, 0x1E)));
            assert_eq!(key(layout, '0'), Some((NONE, 0x27)));
            assert_eq!(key(layout, ' '), Some((NONE, SPACE)));
            assert_eq!(key(layout, '\n'), Some((NONE, ENTER)));
            assert_eq!(key(layout, '\t'), Some((NONE, TAB)));
        }
    }

    #[test]
    fn german_layout_swaps_y_and_z() {
        assert_eq!(key(KeyboardLayout::Us, 'y'), Some((NONE, 0x1C)));
        assert_eq!(key(KeyboardLayout::Us, 'z'), Some((NONE, 0x1D)));
        assert_eq!(key(KeyboardLayout::De, 'y'), Some((NONE, 0x1D)));
        assert_eq!(key(KeyboardLayout::De, 'Z'), Some((SHIFT, 0x1C)));
    }

    // Symbols

    #[test]
    fn us_symbols() {
        assert_eq!(key(KeyboardLayout::Us, '@'), Some((SHIFT, 0x1F)));
        assert_eq!(key(KeyboardLayout::Us, '/'), Some((NONE, SLASH)));
        assert_eq!(key(KeyboardLayout::Us, '"'), Some((SHIFT, QUOTE)));
        assert_eq!(key(KeyboardLayout::Us, '\\'), Some((NONE, BACKSLASH)));
        assert_eq!(key(KeyboardLayout::Us, '^'), Some((SHIFT, 0x23)));
        assert_eq!(keystrokes(KeyboardLayout::Us, 'ä'), None);
        assert_eq!(keystrokes(KeyboardLayout::Us, '€'), None);
    }

    #[test]
    fn german_symbols_and_umlauts() {
        assert_eq!(key(KeyboardLayout::De, 'ä'), Some((NONE, QUOTE)));
        assert_eq!(key(KeyboardLayout::De, 'Ö'), Some((SHIFT, SEMICOLON)));
        assert_eq!(key(KeyboardLayout::De, 'ß'), Some((NONE, MINUS)));
        assert_eq!(key(KeyboardLayout::De, '@'), Some((ALT_GR, 0x14)));
        assert_eq!(key(KeyboardLayout::De, '€'), Some((ALT_GR, 0x08)));
        assert_eq!(key(KeyboardLayout::De, '/'), Some((SHIFT, 0x24)));
        assert_eq!(key(KeyboardLayout::De, '-'), Some((NONE, SLASH)));
        assert_eq!(key(KeyboardLayout::De, '<'), Some((NONE, INTL_BACKSLASH)));
        assert_eq!(key(KeyboardLayout::De, '|'), Some((ALT_GR, INTL_BACKSLASH)));
        assert_eq!(key(KeyboardLayout::De, '\\'), Some((ALT_GR, MINUS)));
        assert_eq!(key(KeyboardLayout::De, '#'), Some((NONE, INTL_HASH)));
    }

    #[test]
    fn german_dead_keys() {
        assert_eq!(
            keystrokes(KeyboardLayout::De, '^'),
            Some(Keystrokes {
                dead_key: Some(stroke(NONE, BACKQUOTE)),
                key: stroke(NONE, SPACE),
            })
        );
        assert_eq!(
            keystrokes(KeyboardLayout::De, 'é'),
            Some(Keystrokes {
                dead_key: Some(stroke(NONE, EQUAL)),
                key: stroke(NONE, 0x08),
            })
        );
        assert_eq!(
            keystrokes(KeyboardLayout::De, 'À'),
            Some(Keystrokes {
                dead_key: Some(stroke(SHIFT, EQUAL)),
                key: stroke(SHIFT, 0x04),
            })
        );
        assert_eq!(
            keystrokes(KeyboardLayout::De, 'Û'),
            Some(Keystrokes {
                dead_key: Some(stroke(NONE, BACKQUOTE)),
                key: stroke(SHIFT, 0x18),
            })
        );
    }

    #[test]
    fn every_printable_ascii_character_can_be_typed() {
        for layout in [KeyboardLayout::Us, KeyboardLayout::De] {
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
        assert!(can_type(KeyboardLayout::De, "Größe: 5 €"));
        assert!(!can_type(KeyboardLayout::Us, "Größe"));
        assert!(!can_type(KeyboardLayout::De, "😀"));
        assert!(can_type(KeyboardLayout::Us, ""));
    }

    #[test]
    fn layout_names() {
        assert_eq!(KeyboardLayout::from_name("us"), Some(KeyboardLayout::Us));
        assert_eq!(KeyboardLayout::from_name("de"), Some(KeyboardLayout::De));
        assert_eq!(KeyboardLayout::from_name("fr"), None);
    }
}
