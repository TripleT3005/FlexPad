use crate::macros::Preset;

pub const FIRMWARE_PINS: &[u8] = &[
    1, 2, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 21, 38, 39, 40, 41, 42, 47, 48,
];

pub const MEDIA_ACTIONS: &[(&str, &str)] = &[
    ("Volume Up", "vol_up"),
    ("Volume Down", "vol_down"),
    ("Mute", "mute"),
    ("Next Track", "next"),
    ("Previous Track", "prev"),
    ("Play/Pause", "play"),
];

pub const PRESETS: &[(&str, Preset)] = &[
    ("Copy [Universal]", Preset::Chord(&["ControlLeft", "KeyC"])),
    ("Paste [Universal]", Preset::Chord(&["ControlLeft", "KeyV"])),
    ("Cut [Universal]", Preset::Chord(&["ControlLeft", "KeyX"])),
    ("Find [Universal]", Preset::Chord(&["ControlLeft", "KeyF"])),
    (
        "New Tab [Universal]",
        Preset::Chord(&["ControlLeft", "KeyT"]),
    ),
    (
        "Close Tab [Universal]",
        Preset::Chord(&["ControlLeft", "KeyW"]),
    ),
    (
        "Select All [Universal]",
        Preset::Chord(&["ControlLeft", "KeyA"]),
    ),
    (
        "Task Manager [Windows]",
        Preset::Chord(&["ControlLeft", "ShiftLeft", "Escape"]),
    ),
    ("Lock PC [Windows]", Preset::Chord(&["MetaLeft", "KeyL"])),
    (
        "Lock PC [Linux Gnome]",
        Preset::Chord(&["ControlLeft", "AltLeft", "KeyL"]),
    ),
    (
        "File Explorer [Windows]",
        Preset::Chord(&["MetaLeft", "KeyE"]),
    ),
    (
        "Emoji Picker [Windows]",
        Preset::Chord(&["MetaLeft", "Period"]),
    ),
    (
        "Close Window [Win/Linux]",
        Preset::Chord(&["AltLeft", "F4"]),
    ),
    (
        "Open Terminal [Linux]",
        Preset::Chord(&["ControlLeft", "AltLeft", "KeyT"]),
    ),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::macros::MacroAction;

    // Presets

    #[test]
    fn presets_use_position_names_known_to_the_firmware() {
        for (name, preset) in PRESETS {
            let mut migrated = preset.to_macro();
            migrated.migrate_keys();
            assert_eq!(migrated, preset.to_macro(), "{name}");
            for action in preset.to_macro().actions {
                if let MacroAction::Press(key) | MacroAction::Release(key) = action {
                    assert!(
                        crate::keys::firmware_keys::key_from_name(&key).is_some(),
                        "{name}: {key}"
                    );
                }
            }
        }
    }
}
