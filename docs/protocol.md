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
| `RUN_MACRO:<macro>` | `MACRO_QUEUED`, or `MACRO_ERROR:<reason>` |
| `REBOOT_BOOTLOADER` | `REBOOTING:bootloader`; the device then restarts in the ROM download mode (USB ID `303A:0009`) for flashing |
| anything else | `ERROR:unknown_command` |

The profile JSON must fit on one line (no line breaks inside) and may be up to 16 384 bytes.

## Events

| Event | Meaning |
|---|---|
| `EVENT:PIN:<pin>:PRESSED` / `RELEASED` | A debounced change of a button pin, including unassigned pins |
| `EVENT:ENCODER:<pin a>:RIGHT` / `LEFT` | One detent of the encoder whose pin A is `<pin a>` |

Events that occur while no serial connection is open are dropped.

## Macros

A macro is a comma-separated list of actions:

| Action | Meaning |
|---|---|
| `P:<key>` | Press a key |
| `R:<key>` | Release a key |
| `D:<ms>` | Wait 0–60 000 ms |
| `MEDIA:<name>` | Press and release a media key: `vol_up`, `vol_down`, `mute`, `next`, `prev`, `play`, `stop` (also `AudioVolumeUp`, `AudioVolumeDown`, `AudioVolumeMute`, `MediaTrackNext`, `MediaTrackPrevious`, `MediaPlayPause`, `MediaStop`) |
| `TEXT:<text>` | Type text; accepted in profiles but not executed yet (#5). Commas inside the text are not escaped yet (#6), so such macros are rejected |

Macros run one after another in the background (up to 8 waiting). After each macro, all keys are released.

### Key names

Key names describe the **physical key position** on a US keyboard and use the names of [`KeyboardEvent.code`](https://developer.mozilla.org/en-US/docs/Web/API/UI_Events/Keyboard_event_code_values), case-insensitive:

- Letters and digits: `KeyA`–`KeyZ`, `Digit0`–`Digit9` (short: `a`–`z`, `0`–`9`)
- Modifiers: `ControlLeft`, `ShiftLeft`, `AltLeft`, `MetaLeft`, `ControlRight`, `ShiftRight`, `AltRight`, `MetaRight`
- Function keys: `F1`–`F24`
- Others: `Enter`, `Escape`, `Backspace`, `Tab`, `Space`, `Minus`, `Equal`, `BracketLeft`, `BracketRight`, `Backslash`, `IntlHash`, `Semicolon`, `Quote`, `Backquote`, `Comma`, `Period`, `Slash`, `IntlBackslash`, `CapsLock`, `PrintScreen`, `ScrollLock`, `Pause`, `Insert`, `Home`, `PageUp`, `Delete`, `End`, `PageDown`, `ArrowRight`, `ArrowLeft`, `ArrowDown`, `ArrowUp`, `ContextMenu`
- Numpad: `NumLock`, `Numpad0`–`Numpad9`, `NumpadDivide`, `NumpadMultiply`, `NumpadSubtract`, `NumpadAdd`, `NumpadEnter`, `NumpadDecimal`

The computer's keyboard layout decides which character a position produces: on a German layout, `Semicolon` types `ö` and `KeyZ` types `y`.

The firmware also accepts the older names used by the app so far, e.g. `lctrl`, `rshift`, `lwin`, `esc`, `left_brace`, `ael`, `oel`, `uel`, `sz`, `kp_0`, `kp_enter`. New profiles should use the names above.

## Profile format 2

```json
{
  "format": 2,
  "profile_name": "Desk",
  "tiles": [
    {
      "id": "tile_1",
      "display_name": "Copy",
      "component": "button",
      "kind": "button",
      "pins": { "switch": 4 },
      "parameters": { "debounce_ms": 10 },
      "functions": { "press": "P:ControlLeft,P:KeyC,D:50,R:KeyC,R:ControlLeft" },
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
      "functions": { "turn_right": "MEDIA:vol_up", "turn_left": "MEDIA:vol_down", "press": "MEDIA:mute" },
      "x": 200.0,
      "y": 50.0
    }
  ]
}
```

The profile is written by the app, stored on the device unchanged and read back with `GET_CONFIG`, so any computer can show and edit the current configuration. The firmware only evaluates `format`, `tiles[].kind`, `pins`, `parameters` and `functions`; all other fields (`id`, `display_name`, `component`, `x`, `y`, …) belong to the app and are kept as they are.

- `pins`: role → GPIO number. A role without a pin is left out (or `null`, `""`).
- `functions`: function → macro. An empty string means "nothing".
- `parameters`: missing values use the defaults below; unknown parameters are ignored.
- `component` names the component the tile was created from (`button`, `ec11`). The app uses it for the display; the firmware ignores it.

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

Profiles without `format` (from the app so far) are still accepted: each entry of `functions` is an object `{ "pin": "<pin>", "macro_str": "<macro>" }` and is treated as a button with default debounce. The format of the former Arduino firmware `{ "pins": { "<pin>": "<macro>" } }` is accepted as well. Both are only meant for the transition.

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
| `invalid_macro:<pin>` | A macro contains an unknown action, key or media name |
| `macros_too_large` | All macros together exceed 8 KB |
| `storage` | Writing to flash failed |

`MACRO_ERROR:<reason>`: `unknown_key:<name>`, `unknown_media:<name>`, `invalid_delay:<value>`, `unsupported:TEXT`, `unknown_action:<token>`, `busy` (queue full).

## Storage

The profile is stored in the flash partition `flexpad` (64 KB, see `flexpad-firmware/partitions.csv`) in two slots of 32 KB. Each save writes the other slot with a sequence number and a CRC32 and writes the header last; on start, the newest valid slot is used. An interrupted save therefore keeps the previous profile.
