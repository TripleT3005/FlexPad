use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::profile::{self, Profile};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MacroAction {
    PressKey(String),
    ReleaseKey(String),
    Delay(u32),
    Media(String),
    TypeText(String),
}

impl std::fmt::Display for MacroAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MacroAction::PressKey(k) => write!(f, "Press: {}", k),
            MacroAction::ReleaseKey(k) => write!(f, "Release: {}", k),
            MacroAction::Delay(ms) => write!(f, "Delay: {} ms", ms),
            MacroAction::Media(m) => write!(f, "Media: {}", m),
            MacroAction::TypeText(t) => {
                let preview = t.replace('\n', "\\n");
                write!(f, "Text: \"{}\"", preview)
            }
        }
    }
}

pub fn map_to_us_layout(input: &str) -> String {
    let mut result = String::new();
    for c in input.chars() {
        match c {
            'ä' => result.push_str("ae"),
            'ö' => result.push_str("oe"),
            'ü' => result.push_str("ue"),
            'Ä' => result.push_str("Ae"),
            'Ö' => result.push_str("Oe"),
            'Ü' => result.push_str("Ue"),
            'ß' => result.push_str("ss"),
            'z' => result.push('y'),
            'y' => result.push('z'),
            'Z' => result.push('Y'),
            'Y' => result.push('Z'),
            other => result.push(other),
        }
    }
    result
}

pub fn parse_macro(macro_str: &str) -> Vec<MacroAction> {
    let mut actions = Vec::new();
    if macro_str.trim().is_empty() {
        return actions;
    }
    for part in macro_str.split(',') {
        let part = part.trim();
        if part.starts_with("P:") {
            actions.push(MacroAction::PressKey(
                part.strip_prefix("P:").unwrap().to_string(),
            ));
        } else if part.starts_with("R:") {
            actions.push(MacroAction::ReleaseKey(
                part.strip_prefix("R:").unwrap().to_string(),
            ));
        } else if part.starts_with("D:") {
            if let Ok(ms) = part.strip_prefix("D:").unwrap().parse::<u32>() {
                actions.push(MacroAction::Delay(ms));
            }
        } else if part.starts_with("MEDIA:") {
            actions.push(MacroAction::Media(
                part.strip_prefix("MEDIA:").unwrap().to_string(),
            ));
        } else if part.starts_with("TEXT:") {
            let text_val = part.strip_prefix("TEXT:").unwrap().replace("\\n", "\n");
            actions.push(MacroAction::TypeText(text_val));
        }
    }
    actions
}

pub fn compile_macro(actions: &[MacroAction]) -> String {
    actions
        .iter()
        .map(|action| match action {
            MacroAction::PressKey(k) => format!("P:{}", k),
            MacroAction::ReleaseKey(k) => format!("R:{}", k),
            MacroAction::Delay(ms) => format!("D:{}", ms),
            MacroAction::Media(m) => format!("MEDIA:{}", m),
            MacroAction::TypeText(t) => {
                let mapped = map_to_us_layout(t);
                let encoded = mapped.replace('\n', "\\n");
                format!("TEXT:{}", encoded)
            }
        })
        .collect::<Vec<String>>()
        .join(",")
}

pub fn get_base_dir() -> PathBuf {
    let home_dir = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));

    let path = home_dir.join("FlexPad");
    if !path.exists() {
        let _ = fs::create_dir_all(&path);
    }
    path
}

pub fn get_profiles_dir() -> PathBuf {
    let path = get_base_dir().join("profiles");
    if !path.exists() {
        let _ = fs::create_dir_all(&path);
    }
    path
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserSettings {
    pub last_profile: Option<String>,
    #[serde(default)]
    pub advanced_mode: bool,
}

pub fn get_settings_path() -> PathBuf {
    get_base_dir().join("settings.json")
}

pub fn load_user_settings() -> UserSettings {
    let path = get_settings_path();
    if let Ok(content) = fs::read_to_string(path) {
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        UserSettings::default()
    }
}

pub fn save_user_settings(settings: &UserSettings) {
    let path = get_settings_path();
    if let Ok(json) = serde_json::to_string_pretty(settings) {
        let _ = fs::write(path, json);
    }
}

pub fn save_profile_to_disk(config: &Profile) -> Result<PathBuf, String> {
    let dir = get_profiles_dir();
    let file_name = format!("{}.json", config.profile_name.replace(' ', "_"));
    let file_path = dir.join(file_name);

    let json_str = config.to_pretty_json()?;
    fs::write(&file_path, json_str).map_err(|e| e.to_string())?;

    Ok(file_path)
}

pub fn load_profile_from_disk(file_path: &PathBuf) -> Result<Profile, String> {
    let json_str = fs::read_to_string(file_path).map_err(|e| e.to_string())?;
    profile::parse_profile(&json_str)
}

pub fn list_saved_profiles() -> Vec<(String, PathBuf)> {
    let dir = get_profiles_dir();
    let mut profiles = Vec::new();

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json")
                && let Some(file_stem) = path.file_stem().and_then(|s| s.to_str())
            {
                profiles.push((file_stem.to_string(), path));
            }
        }
    }
    profiles
}

