use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use crate::profile::{self, Profile};

// Constants

const APP_FOLDER: &str = "FlexPad";
const SETTINGS_FILE: &str = "settings.json";
const PROFILES_FOLDER: &str = "profiles";
#[cfg(all(unix, not(target_os = "macos")))]
const LINUX_APP_FOLDER: &str = "flexpad";
const MAX_FILE_STEM_LENGTH: usize = 64;
const FALLBACK_FILE_STEM: &str = "profile";
const RESERVED_FILE_STEMS: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

// Directories

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppDirs {
    pub config: PathBuf,
    pub data: PathBuf,
}

impl AppDirs {
    #[cfg(windows)]
    pub fn from_env(var: impl Fn(&str) -> Option<String>) -> Option<Self> {
        let base = absolute_var(&var, "APPDATA")?.join(APP_FOLDER);
        Some(Self {
            config: base.clone(),
            data: base,
        })
    }

    #[cfg(target_os = "macos")]
    pub fn from_env(var: impl Fn(&str) -> Option<String>) -> Option<Self> {
        let base = absolute_var(&var, "HOME")?
            .join("Library")
            .join("Application Support")
            .join(APP_FOLDER);
        Some(Self {
            config: base.clone(),
            data: base,
        })
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    pub fn from_env(var: impl Fn(&str) -> Option<String>) -> Option<Self> {
        let home = absolute_var(&var, "HOME");
        let config = absolute_var(&var, "XDG_CONFIG_HOME")
            .or_else(|| home.as_ref().map(|home| home.join(".config")))?;
        let data = absolute_var(&var, "XDG_DATA_HOME")
            .or_else(|| home.as_ref().map(|home| home.join(".local").join("share")))?;
        Some(Self {
            config: config.join(LINUX_APP_FOLDER),
            data: data.join(LINUX_APP_FOLDER),
        })
    }

    pub fn profiles(&self) -> PathBuf {
        self.data.join(PROFILES_FOLDER)
    }

    pub fn settings(&self) -> PathBuf {
        self.config.join(SETTINGS_FILE)
    }
}

fn absolute_var(var: &impl Fn(&str) -> Option<String>, name: &str) -> Option<PathBuf> {
    var(name)
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
}

fn env_var(name: &str) -> Option<String> {
    std::env::var(name).ok()
}

pub fn app_dirs() -> &'static AppDirs {
    static DIRS: OnceLock<AppDirs> = OnceLock::new();
    DIRS.get_or_init(|| {
        AppDirs::from_env(env_var).unwrap_or_else(|| AppDirs {
            config: PathBuf::from(APP_FOLDER),
            data: PathBuf::from(APP_FOLDER),
        })
    })
}

pub fn get_profiles_dir() -> PathBuf {
    let path = app_dirs().profiles();
    if !path.exists() {
        let _ = fs::create_dir_all(&path);
    }
    path
}

// Migration

pub fn legacy_dir(var: impl Fn(&str) -> Option<String>) -> Option<PathBuf> {
    absolute_var(&var, "USERPROFILE")
        .or_else(|| absolute_var(&var, "HOME"))
        .map(|home| home.join(APP_FOLDER))
}

pub fn migrate_legacy_files() -> usize {
    legacy_dir(env_var).map_or(0, |legacy| move_legacy_files(&legacy, app_dirs()))
}

fn move_legacy_files(legacy: &Path, dirs: &AppDirs) -> usize {
    if !legacy.is_dir() || legacy == dirs.config || legacy == dirs.data {
        return 0;
    }
    let mut moved = usize::from(move_file(&legacy.join(SETTINGS_FILE), &dirs.settings()));
    let legacy_profiles = legacy.join(PROFILES_FOLDER);
    if let Ok(entries) = fs::read_dir(&legacy_profiles) {
        let profiles = entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.extension().and_then(|s| s.to_str()) == Some("json"));
        for path in profiles {
            if let Some(name) = path.file_name() {
                moved += usize::from(move_file(&path, &dirs.profiles().join(name)));
            }
        }
    }
    let _ = fs::remove_dir(&legacy_profiles);
    let _ = fs::remove_dir(legacy);
    moved
}

fn move_file(from: &Path, to: &Path) -> bool {
    if !from.is_file() || to.exists() {
        return false;
    }
    if let Some(parent) = to.parent()
        && fs::create_dir_all(parent).is_err()
    {
        return false;
    }
    fs::rename(from, to).is_ok() || (fs::copy(from, to).is_ok() && fs::remove_file(from).is_ok())
}

// Settings

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSettings {
    pub last_profile: Option<String>,
    #[serde(default)]
    pub advanced_mode: bool,
    #[serde(default = "default_check_for_updates")]
    pub check_for_updates: bool,
}

fn default_check_for_updates() -> bool {
    true
}

impl Default for UserSettings {
    fn default() -> Self {
        Self {
            last_profile: None,
            advanced_mode: false,
            check_for_updates: default_check_for_updates(),
        }
    }
}

pub fn load_user_settings() -> UserSettings {
    let path = app_dirs().settings();
    if let Ok(content) = fs::read_to_string(path) {
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        UserSettings::default()
    }
}

