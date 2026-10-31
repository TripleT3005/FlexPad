use serde::{Deserialize, Serialize};

use crate::keys;

// Constants

const CHORD_DELAY_MS: u32 = 50;

// Types

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MacroAction {
    Press(String),
    Release(String),
    Delay(u32),
    Media(String),
    Text(String),
}

impl std::fmt::Display for MacroAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MacroAction::Press(key) => write!(f, "Press: {key}"),
            MacroAction::Release(key) => write!(f, "Release: {key}"),
            MacroAction::Delay(ms) => write!(f, "Delay: {ms} ms"),
            MacroAction::Media(name) => write!(f, "Media: {name}"),
            MacroAction::Text(text) => write!(f, "Text: \"{}\"", text.replace('\n', "\\n")),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Macro {
    #[serde(default)]
    pub actions: Vec<MacroAction>,
}

impl Macro {
    pub fn migrate_keys(&mut self) {
        for action in &mut self.actions {
            if let MacroAction::Press(key) | MacroAction::Release(key) = action
                && let Some(code) = keys::code_for(key)
            {
                *key = code.to_string();
            }
        }
    }
}

// Presets

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Preset {
    Chord(&'static [&'static str]),
    Media(&'static str),
}

impl Preset {
    pub fn to_macro(self) -> Macro {
        let actions = match self {
            Preset::Chord(keys) => keys
                .iter()
                .map(|key| MacroAction::Press(key.to_string()))
                .chain([MacroAction::Delay(CHORD_DELAY_MS)])
                .chain(
                    keys.iter()
                        .rev()
                        .map(|key| MacroAction::Release(key.to_string())),
                )
                .collect(),
            Preset::Media(name) => vec![MacroAction::Media(name.to_string())],
        };
        Macro { actions }
    }
}

// Legacy strings

pub fn parse_legacy(macro_str: &str) -> Macro {
    let mut actions = Vec::new();
    for token in macro_str.split(',') {
        let trimmed = token.trim();
        let action = if let Some(key) = trimmed.strip_prefix("P:") {
            Some(MacroAction::Press(key.to_string()))
        } else if let Some(key) = trimmed.strip_prefix("R:") {
            Some(MacroAction::Release(key.to_string()))
        } else if let Some(ms) = trimmed.strip_prefix("D:") {
            ms.parse().ok().map(MacroAction::Delay)
        } else if let Some(name) = trimmed.strip_prefix("MEDIA:") {
            Some(MacroAction::Media(name.to_string()))
        } else if let Some(text) = trimmed.strip_prefix("TEXT:") {
            Some(MacroAction::Text(text.replace("\\n", "\n")))
        } else if let Some(MacroAction::Text(text)) = actions.last_mut() {
            text.push(',');
            text.push_str(&token.replace("\\n", "\n"));
            None
        } else {
            None
        };
        actions.extend(action);
    }
    let mut parsed = Macro { actions };
    parsed.migrate_keys();
    parsed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn actions(json: &str) -> Vec<MacroAction> {
        serde_json::from_str::<Macro>(json).unwrap().actions
    }

    // Format

    #[test]
    fn serializes_actions_as_single_member_objects() {
        let value = Macro {
            actions: vec![
                MacroAction::Press("ControlLeft".to_string()),
                MacroAction::Delay(50),
                MacroAction::Release("ControlLeft".to_string()),
                MacroAction::Media("mute".to_string()),
                MacroAction::Text("Hello".to_string()),
            ],
        };

        assert_eq!(
            serde_json::to_string(&value).unwrap(),
            r#"{"actions":[{"press":"ControlLeft"},{"delay":50},{"release":"ControlLeft"},{"media":"mute"},{"text":"Hello"}]}"#
        );
    }

    #[test]
    fn reads_the_firmware_example() {
        assert_eq!(
            actions(
                r#"{"actions":[{"press":"ControlLeft"},{"press":"KeyC"},{"delay":50},{"release":"KeyC"},{"release":"ControlLeft"}]}"#
            ),
            Preset::Chord(&["ControlLeft", "KeyC"]).to_macro().actions
        );
        assert!(actions("{}").is_empty());
    }

    #[test]
    fn rejects_unknown_actions() {
        assert!(serde_json::from_str::<Macro>(r#"{"actions":[{"jump":1}]}"#).is_err());
        assert!(serde_json::from_str::<Macro>(r#"{"actions":["P:a"]}"#).is_err());
    }

    fn assert_roundtrip(text: &str) {
        let value = Macro {
            actions: vec![MacroAction::Text(text.to_string()), MacroAction::Delay(10)],
        };

        let json = serde_json::to_string(&value).unwrap();

        assert_eq!(
            serde_json::from_str::<Macro>(&json).unwrap(),
            value,
            "{json}"
        );
    }

    #[test]
    fn roundtrip_keeps_text_unchanged() {
        assert_roundtrip("xyz XYZ");
        assert_roundtrip("Größe ändern");
        assert_roundtrip("Hello, world");
        assert_roundtrip(r#" C:\new\folder "quoted" "#);
        assert_roundtrip("first\nsecond");
    }

    // Presets

    #[test]
    fn chords_press_in_order_and_release_in_reverse() {
        assert_eq!(
            Preset::Chord(&["ControlLeft", "ShiftLeft", "Escape"])
                .to_macro()
                .actions,
            vec![
                MacroAction::Press("ControlLeft".to_string()),
                MacroAction::Press("ShiftLeft".to_string()),
                MacroAction::Press("Escape".to_string()),
                MacroAction::Delay(50),
                MacroAction::Release("Escape".to_string()),
                MacroAction::Release("ShiftLeft".to_string()),
                MacroAction::Release("ControlLeft".to_string()),
            ]
        );
        assert_eq!(
            Preset::Media("mute").to_macro().actions,
            vec![MacroAction::Media("mute".to_string())]
        );
    }

    // Legacy strings

    #[test]
    fn parse_legacy_reads_all_action_types_and_migrates_keys() {
        assert_eq!(
            parse_legacy(" P:lctrl , P:c,D:50,R:c,R:lctrl,MEDIA:vol_up,TEXT:Hello").actions,
            vec![
                MacroAction::Press("ControlLeft".to_string()),
                MacroAction::Press("KeyC".to_string()),
                MacroAction::Delay(50),
                MacroAction::Release("KeyC".to_string()),
                MacroAction::Release("ControlLeft".to_string()),
                MacroAction::Media("vol_up".to_string()),
                MacroAction::Text("Hello".to_string()),
            ]
        );
    }

    #[test]
    fn parse_legacy_returns_nothing_for_empty_input() {
        assert!(parse_legacy("").actions.is_empty());
        assert!(parse_legacy("   ").actions.is_empty());
    }

    #[test]
    fn parse_legacy_skips_unknown_and_invalid_tokens() {
        assert_eq!(
            parse_legacy("X:foo,D:abc,D:-5,P:hyper,garbage").actions,
            vec![MacroAction::Press("hyper".to_string())]
        );
    }

    #[test]
    fn parse_legacy_keeps_commas_and_line_breaks_in_text() {
        assert_eq!(
            parse_legacy(r"TEXT:Hello, world\nbye,D:10").actions,
            vec![
                MacroAction::Text("Hello, world\nbye".to_string()),
                MacroAction::Delay(10),
            ]
        );
    }
}