#[cfg(test)]
mod tests {
    use super::*;

    // Macro parsing

    #[test]
    fn parse_macro_reads_all_action_types() {
        let actions = parse_macro("P:lctrl,P:c,D:50,R:c,R:lctrl,MEDIA:vol_up,TEXT:Hello");

        assert_eq!(
            actions,
            vec![
                MacroAction::PressKey("lctrl".to_string()),
                MacroAction::PressKey("c".to_string()),
                MacroAction::Delay(50),
                MacroAction::ReleaseKey("c".to_string()),
                MacroAction::ReleaseKey("lctrl".to_string()),
                MacroAction::Media("vol_up".to_string()),
                MacroAction::TypeText("Hello".to_string()),
            ]
        );
    }

    #[test]
    fn parse_macro_returns_nothing_for_empty_input() {
        assert!(parse_macro("").is_empty());
        assert!(parse_macro("   ").is_empty());
    }

    #[test]
    fn parse_macro_ignores_spaces_around_tokens() {
        let actions = parse_macro(" P:a , D:10 ,R:a ");

        assert_eq!(
            actions,
            vec![
                MacroAction::PressKey("a".to_string()),
                MacroAction::Delay(10),
                MacroAction::ReleaseKey("a".to_string()),
            ]
        );
    }

    #[test]
    fn parse_macro_skips_unknown_and_invalid_tokens() {
        let actions = parse_macro("X:foo,D:abc,D:-5,P:a,garbage");

        assert_eq!(actions, vec![MacroAction::PressKey("a".to_string())]);
    }

    #[test]
    fn parse_macro_decodes_line_breaks_in_text() {
        let actions = parse_macro(r"TEXT:first\nsecond");

        assert_eq!(
            actions,
            vec![MacroAction::TypeText("first\nsecond".to_string())]
        );
    }

    // Macro compiling

    #[test]
    fn compile_macro_writes_all_action_types() {
        let actions = vec![
            MacroAction::PressKey("lctrl".to_string()),
            MacroAction::Delay(50),
            MacroAction::ReleaseKey("lctrl".to_string()),
            MacroAction::Media("mute".to_string()),
            MacroAction::TypeText("Hello".to_string()),
        ];

        assert_eq!(
            compile_macro(&actions),
            "P:lctrl,D:50,R:lctrl,MEDIA:mute,TEXT:Hello"
        );
    }

    #[test]
    fn compile_macro_returns_empty_string_for_no_actions() {
        assert_eq!(compile_macro(&[]), "");
    }

    #[test]
    fn compile_macro_encodes_line_breaks_in_text() {
        let actions = vec![MacroAction::TypeText("first\nsecond".to_string())];

        assert_eq!(compile_macro(&actions), r"TEXT:first\nsecond");
    }

    // Roundtrip

    fn assert_roundtrip(actions: Vec<MacroAction>) {
        let compiled = compile_macro(&actions);
        assert_eq!(parse_macro(&compiled), actions, "compiled: {compiled}");
    }

    #[test]
    fn roundtrip_keeps_keys_delays_and_media() {
        assert_roundtrip(vec![
            MacroAction::PressKey("lwin".to_string()),
            MacroAction::PressKey("period".to_string()),
            MacroAction::Delay(0),
            MacroAction::Delay(u32::MAX),
            MacroAction::ReleaseKey("period".to_string()),
            MacroAction::ReleaseKey("lwin".to_string()),
            MacroAction::Media("play".to_string()),
        ]);
    }

    #[test]
    fn roundtrip_keeps_plain_text_with_line_breaks() {
        assert_roundtrip(vec![MacroAction::TypeText("Hello\nWorld".to_string())]);
    }

    #[test]
    fn roundtrip_is_stable_when_compiled_twice() {
        let actions = vec![
            MacroAction::PressKey("lctrl".to_string()),
            MacroAction::TypeText("Hello".to_string()),
        ];

        let first = compile_macro(&actions);
        let second = compile_macro(&parse_macro(&first));

        assert_eq!(first, second);
    }

    #[test]
    #[ignore = "known bug, see #6"]
    fn roundtrip_keeps_y_and_z_in_text() {
        assert_roundtrip(vec![MacroAction::TypeText("xyz XYZ".to_string())]);
    }

    #[test]
    #[ignore = "known bug, see #6"]
    fn roundtrip_keeps_umlauts_in_text() {
        assert_roundtrip(vec![MacroAction::TypeText("Größe ändern".to_string())]);
    }

    #[test]
    #[ignore = "known bug, see #6"]
    fn roundtrip_keeps_commas_in_text() {
        assert_roundtrip(vec![
            MacroAction::TypeText("Hello, world".to_string()),
            MacroAction::Delay(10),
        ]);
    }

    #[test]
    #[ignore = "known bug, see #6"]
    fn roundtrip_keeps_backslashes_and_spaces_in_text() {
        assert_roundtrip(vec![MacroAction::TypeText(r" C:\new\folder ".to_string())]);
    }
}