pub fn save_user_settings(settings: &UserSettings) {
    let path = app_dirs().settings();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
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
    app_dirs()
        .profiles()
        .join(format!("{}.json", profile_file_stem(profile_name)))
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
    let dir = app_dirs().profiles();
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

    // Settings

    #[test]
    fn update_check_is_on_by_default() {
        assert!(UserSettings::default().check_for_updates);
        let old: UserSettings =
            serde_json::from_str(r#"{"last_profile":"A","advanced_mode":true}"#).unwrap();
        assert!(old.check_for_updates);
        assert!(old.advanced_mode);
        let off: UserSettings =
            serde_json::from_str(r#"{"last_profile":null,"check_for_updates":false}"#).unwrap();
        assert!(!off.check_for_updates);
    }

    // Directories

    fn vars<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |name| {
            pairs
                .iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| value.to_string())
        }
    }

    #[cfg(windows)]
    #[test]
    fn windows_uses_the_roaming_app_data_folder() {
        let dirs = AppDirs::from_env(vars(&[("APPDATA", r"C:\Users\u\AppData\Roaming")])).unwrap();

        assert_eq!(
            dirs.settings(),
            PathBuf::from(r"C:\Users\u\AppData\Roaming\FlexPad\settings.json")
        );
        assert_eq!(
            dirs.profiles(),
            PathBuf::from(r"C:\Users\u\AppData\Roaming\FlexPad\profiles")
        );
        assert_eq!(AppDirs::from_env(vars(&[("APPDATA", "relative")])), None);
        assert_eq!(AppDirs::from_env(vars(&[])), None);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_uses_application_support() {
        let dirs = AppDirs::from_env(vars(&[("HOME", "/Users/u")])).unwrap();

        assert_eq!(
            dirs.settings(),
            PathBuf::from("/Users/u/Library/Application Support/FlexPad/settings.json")
        );
        assert_eq!(
            dirs.profiles(),
            PathBuf::from("/Users/u/Library/Application Support/FlexPad/profiles")
        );
        assert_eq!(AppDirs::from_env(vars(&[])), None);
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    #[test]
    fn linux_follows_the_xdg_base_directories() {
        let dirs = AppDirs::from_env(vars(&[("HOME", "/home/u")])).unwrap();
        let custom = AppDirs::from_env(vars(&[
            ("HOME", "/home/u"),
            ("XDG_CONFIG_HOME", "/cfg"),
            ("XDG_DATA_HOME", "relative"),
        ]))
        .unwrap();

        assert_eq!(
            dirs.settings(),
            PathBuf::from("/home/u/.config/flexpad/settings.json")
        );
        assert_eq!(
            dirs.profiles(),
            PathBuf::from("/home/u/.local/share/flexpad/profiles")
        );
        assert_eq!(
            custom.settings(),
            PathBuf::from("/cfg/flexpad/settings.json")
        );
        assert_eq!(
            custom.profiles(),
            PathBuf::from("/home/u/.local/share/flexpad/profiles")
        );
        assert_eq!(AppDirs::from_env(vars(&[])), None);
    }

    // Migration

    fn temp_dir(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("flexpad-test-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn write(path: &Path, content: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    #[test]
    fn legacy_dir_is_flexpad_in_the_home_folder() {
        let (user, home) = if cfg!(windows) {
            (r"C:\Users\u", r"C:\h")
        } else {
            ("/home/u", "/h")
        };
        assert_eq!(
            legacy_dir(vars(&[("USERPROFILE", user), ("HOME", home)])),
            Some(PathBuf::from(user).join("FlexPad"))
        );
        assert_eq!(
            legacy_dir(vars(&[("HOME", home)])),
            Some(PathBuf::from(home).join("FlexPad"))
        );
        assert_eq!(legacy_dir(vars(&[("HOME", "relative")])), None);
        assert_eq!(legacy_dir(vars(&[])), None);
    }

    #[test]
    fn moves_settings_and_profiles_and_removes_the_empty_folder() {
        let root = temp_dir("move");
        let legacy = root.join("FlexPad");
        let dirs = AppDirs {
            config: root.join("config"),
            data: root.join("data"),
        };
        write(&legacy.join("settings.json"), "{}");
        write(&legacy.join("profiles").join("Desk.json"), "desk");

        let moved = move_legacy_files(&legacy, &dirs);

        assert_eq!(moved, 2);
        assert_eq!(fs::read_to_string(dirs.settings()).unwrap(), "{}");
        assert_eq!(
            fs::read_to_string(dirs.profiles().join("Desk.json")).unwrap(),
            "desk"
        );
        assert!(!legacy.exists());
        assert_eq!(move_legacy_files(&legacy, &dirs), 0);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn keeps_existing_files_and_unknown_files() {
        let root = temp_dir("keep");
        let legacy = root.join("FlexPad");
        let dirs = AppDirs {
            config: root.join("config"),
            data: root.join("data"),
        };
        write(&legacy.join("profiles").join("Desk.json"), "old");
        write(&legacy.join("profiles").join("notes.txt"), "notes");
        write(&dirs.profiles().join("Desk.json"), "new");

        let moved = move_legacy_files(&legacy, &dirs);

        assert_eq!(moved, 0);
        assert_eq!(
            fs::read_to_string(dirs.profiles().join("Desk.json")).unwrap(),
            "new"
        );
        assert!(legacy.join("profiles").join("Desk.json").exists());
        assert!(legacy.join("profiles").join("notes.txt").exists());
        fs::remove_dir_all(root).unwrap();
    }

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
                Some(app_dirs().profiles().as_path()),
                "{name}"
            );
        }
    }
}
