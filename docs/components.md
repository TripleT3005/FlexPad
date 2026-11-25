# Component definitions

[English](#english) · [Deutsch](#deutsch)

## English

The components a tile can be made of (e.g. a button, a toggle switch or an EC11 rotary encoder) are described in JSON files. The built-in ones are in [`flexpad-desktop/components`](../flexpad-desktop/components) and are compiled into the app. Your own definitions go into the folder `components` next to the settings: on Windows `%APPDATA%\FlexPad\components`, on macOS `~/Library/Application Support/FlexPad/components`, on Linux `~/.config/flexpad/components`. The app reads them at start; after adding or changing a file, restart it.

A definition only chooses names, pins, functions, default macros and parameters. How an input is read is up to the firmware, so each definition picks one of the kinds the firmware knows. A new component therefore needs no new firmware, as long as it works like one of these kinds.

### Kinds

| Kind | Pins | Functions | Parameters |
|---|---|---|---|
| `button` | `switch` (required) | `press` (required) | `debounce_ms`: number, 0 to 200 |
| `encoder` | `a`, `b` (required), `switch` | `turn_right`, `turn_left` (required), `press` (needs the pin `switch`) | `steps_per_detent`: choice of 1, 2, 4; `reverse`: flag; `debounce_ms`: number, 0 to 200 |
| `switch` | `switch` (required) | `on`, `off` (required) | `debounce_ms`: number, 0 to 200 |

A `switch` is a toggle switch: `on` runs when it is switched on (closed), `off` when it is switched off (open). It needs firmware 1.4.0 or newer.

### File format

The file name is the id followed by `.json`, e.g. `panel-encoder.json`. Example of an encoder without a push button that sends two steps per detent:

```json
{
  "id": "panel-encoder",
  "name": "Panel encoder",
  "kind": "encoder",
  "pins": [
    { "id": "a", "label": "pin.a" },
    { "id": "b", "label": "pin.b" }
  ],
  "functions": [
    { "id": "turn_right", "label": "function.turn_right", "default_macro": { "media": "vol_up" } },
    { "id": "turn_left", "label": "function.turn_left", "default_macro": { "media": "vol_down" } }
  ],
  "parameters": [
    { "id": "steps_per_detent", "label": "parameter.steps_per_detent", "type": "choice", "options": [1, 2, 4], "default": 2 },
    { "id": "reverse", "label": "parameter.reverse", "type": "flag", "default": false }
  ]
}
```

- `id`: lowercase letters, digits, `-` and `_`, at most 32 characters. The profile stores it in `component`.
- `name` and every `label`: a text key of the [language files](../flexpad-desktop/lang/en.lang) (e.g. `function.turn_right`), which is then shown translated, or plain text, which is shown as written.
- `pins` and `functions`: only those of the kind, each at most once, in the order shown in the Tile Inspector.
- `default_macro` (optional): the macro of a new tile, either `{ "chord": ["ControlLeft", "KeyC"] }` (keys pressed together, named as in the macro editor) or `{ "media": "mute" }` (`vol_up`, `vol_down`, `mute`, `next`, `prev`, `play`). Without it, the function starts empty.
- `parameters` (optional): `type` is `number` with `min`, `max` and `default`, `choice` with `options` and `default`, or `flag` with `default`. Ranges and options must stay within what the firmware accepts. Parameters left out use the firmware's default.

Files that do not follow these rules, or whose id is already used by a built-in component, are skipped. Tiles whose component is not available keep their settings, but cannot be edited until the definition is back.

## Deutsch

Die Bauteile, aus denen eine Kachel bestehen kann (z. B. ein Taster, ein Kippschalter oder ein EC11-Drehencoder), sind in JSON-Dateien beschrieben. Die eingebauten liegen in [`flexpad-desktop/components`](../flexpad-desktop/components) und werden in die App eingebunden. Eigene Definitionen kommen in den Ordner `components` neben den Einstellungen: unter Windows `%APPDATA%\FlexPad\components`, unter macOS `~/Library/Application Support/FlexPad/components`, unter Linux `~/.config/flexpad/components`. Die App liest sie beim Start; nach dem Hinzufügen oder Ändern einer Datei die App neu starten.

Eine Definition legt nur Namen, Pins, Funktionen, Standard-Makros und Parameter fest. Wie ein Eingang gelesen wird, bestimmt die Firmware; jede Definition wählt daher eine der Arten, die die Firmware kennt. Ein neues Bauteil braucht so keine neue Firmware, solange es wie eine dieser Arten funktioniert.

### Arten

| Art | Pins | Funktionen | Parameter |
|---|---|---|---|
| `button` | `switch` (Pflicht) | `press` (Pflicht) | `debounce_ms`: Zahl von 0 bis 200 |
| `encoder` | `a`, `b` (Pflicht), `switch` | `turn_right`, `turn_left` (Pflicht), `press` (braucht den Pin `switch`) | `steps_per_detent`: Auswahl aus 1, 2, 4; `reverse`: Schalter; `debounce_ms`: Zahl von 0 bis 200 |
| `switch` | `switch` (Pflicht) | `on`, `off` (Pflicht) | `debounce_ms`: Zahl von 0 bis 200 |

Ein `switch` ist ein Kippschalter: `on` läuft beim Einschalten (geschlossen), `off` beim Ausschalten (offen). Die Art braucht Firmware 1.4.0 oder neuer.

### Dateiformat

Der Dateiname ist die ID mit `.json`, z. B. `panel-encoder.json`. Das [Beispiel oben](#file-format) beschreibt einen Encoder ohne Drucktaster, der zwei Schritte pro Rastung sendet.

- `id`: Kleinbuchstaben, Ziffern, `-` und `_`, höchstens 32 Zeichen. Das Profil speichert sie in `component`.
- `name` und jedes `label`: ein Textschlüssel der [Sprachdateien](../flexpad-desktop/lang/en.lang) (z. B. `function.turn_right`), der dann übersetzt angezeigt wird, oder einfacher Text, der so angezeigt wird, wie er dasteht.
- `pins` und `functions`: nur die der Art, jeweils höchstens einmal, in der Reihenfolge, in der der Tile Inspector sie zeigt.
- `default_macro` (optional): das Makro einer neuen Kachel, entweder `{ "chord": ["ControlLeft", "KeyC"] }` (gleichzeitig gedrückte Tasten, benannt wie im Makro-Editor) oder `{ "media": "mute" }` (`vol_up`, `vol_down`, `mute`, `next`, `prev`, `play`). Ohne Angabe beginnt die Funktion leer.
- `parameters` (optional): `type` ist `number` mit `min`, `max` und `default`, `choice` mit `options` und `default` oder `flag` mit `default`. Bereiche und Auswahlwerte müssen in dem bleiben, was die Firmware annimmt. Fehlende Parameter nutzen den Standardwert der Firmware.

Dateien, die diese Regeln nicht einhalten oder deren ID schon ein eingebautes Bauteil nutzt, werden übersprungen. Kacheln, deren Bauteil fehlt, behalten ihre Einstellungen, lassen sich aber erst wieder bearbeiten, wenn die Definition zurück ist.
