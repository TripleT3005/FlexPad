use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TileType {
    Button,
    EC11,
}

impl TileType {
    pub const ALL: &[TileType] = &[TileType::Button, TileType::EC11];
}

impl std::fmt::Display for TileType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TileType::Button => write!(f, "Button"),
            TileType::EC11 => write!(f, "EC11 Rotary Encoder"),
        }
    }
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileFunction {
    pub pin: String,
    pub macro_str: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasTile {
    pub id: String,
    pub display_name: String,
    pub tile_type: TileType,
    pub functions: BTreeMap<String, TileFunction>,
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlexPadConfig {
    #[serde(default = "default_profile_name")]
    pub profile_name: String,
    #[serde(default)]
    pub tiles: Vec<CanvasTile>,
}

fn default_profile_name() -> String {
    "Default_Profile".to_string()
}

impl Default for FlexPadConfig {
    fn default() -> Self {
        let mut default_functions = BTreeMap::new();
        default_functions.insert(
            "button".to_string(),
            TileFunction {
                pin: "1".to_string(),
                macro_str: "P:lctrl,P:c,D:50,R:c,R:lctrl".to_string(),
            },
        );

        let initial_tile = CanvasTile {
            id: "tile_1".to_string(),
            display_name: "Button 1".to_string(),
            tile_type: TileType::Button,
            functions: default_functions,
            x: 50.0,
            y: 50.0,
        };

        Self {
            profile_name: "Default_Profile".to_string(),
            tiles: vec![initial_tile],
        }
    }
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

pub fn save_profile_to_disk(config: &FlexPadConfig) -> Result<PathBuf, String> {
    let dir = get_profiles_dir();
    let file_name = format!("{}.json", config.profile_name.replace(' ', "_"));
    let file_path = dir.join(file_name);

    let json_str = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    fs::write(&file_path, json_str).map_err(|e| e.to_string())?;

    Ok(file_path)
}

pub fn load_profile_from_disk(file_path: &PathBuf) -> Result<FlexPadConfig, String> {
    let json_str = fs::read_to_string(file_path).map_err(|e| e.to_string())?;
    let config: FlexPadConfig = serde_json::from_str(&json_str).map_err(|e| e.to_string())?;
    Ok(config)
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
