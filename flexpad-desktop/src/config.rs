use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::profile::{self, Profile};

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
