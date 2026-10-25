pub const FIRMWARE_PINS: &[i32] = &[
    1, 2, 4, 5, 6, 7, 8, 9, 10, 11, 
    12, 13, 14, 15, 16, 17, 18, 21, 
    33, 34, 35, 36, 37, 38, 39, 40, 
    41, 42, 47, 48
];

pub const AVAILABLE_KEYS: &[(&str, &str)] = &[
    ("Ctrl (Left)", "lctrl"),
    ("Ctrl (Right)", "rctrl"),
    ("Shift (Left)", "lshift"),
    ("Shift (Right)", "rshift"),
    ("Alt (Left)", "lalt"),
    ("Alt (Right)", "ralt"),
    ("Win/Super (Left)", "lwin"),
    ("Win/Super (Right)", "rwin"),
    ("Enter", "enter"),
    ("Escape", "esc"),
    ("Backspace", "backspace"),
    ("Tab", "tab"),
    ("Space", "space"),
    ("Arrow Up", "up"),
    ("Arrow Down", "down"),
    ("Arrow Left", "left"),
    ("Arrow Right", "right"),
    ("0", "0"), ("1", "1"), ("2", "2"), ("3", "3"), ("4", "4"),
    ("5", "5"), ("6", "6"), ("7", "7"), ("8", "8"), ("9", "9"),
    ("A", "a"), ("B", "b"), ("C", "c"), ("D", "d"), ("E", "e"), ("F", "f"), ("G", "g"),
    ("H", "h"), ("I", "i"), ("J", "j"), ("K", "k"), ("L", "l"), ("M", "m"), ("N", "n"),
    ("O", "o"), ("P", "p"), ("Q", "q"), ("R", "r"), ("S", "s"), ("T", "t"), ("U", "u"),
    ("V", "v"), ("W", "w"), ("X", "x"), ("Y", "y"), ("Z", "z"),
    ("Ä", "ael"), ("Ö", "oel"), ("Ü", "uel"), ("ß", "sz"),
    ("Minus (-)", "minus"), ("Equal (=)", "equal"), ("Bracket [", "left_brace"), ("Bracket ]", "right_brace"),
    ("Semicolon (;)", "semicolon"), ("Quote (')", "quote"), ("Backslash (\\)", "backslash"),
    ("Comma (,)", "comma"), ("Period (.)", "period"), ("Slash (/)", "slash"),
    ("Numpad 0", "kp_0"), ("Numpad 1", "kp_1"), ("Numpad 2", "kp_2"), ("Numpad 3", "kp_3"), ("Numpad 4", "kp_4"),
    ("Numpad 5", "kp_5"), ("Numpad 6", "kp_6"), ("Numpad 7", "kp_7"), ("Numpad 8", "kp_8"), ("Numpad 9", "kp_9"),
    ("Numpad Enter", "kp_enter"), ("Numpad Plus", "kp_plus"), ("Numpad Minus", "kp_minus"), ("Numpad Asterisk", "kp_asterisk"), ("Numpad Slash", "kp_slash"),
    ("F1", "f1"), ("F2", "f2"), ("F3", "f3"), ("F4", "f4"), ("F5", "f5"), ("F6", "f6"),
    ("F7", "f7"), ("F8", "f8"), ("F9", "f9"), ("F10", "f10"), ("F11", "f11"), ("F12", "f12"),
    ("F13", "f13"), ("F14", "f14"), ("F15", "f15"), ("F16", "f16"), ("F17", "f17"), ("F18", "f18"),
    ("F19", "f19"), ("F20", "f20"), ("F21", "f21"), ("F22", "f22"), ("F23", "f23"), ("F24", "f24")
];

pub const MEDIA_ACTIONS: &[(&str, &str)] = &[
    ("Volume Up", "vol_up"),
    ("Volume Down", "vol_down"),
    ("Mute", "mute"),
    ("Next Track", "next"),
    ("Previous Track", "prev"),
    ("Play/Pause", "play")
];

pub const PRESETS: &[(&str, &str)] = &[
    ("Copy [Universal]", "P:lctrl,P:c,D:50,R:c,R:lctrl"),
    ("Paste [Universal]", "P:lctrl,P:v,D:50,R:v,R:lctrl"),
    ("Cut [Universal]", "P:lctrl,P:x,D:50,R:x,R:lctrl"),
    ("Find [Universal]", "P:lctrl,P:f,D:50,R:f,R:lctrl"),
    ("New Tab [Universal]", "P:lctrl,P:t,D:50,R:t,R:lctrl"),
    ("Close Tab [Universal]", "P:lctrl,P:w,D:50,R:w,R:lctrl"),
    ("Select All [Universal]", "P:lctrl,P:a,D:50,R:a,R:lctrl"),
    ("Task Manager [Windows]", "P:lctrl,P:shift,P:esc,D:50,R:esc,R:shift,R:lctrl"),
    ("Lock PC [Windows]", "P:lwin,P:l,D:50,R:l,R:lwin"),
    ("Lock PC [Linux Gnome]", "P:lctrl,P:lalt,P:l,D:50,R:l,R:lalt,R:lctrl"),
    ("File Explorer [Windows]", "P:lwin,P:e,D:50,R:e,R:lwin"),
    ("Emoji Picker [Windows]", "P:lwin,P:period,D:50,R:period,R:lwin"),
    ("Close Window [Win/Linux]", "P:lalt,P:f4,D:50,R:f4,R:lalt"),
    ("Open Terminal [Linux]", "P:lctrl,P:lalt,P:t,D:50,R:t,R:lalt,R:lctrl"),
];