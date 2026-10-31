use core::str::Chars;

use crate::hid::{KeyboardState, MAX_REPORT_SIZE, consumer_report, stroke_report};
use crate::layouts::{KeyboardLayout, Stroke, keystrokes};
use crate::macros::{Action, Actions};

// Constants

pub const MEDIA_PRESS_MS: u32 = 30;

// Types

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Output {
    Keyboard([u8; MAX_REPORT_SIZE]),
    Consumer([u8; 3]),
    Wait(u32),
}

// Text

pub struct TextStrokes<'a> {
    characters: Chars<'a>,
    keyboard_layout: KeyboardLayout,
    pending: Option<Stroke>,
}

impl<'a> TextStrokes<'a> {
    pub fn new(keyboard_layout: KeyboardLayout, text: &'a str) -> Self {
        Self {
            characters: text.chars(),
            keyboard_layout,
            pending: None,
        }
    }
}

impl Iterator for TextStrokes<'_> {
    type Item = Stroke;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(stroke) = self.pending.take() {
            return Some(stroke);
        }
        loop {
            let character = self.characters.next()?;
            let Some(strokes) = keystrokes(self.keyboard_layout, character) else {
                continue;
            };
            return Some(match strokes.dead_key {
                Some(dead_key) => {
                    self.pending = Some(strokes.key);
                    dead_key
                }
                None => strokes.key,
            });
        }
    }
}

// Player

pub struct Player<'a> {
    actions: Actions<'a>,
    keyboard_layout: KeyboardLayout,
    state: KeyboardState,
    text: Option<TextStrokes<'a>>,
    release_stroke: bool,
    queued: [Option<Output>; 2],
    finished: bool,
}

impl<'a> Player<'a> {
    pub fn new(actions: Actions<'a>, keyboard_layout: KeyboardLayout) -> Self {
        Self {
            actions,
            keyboard_layout,
            state: KeyboardState::new(),
            text: None,
            release_stroke: false,
            queued: [None; 2],
            finished: false,
        }
    }

    fn take_queued(&mut self) -> Option<Output> {
        self.queued.iter_mut().find_map(Option::take)
    }

    fn next_text_output(&mut self) -> Option<Output> {
        let text = self.text.as_mut()?;
        if let Some(stroke) = text.next() {
            self.release_stroke = true;
            return Some(Output::Keyboard(stroke_report(Some(stroke))));
        }
        self.text = None;
        (!self.state.is_empty()).then(|| Output::Keyboard(self.state.report()))
    }
}

impl Iterator for Player<'_> {
    type Item = Output;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(output) = self.take_queued() {
                return Some(output);
            }
            if self.release_stroke {
                self.release_stroke = false;
                return Some(Output::Keyboard(stroke_report(None)));
            }
            if self.text.is_some() {
                match self.next_text_output() {
                    Some(output) => return Some(output),
                    None => continue,
                }
            }
            match self.actions.next() {
                Some(Action::Press(key)) => {
                    self.state.press(key);
                    return Some(Output::Keyboard(self.state.report()));
                }
                Some(Action::Release(key)) => {
                    self.state.release(key);
                    return Some(Output::Keyboard(self.state.report()));
                }
                Some(Action::Delay(ms)) => return Some(Output::Wait(ms)),
                Some(Action::Media(usage)) => {
                    self.queued = [
                        Some(Output::Wait(MEDIA_PRESS_MS)),
                        Some(Output::Consumer(consumer_report(0))),
                    ];
                    return Some(Output::Consumer(consumer_report(usage)));
                }
                Some(Action::Text(text)) => {
                    self.text = Some(TextStrokes::new(self.keyboard_layout, text));
                }
                None => {
                    if self.finished || self.state.is_empty() {
                        self.finished = true;
                        return None;
                    }
                    self.finished = true;
                    self.state.release_all();
                    return Some(Output::Keyboard(self.state.report()));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hid::KEYBOARD_REPORT_ID;
    use crate::macros::MacroBuffer;

    fn play(json: &str, keyboard_layout: KeyboardLayout) -> Vec<Output> {
        let buffer = MacroBuffer::compile(json.as_bytes()).unwrap();
        Player::new(buffer.actions(), keyboard_layout).collect()
    }

    fn keys(modifiers: u8, usage: u8) -> Output {
        Output::Keyboard([KEYBOARD_REPORT_ID, modifiers, 0, usage, 0, 0, 0, 0, 0])
    }

    // Keys and media

    #[test]
    fn plays_keys_delays_and_media() {
        assert_eq!(
            play(
                r#"{"actions":[{"press":"ControlLeft"},{"press":"KeyC"},{"delay":50},{"release":"KeyC"},{"release":"ControlLeft"},{"media":"mute"}]}"#,
                KeyboardLayout::Us
            ),
            [
                keys(0x01, 0),
                keys(0x01, 0x06),
                Output::Wait(50),
                keys(0x01, 0),
                keys(0, 0),
                Output::Consumer(consumer_report(0x00E2)),
                Output::Wait(MEDIA_PRESS_MS),
                Output::Consumer(consumer_report(0)),
            ]
        );
    }

    #[test]
    fn releases_held_keys_at_the_end() {
        assert_eq!(
            play(r#"{"actions":[{"press":"ShiftLeft"}]}"#, KeyboardLayout::Us),
            [keys(0x02, 0), keys(0, 0)]
        );
        assert!(play(r#"{"actions":[]}"#, KeyboardLayout::Us).is_empty());
    }

    // Text

    #[test]
    fn types_text_for_the_layout() {
        assert_eq!(
            play(r#"{"actions":[{"text":"Zy"}]}"#, KeyboardLayout::De),
            [keys(0x02, 0x1C), keys(0, 0), keys(0, 0x1D), keys(0, 0)]
        );
        assert_eq!(
            play(r#"{"actions":[{"text":"Zy"}]}"#, KeyboardLayout::Us),
            [keys(0x02, 0x1D), keys(0, 0), keys(0, 0x1C), keys(0, 0)]
        );
    }

    #[test]
    fn types_dead_keys_and_skips_unknown_characters() {
        assert_eq!(
            play(r#"{"actions":[{"text":"é😀\n"}]}"#, KeyboardLayout::De),
            [
                keys(0, 0x2E),
                keys(0, 0),
                keys(0, 0x08),
                keys(0, 0),
                keys(0, 0x28),
                keys(0, 0),
            ]
        );
        assert!(play(r#"{"actions":[{"text":"ä"}]}"#, KeyboardLayout::Us).is_empty());
    }

    #[test]
    fn restores_held_keys_after_text() {
        assert_eq!(
            play(
                r#"{"actions":[{"press":"ControlLeft"},{"text":"a"},{"release":"ControlLeft"}]}"#,
                KeyboardLayout::Us
            ),
            [
                keys(0x01, 0),
                keys(0, 0x04),
                keys(0, 0),
                keys(0x01, 0),
                keys(0, 0),
            ]
        );
    }
}
