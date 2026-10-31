use crate::keys::Key;
use crate::layouts::{KeyboardLayout, Stroke, keystrokes};

// Report descriptors

pub const KEYBOARD_REPORT_ID: u8 = 1;
pub const CONSUMER_REPORT_ID: u8 = 2;
pub const MAX_REPORT_SIZE: usize = 9;

pub const REPORT_DESCRIPTOR: &[u8] = &[
    0x05,
    0x01,
    0x09,
    0x06,
    0xA1,
    0x01,
    0x85,
    KEYBOARD_REPORT_ID,
    0x05,
    0x07,
    0x19,
    0xE0,
    0x29,
    0xE7,
    0x15,
    0x00,
    0x25,
    0x01,
    0x75,
    0x01,
    0x95,
    0x08,
    0x81,
    0x02,
    0x95,
    0x01,
    0x75,
    0x08,
    0x81,
    0x01,
    0x95,
    0x06,
    0x75,
    0x08,
    0x15,
    0x00,
    0x26,
    0xFF,
    0x00,
    0x05,
    0x07,
    0x19,
    0x00,
    0x2A,
    0xFF,
    0x00,
    0x81,
    0x00,
    0xC0,
    0x05,
    0x0C,
    0x09,
    0x01,
    0xA1,
    0x01,
    0x85,
    CONSUMER_REPORT_ID,
    0x15,
    0x00,
    0x26,
    0xFF,
    0x03,
    0x19,
    0x00,
    0x2A,
    0xFF,
    0x03,
    0x75,
    0x10,
    0x95,
    0x01,
    0x81,
    0x00,
    0xC0,
];

// Keyboard state

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct KeyboardState {
    modifiers: u8,
    keys: [u8; 6],
}

impl KeyboardState {
    pub const fn new() -> Self {
        Self {
            modifiers: 0,
            keys: [0; 6],
        }
    }

    pub fn press(&mut self, key: Key) {
        match key {
            Key::Modifier(bit) => self.modifiers |= bit,
            Key::Usage(usage) => {
                if self.keys.contains(&usage) {
                    return;
                }
                if let Some(slot) = self.keys.iter_mut().find(|slot| **slot == 0) {
                    *slot = usage;
                }
            }
        }
    }

    pub fn release(&mut self, key: Key) {
        match key {
            Key::Modifier(bit) => self.modifiers &= !bit,
            Key::Usage(usage) => {
                for slot in self.keys.iter_mut().filter(|slot| **slot == usage) {
                    *slot = 0;
                }
            }
        }
    }

    pub fn release_all(&mut self) {
        *self = Self::new();
    }

    pub fn is_empty(&self) -> bool {
        self.modifiers == 0 && self.keys.iter().all(|&key| key == 0)
    }

    pub fn report(&self) -> [u8; MAX_REPORT_SIZE] {
        let mut report = [0; MAX_REPORT_SIZE];
        report[0] = KEYBOARD_REPORT_ID;
        report[1] = self.modifiers;
        report[3..].copy_from_slice(&self.keys);
        report
    }
}

// Text

pub fn stroke_report(stroke: Option<Stroke>) -> [u8; MAX_REPORT_SIZE] {
    let mut report = [0; MAX_REPORT_SIZE];
    report[0] = KEYBOARD_REPORT_ID;
    if let Some(stroke) = stroke {
        report[1] = stroke.modifiers;
        report[3] = stroke.usage;
    }
    report
}

pub fn text_reports(
    layout: KeyboardLayout,
    text: &str,
) -> impl Iterator<Item = [u8; MAX_REPORT_SIZE]> + '_ {
    text.chars()
        .filter_map(move |character| keystrokes(layout, character))
        .flat_map(|strokes| {
            strokes
                .dead_key
                .into_iter()
                .chain([strokes.key])
                .flat_map(|stroke| [Some(stroke), None])
        })
        .map(stroke_report)
}

// Consumer control

pub fn consumer_report(usage: u16) -> [u8; 3] {
    let [low, high] = usage.to_le_bytes();
    [CONSUMER_REPORT_ID, low, high]
}

#[cfg(test)]
mod tests {
    use super::*;

    // Keyboard state

    #[test]
    fn press_and_release_build_reports() {
        let mut state = KeyboardState::new();

        state.press(Key::Modifier(0x01));
        state.press(Key::Usage(0x06));

        assert_eq!(state.report(), [1, 0x01, 0, 0x06, 0, 0, 0, 0, 0]);

        state.release(Key::Usage(0x06));
        state.release(Key::Modifier(0x01));

        assert_eq!(state.report(), [1, 0, 0, 0, 0, 0, 0, 0, 0]);
        assert!(state.is_empty());
    }

    #[test]
    fn pressing_twice_keeps_one_slot() {
        let mut state = KeyboardState::new();

        state.press(Key::Usage(0x04));
        state.press(Key::Usage(0x04));

        assert_eq!(state.report(), [1, 0, 0, 0x04, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn seventh_key_is_ignored() {
        let mut state = KeyboardState::new();

        for usage in 0x04..=0x0A {
            state.press(Key::Usage(usage));
        }

        assert_eq!(
            state.report(),
            [1, 0, 0, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09]
        );
    }

    #[test]
    fn release_all_clears_everything() {
        let mut state = KeyboardState::new();
        state.press(Key::Modifier(0x02));
        state.press(Key::Usage(0x04));

        state.release_all();

        assert!(state.is_empty());
    }

    // Consumer control

    #[test]
    fn consumer_report_is_little_endian() {
        assert_eq!(consumer_report(0x00E9), [2, 0xE9, 0x00]);
        assert_eq!(consumer_report(0), [2, 0, 0]);
    }

    // Descriptor

    #[test]
    fn descriptor_collections_are_balanced() {
        let mut depth = 0i32;
        let mut index = 0;
        while index < REPORT_DESCRIPTOR.len() {
            let prefix = REPORT_DESCRIPTOR[index];
            match prefix {
                0xA1 => depth += 1,
                0xC0 => depth -= 1,
                _ => {}
            }
            let size = match prefix & 0x03 {
                3 => 4,
                size => size as usize,
            };
            index += 1 + size;
        }
        assert_eq!(index, REPORT_DESCRIPTOR.len());
        assert_eq!(depth, 0);
    }

    // Text

    fn report(modifiers: u8, usage: u8) -> [u8; MAX_REPORT_SIZE] {
        [KEYBOARD_REPORT_ID, modifiers, 0, usage, 0, 0, 0, 0, 0]
    }

    #[test]
    fn text_presses_and_releases_every_character() {
        let reports: Vec<_> = text_reports(KeyboardLayout::De, "Zy").collect();

        assert_eq!(
            reports,
            [
                report(0x02, 0x1C),
                report(0, 0),
                report(0, 0x1D),
                report(0, 0),
            ]
        );
    }

    #[test]
    fn text_uses_dead_keys_and_skips_unknown_characters() {
        let reports: Vec<_> = text_reports(KeyboardLayout::De, "é😀").collect();

        assert_eq!(
            reports,
            [report(0, 0x2E), report(0, 0), report(0, 0x08), report(0, 0),]
        );
        assert_eq!(text_reports(KeyboardLayout::Us, "ä").count(), 0);
    }
}
