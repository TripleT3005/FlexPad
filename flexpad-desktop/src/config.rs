use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::profile::{self, Profile};

// Constants

const MAX_FILE_STEM_LENGTH: usize = 64;
const FALLBACK_FILE_STEM: &str = "profile";
const RESERVED_FILE_STEMS: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

// Directories

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

// Settings

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

// Profiles

pub fn profile_file_stem(profile_name: &str) -> String {
    let stem: String = profile_name
        .trim()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .take(MAX_FILE_STEM_LENGTH)
        .collect();
    if stem.is_empty() {
        FALLBACK_FILE_STEM.to_string()
    } else if RESERVED_FILE_STEMS
        .iter()
        .any(|reserved| reserved.eq_ignore_ascii_case(&stem))
    {
        format!("{stem}_")
    } else {
        stem
    }
}

pub fn profile_path(profile_name: &str) -> PathBuf {
    get_profiles_dir().join(format!("{}.json", profile_file_stem(profile_name)))
}

pub fn save_profile_to_disk(config: &Profile) -> Result<PathBuf, String> {
    let file_path = profile_path(&config.profile_name);
    if file_path.parent() != Some(get_profiles_dir().as_path()) {
        return Err("The profile name gives an invalid file name.".to_string());
    }

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

    // Profile file names

    #[test]
    fn file_stems_keep_letters_digits_dashes_and_underscores() {
        assert_eq!(profile_file_stem("Desk-2_Größe"), "Desk-2_Größe");
        assert_eq!(profile_file_stem(" My Profile "), "My_Profile");
    }

    #[test]
    fn file_stems_replace_invalid_and_path_characters() {
        assert_eq!(
            profile_file_stem(r#"a<b>c:d"e/f\g|h?i*j"#),
            "a_b_c_d_e_f_g_h_i_j"
        );
        assert_eq!(profile_file_stem("../../settings"), "______settings");
        assert_eq!(profile_file_stem(".."), "__");
    }

    #[test]
    fn file_stems_avoid_empty_and_reserved_names() {
        assert_eq!(profile_file_stem(""), "profile");
        assert_eq!(profile_file_stem("   "), "profile");
        assert_eq!(profile_file_stem("CON"), "CON_");
        assert_eq!(profile_file_stem("nul"), "nul_");
        assert_eq!(profile_file_stem("COM1"), "COM1_");
        assert_eq!(profile_file_stem("COM10"), "COM10");
    }

    #[test]
    fn file_stems_are_limited_in_length() {
        assert_eq!(
            profile_file_stem(&"ä".repeat(100)).chars().count(),
            MAX_FILE_STEM_LENGTH
        );
    }

    #[test]
    fn profile_paths_stay_in_the_profiles_folder() {
        for name in [
            "../../settings",
            r"..\..\x",
            "/etc/passwd",
            r"C:\Windows\x",
            "",
        ] {
            assert_eq!(
                profile_path(name).parent(),
                Some(get_profiles_dir().as_path()),
                "{name}"
            );
        }
    }
}
