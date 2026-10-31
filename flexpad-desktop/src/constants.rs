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

pub const PRESETS: &[(&str, &str)] = &[
    (
        "Copy [Universal]",
        "P:ControlLeft,P:KeyC,D:50,R:KeyC,R:ControlLeft",
    ),
    (
        "Paste [Universal]",
        "P:ControlLeft,P:KeyV,D:50,R:KeyV,R:ControlLeft",
    ),
    (
        "Cut [Universal]",
        "P:ControlLeft,P:KeyX,D:50,R:KeyX,R:ControlLeft",
    ),
    (
        "Find [Universal]",
        "P:ControlLeft,P:KeyF,D:50,R:KeyF,R:ControlLeft",
    ),
    (
        "New Tab [Universal]",
        "P:ControlLeft,P:KeyT,D:50,R:KeyT,R:ControlLeft",
    ),
    (
        "Close Tab [Universal]",
        "P:ControlLeft,P:KeyW,D:50,R:KeyW,R:ControlLeft",
    ),
    (
        "Select All [Universal]",
        "P:ControlLeft,P:KeyA,D:50,R:KeyA,R:ControlLeft",
    ),
    (
        "Task Manager [Windows]",
        "P:ControlLeft,P:ShiftLeft,P:Escape,D:50,R:Escape,R:ShiftLeft,R:ControlLeft",
    ),
    (
        "Lock PC [Windows]",
        "P:MetaLeft,P:KeyL,D:50,R:KeyL,R:MetaLeft",
    ),
    (
        "Lock PC [Linux Gnome]",
        "P:ControlLeft,P:AltLeft,P:KeyL,D:50,R:KeyL,R:AltLeft,R:ControlLeft",
    ),
    (
        "File Explorer [Windows]",
        "P:MetaLeft,P:KeyE,D:50,R:KeyE,R:MetaLeft",
    ),
    (
        "Emoji Picker [Windows]",
        "P:MetaLeft,P:Period,D:50,R:Period,R:MetaLeft",
    ),
    (
        "Close Window [Win/Linux]",
        "P:AltLeft,P:F4,D:50,R:F4,R:AltLeft",
    ),
    (
        "Open Terminal [Linux]",
        "P:ControlLeft,P:AltLeft,P:KeyT,D:50,R:KeyT,R:AltLeft,R:ControlLeft",
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    // Presets

    #[test]
    fn presets_use_position_names_known_to_the_firmware() {
        for (name, macro_str) in PRESETS {
            assert_eq!(&crate::keys::migrate_macro(macro_str), macro_str, "{name}");
            for token in macro_str.split(',') {
                if let Some(key) = token
                    .strip_prefix("P:")
                    .or_else(|| token.strip_prefix("R:"))
                {
                    assert!(
                        crate::keys::firmware_keys::key_from_name(key).is_some(),
                        "{name}: {key}"
                    );
                }
            }
        }
    }
}
