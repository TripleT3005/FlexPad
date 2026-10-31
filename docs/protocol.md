# FlexPad protocol and profile format

This document describes how the desktop app and the Rust firmware (`flexpad-firmware/`) talk to each other and what a profile looks like. It is the single reference for both sides; when the code and this document disagree, one of them has a bug.

## Serial connection

- USB CDC serial port of the FlexPad (USB ID `303A:1001`, product name `FlexPad`, serial number `FlexPad-<MAC>`). The baud rate setting is ignored.
- Text lines in UTF-8, each terminated with `\n`. A `\r` before it is ignored.
- Lines longer than 16 448 bytes are answered with `ERROR:line_too_long`.
- Every command gets exactly one response line. In addition, the device may send event lines (see [Events](#events)) at any time while the port is open.

## Commands

| Command | Response |
|---|---|
| `GET_DEVICE_INFO` | `DEVICE_INFO:FlexPad,<serial number>,<firmware version>` |
| `GET_CONFIG` | `CONFIG_DATA:<profile JSON>`, exactly as stored; `CONFIG_DATA:{}` if none is stored |
| `SET_CONFIG:<profile JSON>` | `CONFIG_SAVED`, or `CONFIG_ERROR:<reason>` (see [Errors](#errors)); on error the stored profile stays unchanged |
| `RUN_MACRO:<function JSON>` | Runs a [function object](#functions) once, e.g. `RUN_MACRO:{"actions":[{"media":"mute"}]}`; `MACRO_QUEUED`, or `MACRO_ERROR:<reason>` |
| `REBOOT_BOOTLOADER` | `REBOOTING:bootloader`; the device then restarts in the ROM download mode (USB ID `303A:0009`) for flashing |
| anything else | `ERROR:unknown_command` |

The profile JSON must fit on one line (no line breaks inside) and may be up to 16 384 bytes.

## Events

| Event | Meaning |
|---|---|
| `EVENT:PIN:<pin>:PRESSED` / `RELEASED` | A debounced change of a button pin, including unassigned pins |
| `EVENT:ENCODER:<pin a>:RIGHT` / `LEFT` | One detent of the encoder whose pin A is `<pin a>` |

Events that occur while no serial connection is open are dropped.

## Functions

What a tile does when it is pressed or turned is a function object. So far it holds one macro, a list of actions:

```json
{ "actions": [ { "press": "ControlLeft" }, { "press": "KeyC" }, { "delay": 50 }, { "release": "KeyC" }, { "release": "ControlLeft" } ] }
```

Each action is an object with exactly one member:

| Action | Meaning |
|---|---|
| `{ "press": "<key>" }` | Press a key |
| `{ "release": "<key>" }` | Release a key |
| `{ "delay": <ms> }` | Wait 0–60 000 ms (a whole number) |
| `{ "media": "<name>" }` | Press and release a media key: `vol_up`, `vol_down`, `mute`, `next`, `prev`, `play`, `stop` (also `AudioVolumeUp`, `AudioVolumeDown`, `AudioVolumeMute`, `MediaTrackNext`, `MediaTrackPrevious`, `MediaPlayPause`, `MediaStop`) |
| `{ "text": "<text>" }` | Type text, as a normal JSON string (commas, quotes, `\n` and `\\` need no extra escaping); accepted in profiles but not executed yet (#4, #5) |

An empty list (or a missing `actions`) means "nothing". Other members of a function object are ignored; they are reserved for functions that are not macros (#47).

Macros run one after another in the background (up to 8 waiting). After each macro, all keys are released. One macro may take up to 512 bytes on the device: about 170 key actions or 500 characters of text.

### Macro strings

Before function objects, a function was a string with comma-separated actions: `P:<key>`, `R:<key>`, `D:<ms>`, `MEDIA:<name>` and `TEXT:<text>`, e.g. `"P:ControlLeft,P:KeyC,D:50,R:KeyC,R:ControlLeft"`. The firmware still accepts such strings in profiles (also in format 1 and the Arduino format) so that a stored profile keeps working after a firmware update; in `TEXT:`, `\n` stands for a line break and a comma ends the text. The app converts them into function objects when a profile is loaded and only writes function objects.

### Key names

Key names describe the **physical key position** on a US keyboard and use the names of [`KeyboardEvent.code`](https://developer.mozilla.org/en-US/docs/Web/API/UI_Events/Keyboard_event_code_values), case-insensitive:

- Letters and digits: `KeyA`–`KeyZ`, `Digit0`–`Digit9` (short: `a`–`z`, `0`–`9`)
- Modifiers: `ControlLeft`, `ShiftLeft`, `AltLeft`, `MetaLeft`, `ControlRight`, `ShiftRight`, `AltRight`, `MetaRight`
- Function keys: `F1`–`F24`
- Others: `Enter`, `Escape`, `Backspace`, `Tab`, `Space`, `Minus`, `Equal`, `BracketLeft`, `BracketRight`, `Backslash`, `IntlHash`, `Semicolon`, `Quote`, `Backquote`, `Comma`, `Period`, `Slash`, `IntlBackslash`, `CapsLock`, `PrintScreen`, `ScrollLock`, `Pause`, `Insert`, `Home`, `PageUp`, `Delete`, `End`, `PageDown`, `ArrowRight`, `ArrowLeft`, `ArrowDown`, `ArrowUp`, `ContextMenu`
- Numpad: `NumLock`, `Numpad0`–`Numpad9`, `NumpadDivide`, `NumpadMultiply`, `NumpadSubtract`, `NumpadAdd`, `NumpadEnter`, `NumpadDecimal`

The computer's keyboard layout decides which character a position produces: on a German layout, `Semicolon` types `ö` and `KeyZ` types `y`.

The app writes these names and shows them with the labels of the chosen keyboard layout (e.g. `KeyY` as "Z" on a German layout). Older names from earlier versions, e.g. `lctrl`, `rshift`, `lwin`, `esc`, `left_brace`, `ael`, `oel`, `uel`, `sz`, `kp_0`, `kp_enter`, are converted by the app when a profile is loaded; the firmware still accepts them.

## Profile format 2

```json
{
  "format": 2,
  "profile_name": "Desk",
  "keyboard_layout": "de",
  "tiles": [
    {
      "id": "tile_1",
      "display_name": "Copy",
      "component": "button",
      "kind": "button",
      "pins": { "switch": 4 },
      "parameters": { "debounce_ms": 10 },
      "functions": {
        "press": { "actions": [ { "press": "ControlLeft" }, { "press": "KeyC" }, { "delay": 50 }, { "release": "KeyC" }, { "release": "ControlLeft" } ] }
      },
      "x": 50.0,
      "y": 50.0
    },
    {
      "id": "tile_2",
      "display_name": "Volume",
      "component": "ec11",
      "kind": "encoder",
      "pins": { "a": 15, "b": 16, "switch": 17 },
      "parameters": { "steps_per_detent": 4, "reverse": false, "debounce_ms": 10 },
      "functions": {
        "turn_right": { "actions": [ { "media": "vol_up" } ] },
        "turn_left": { "actions": [ { "media": "vol_down" } ] },
        "press": { "actions": [ { "media": "mute" } ] }
      },
      "x": 200.0,
      "y": 50.0
    }
  ]
}
```

The profile is written by the app, stored on the device unchanged and read back with `GET_CONFIG`, so any computer can show and edit the current configuration. The firmware only evaluates `format`, `tiles[].kind`, `pins`, `parameters` and `functions`; all other fields (`id`, `display_name`, `component`, `x`, `y`, …) belong to the app and are kept as they are.

- `pins`: role → GPIO number. A role without a pin is left out (or `null`, `""`).
- `functions`: function → [function object](#functions). A missing function or an empty `actions` list means "nothing".
- `parameters`: missing values use the defaults below; unknown parameters are ignored.
- `component` names the component the tile was created from (`button`, `ec11`). The app uses it for the display; the firmware ignores it.
- `keyboard_layout` (`us` or `de`, default `us`) is the layout of the computer the pad is used with. The app uses it to label the keys; the firmware ignores it so far (it will be used for text actions, #5).

### Kinds

| Kind | Pin roles | Functions | Parameters (default) |
|---|---|---|---|
| `button` | `switch` | `press` | `debounce_ms` 0–200 (10) |
| `encoder` | `a`, `b`, `switch` (optional) | `turn_right`, `turn_left`, `press` | `steps_per_detent` 1, 2 or 4 (4); `reverse` (false); `debounce_ms` for the switch (10) |

- Buttons are wired between the pin and GND; internal pull-ups are used.
- An encoder needs both `a` and `b` for turning. `steps_per_detent` is 4 for encoders that go through a full A/B cycle per detent (like the common EC11), 2 or 1 for others. `reverse` swaps the directions.

### Usable pins

1, 2, 4–18, 21, 38–42, 47, 48. Each pin may be used once across all tiles.

### Format 1

Profiles without `format` (from app versions before 0.1.0) are still accepted: each entry of `functions` is an object `{ "pin": "<pin>", "macro_str": "<macro>" }` and is treated as a button with default debounce. The format of the former Arduino firmware `{ "pins": { "<pin>": "<macro>" } }` is accepted as well. Both are only meant for the transition.

## Errors

`CONFIG_ERROR:<reason>`:

| Reason | Meaning |
|---|---|
| `too_large` | Profile larger than 16 384 bytes |
| `invalid_json:<position>` | Not valid JSON, e.g. a truncated transfer; byte position of the problem |
| `invalid_structure:<part>` | A part has the wrong type (`root`, `tiles`, `tile`, `pins`, `functions`, `function`, `parameters`) |
| `unsupported_format` | `format` is not 1 or 2 |
| `unknown_kind` / `unknown_role` / `unknown_function` | Not listed in [Kinds](#kinds) |
| `invalid_parameter:<name>` | Parameter outside its range |
| `invalid_pin` | Pin is not a number |
| `unusable_pin:<pin>` | Pin is not in the list of usable pins |
| `duplicate_pin:<pin>` | Pin used more than once |
| `invalid_macro:<pin>` | A function object or macro string is malformed or contains an unknown action, key or media name |
| `macro_too_large:<pin>` | A single macro takes more than 512 bytes on the device |
| `macros_too_large` | All macros together exceed 8 KB |
| `storage` | Writing to flash failed |

`MACRO_ERROR:<reason>`: `invalid_json:<position>`, `invalid_structure:<part>` (`function`, `actions`, `action`), `unknown_action:<name>`, `unknown_key:<name>`, `unknown_media:<name>`, `invalid_delay:<value>`, `invalid_text`, `unsupported:text` (text actions cannot run yet), `too_large` (more than 512 bytes), `busy` (queue full).

## Storage

The profile is stored in the flash partition `flexpad` (64 KB, see `flexpad-firmware/partitions.csv`) in two slots of 32 KB. Each save writes the other slot with a sequence number and a CRC32 and writes the header last; on start, the newest valid slot is used. An interrupted save therefore keeps the previous profile.
