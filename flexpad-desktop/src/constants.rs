use crate::i18n;
use crate::macros::Preset;
use crate::texts;

// Pins

pub const FIRMWARE_PINS: &[u8] = &[
    1, 2, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 21, 38, 39, 40, 41, 42, 47, 48,
];

// Media keys

pub const MEDIA_ACTIONS: &[(&str, &str)] = &[
    (texts::MEDIA_VOL_UP, "vol_up"),
    (texts::MEDIA_VOL_DOWN, "vol_down"),
    (texts::MEDIA_MUTE, "mute"),
    (texts::MEDIA_NEXT, "next"),
    (texts::MEDIA_PREV, "prev"),
    (texts::MEDIA_PLAY, "play"),
];

pub fn media_label(action: &str) -> String {
    MEDIA_ACTIONS
        .iter()
        .find(|&&(_, known)| known == action)
        .map_or_else(|| action.to_string(), |&(label, _)| i18n::text(label))
}

// Presets

pub const PRESETS: &[(&str, Preset)] = &[
    (texts::PRESET_COPY, Preset::Chord(&["ControlLeft", "KeyC"])),
    (texts::PRESET_PASTE, Preset::Chord(&["ControlLeft", "KeyV"])),
    (texts::PRESET_CUT, Preset::Chord(&["ControlLeft", "KeyX"])),
    (texts::PRESET_FIND, Preset::Chord(&["ControlLeft", "KeyF"])),
    (
        texts::PRESET_NEW_TAB,
        Preset::Chord(&["ControlLeft", "KeyT"]),
    ),
    (
        texts::PRESET_CLOSE_TAB,
        Preset::Chord(&["ControlLeft", "KeyW"]),
    ),
    (
        texts::PRESET_SELECT_ALL,
        Preset::Chord(&["ControlLeft", "KeyA"]),
    ),
    (
        texts::PRESET_TASK_MANAGER,
        Preset::Chord(&["ControlLeft", "ShiftLeft", "Escape"]),
    ),
    (
        texts::PRESET_LOCK_WINDOWS,
        Preset::Chord(&["MetaLeft", "KeyL"]),
    ),
    (
        texts::PRESET_LOCK_GNOME,
        Preset::Chord(&["ControlLeft", "AltLeft", "KeyL"]),
    ),
    (
        texts::PRESET_FILE_EXPLORER,
        Preset::Chord(&["MetaLeft", "KeyE"]),
    ),
    (
        texts::PRESET_EMOJI_PICKER,
        Preset::Chord(&["MetaLeft", "Period"]),
    ),
    (
        texts::PRESET_CLOSE_WINDOW,
        Preset::Chord(&["AltLeft", "F4"]),
    ),
    (
        texts::PRESET_TERMINAL,
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
