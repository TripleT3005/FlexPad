# <img src="assets/logo.svg" alt="" width="40" align="center"> FlexPad

[English](#english) · [Deutsch](#deutsch)

## English

FlexPad is a freely configurable macro pad based on the ESP32-S3. The board acts as a USB keyboard and media controller and runs key sequences when a button is pressed or an encoder is turned. A cross-platform desktop app written in Rust ([iced](https://iced.rs/)) lets you place buttons and rotary encoders on a canvas, assign GPIO pins and macros, and transfers the configuration to the device over USB serial.

> **Work in progress.** FlexPad is not usable yet. Known gaps:
> - **The firmware does not understand the app's current configuration format yet** ([#2](../../issues/2)). The app sends its whole profile (`profile_name`, `tiles[]` with `functions`), while the firmware still expects `{"pins": {"<pin>": "<sequence>"}}`. After uploading a profile from the app, all pins are treated as unassigned.
> - `TEXT:` actions, many key names offered by the app (umlauts, numpad, punctuation) and EC11 rotary encoders are not handled by the firmware yet.
> - The keyboard layout of the computer is not taken into account yet, so text only comes out right on a US layout ([#5](../../issues/5)).
>
> See the [issues](../../issues) for details and plans. The firmware is being rewritten in Rust ([#22](../../issues/22)); the first version in `flexpad-firmware/` answers the serial commands and runs macros sent with `RUN_MACRO` and stores the configuration from the app, but does not read buttons yet.
>
> Not affiliated with or endorsed by Espressif Systems or Arduino.

### Features
- Up to 30 inputs on one ESP32-S3: push buttons and EC11 rotary encoders
- Macros made of key presses and releases, delays, media keys and text
- Visual editor: place tiles on a canvas, assign pins and build macros from presets or step by step
- Profiles stored locally and on the device; the device works without the app once configured
- No driver, no account, no network access

### Hardware
| Part | Notes |
|---|---|
| ESP32-S3 board with native USB | Developed with an ESP32-S3 dev module with 16 MB flash and 8 MB octal PSRAM (N16R8) |
| Push buttons | One side to a GPIO pin, the other side to GND. Internal pull-ups are used, no resistors needed. |
| EC11 rotary encoder (optional) | Pins A and B to two GPIO pins, the middle pin C to GND. The push button of the encoder like a normal button. |

**Usable pins:** 1, 2, 4–18, 21, 33–42, 47, 48. GPIO 0, 3, 45 and 46 (strapping pins) and 19/20 (USB) are left out on purpose. On modules with octal PSRAM (such as N16R8), GPIO 35–37 are used internally and should not be wired; on modules with octal flash also 33 and 34 ([#14](../../issues/14)).

### Installation
There are no prebuilt releases yet. Build the desktop app and flash the firmware as described under [Building](#building).

### Usage
1. Connect the FlexPad via USB and start the app.
2. **Scan Devices**, select the device and **Connect**.
3. **Load Config** reads the configuration stored on the device.
4. Add tiles with **Add Tile** (button or EC11 encoder), drag them into place and select one to edit it in the **Tile Inspector**:
   - choose the GPIO pin(s) the tile is wired to,
   - build the macro from **Presets** or add actions (Press, Release, Delay, Media, Text) one by one. **Advanced View** shows the raw macro string.
5. **Save Profile** stores the profile on your computer, **Set Config** sends it to the device.

### How it works
The firmware polls the configured pins, debounces them and runs the assigned macro as USB HID keyboard and consumer control (media keys) reports. The configuration is kept as JSON in a LittleFS partition on the ESP32-S3, so it survives a restart.

The app and the device talk over the USB serial port of the board (115200 baud, line-based text commands, each terminated with `\n`):

| Command | Response |
|---|---|
| `GET_DEVICE_INFO` | `DEVICE_INFO:FlexPad,<serial number>` |
| `GET_CONFIG` | `CONFIG_DATA:<json>` |
| `SET_CONFIG:<json>` | `CONFIG_SAVED`; the Rust firmware checks the configuration first and answers `CONFIG_ERROR:<reason>` (e.g. `invalid_json:123`, `unusable_pin:35`, `duplicate_pin:4`) without changing the stored one |
| `RUN_MACRO:<macro>` | Checks the macro and runs it: `MACRO_QUEUED`, or `MACRO_ERROR:<reason>` (e.g. `unknown_key:hyper`, `busy`) (Rust firmware only) |
| `REBOOT_BOOTLOADER` | `REBOOTING:bootloader`, then the board restarts in download mode (Rust firmware only) |

Macros are comma-separated actions: `P:<key>` press, `R:<key>` release, `D:<ms>` delay, `MEDIA:<action>` media key (`vol_up`, `vol_down`, `mute`, `next`, `prev`, `play`), `TEXT:<text>` type text (app only so far). Example: `P:lctrl,P:c,D:50,R:c,R:lctrl` (copy).

Key names describe the physical key position on a US keyboard, using the names of [`KeyboardEvent.code`](https://developer.mozilla.org/en-US/docs/Web/API/UI_Events/Keyboard_event_code_values) (`KeyA`, `Digit1`, `ControlLeft`, `Semicolon`, `IntlBackslash`, `NumpadAdd`, `F13` …). On a German layout, `Semicolon` therefore types `ö` and `KeyZ` types `y`. The Rust firmware also accepts the older names used by the app so far (`lctrl`, `ael`, `kp_1` …). Macros run in the background: the device keeps answering serial commands, and all keys are released when a macro ends.

### Privacy
- **No network access.** Neither the app nor the firmware connects to the internet; nothing is downloaded or sent.
- The app only talks to serial ports on your computer. To find FlexPad devices it currently opens every serial port and sends `GET_DEVICE_INFO` ([#15](../../issues/15)).
- Profiles and settings are stored in the folder `FlexPad` in your home directory. The device stores only the configuration you send to it.
- The device identifies itself with a serial number derived from the chip's MAC address.

### Building
#### Desktop app
Requires [Rust](https://www.rust-lang.org/tools/install) 1.85 or newer (edition 2024). On Linux, the serial port library also needs `libudev` (e.g. `sudo apt install libudev-dev pkg-config`).

```
cd flexpad-desktop
cargo run --release
```

`cargo test` runs the unit tests; CI checks formatting, `clippy` and tests on Windows, Linux and macOS.

#### Firmware
Requires the [Arduino IDE](https://www.arduino.cc/en/software) or `arduino-cli` with the Espressif ESP32 core and the library [ArduinoJson](https://arduinojson.org/) (version 7).

1. Open `esp32-firmware/esp32-firmware.ino`. `partitions.csv` must stay in the same folder; it reserves two 6 MB app partitions and a 3 MB `storage` partition for the configuration.
2. Select these settings in the **Tools** menu:

| Setting | Value |
|---|---|
| Board | ESP32S3 Dev Module |
| USB CDC On Boot | Enabled |
| CPU Frequency | 240 MHz (WiFi) |
| Flash Mode | QIO 80 MHz |
| Flash Size | 16 MB (128 Mb) |
| Partition Scheme | Custom |
| PSRAM | OPI PSRAM |
| USB Mode | USB-OTG (TinyUSB) |
| Upload Mode | USB-OTG CDC (TinyUSB) |
| USB Firmware MSC On Boot / USB DFU On Boot / JTAG Adapter | Disabled |

3. Select the board's port and upload. Afterwards the board shows up as the USB device "FlexPad".

Flash size and PSRAM must match your module. If the upload does not start, hold **BOOT**, press **RESET** and release **BOOT** to enter download mode.

#### Rust firmware (in development)
The new firmware in `flexpad-firmware/` uses [esp-hal](https://github.com/esp-rs/esp-hal) and [Embassy](https://embassy.dev/). It needs the Xtensa Rust toolchain and `espflash`:

```
cargo install espup espflash --locked
espup install --targets esp32s3
```

Afterwards, open a new terminal (Linux and macOS: `. ~/export-esp.sh`) and build:

```
cd flexpad-firmware
cargo build --release
cargo +stable test --lib --target <host target>   # protocol tests on your computer, e.g. x86_64-pc-windows-msvc
```

To flash, the board must be in download mode. If the Rust firmware is already running, send `REBOOT_BOOTLOADER` over the serial port; otherwise hold **BOOT**, press **RESET** and release **BOOT**. Then flash to the new port:

```
espflash flash --no-stub --before no-reset --after watchdog-reset --partition-table partitions.csv --port <port> target/xtensa-esp32s3-none-elf/release/flexpad-firmware
```

After flashing, press **RESET** (or unplug the board briefly) to start the firmware. `espflash` may report an error for the final `FlashEnd` step; the firmware is written anyway. The configuration is stored in its own partition `flexpad` (64 KB, two slots with checksum).

### License
[MIT](LICENSE)

## Deutsch

FlexPad ist ein frei konfigurierbares Makropad auf Basis des ESP32-S3. Das Board meldet sich als USB-Tastatur und Mediensteuerung und führt Tastenfolgen aus, wenn ein Taster gedrückt oder ein Encoder gedreht wird. Eine plattformübergreifende Desktop-App in Rust ([iced](https://iced.rs/)) platziert Taster und Drehencoder auf einer Arbeitsfläche, ordnet GPIO-Pins und Makros zu und überträgt die Konfiguration per USB-Seriell auf das Gerät.

> **In Arbeit.** FlexPad ist noch nicht einsatzbereit. Bekannte Lücken:
> - **Die Firmware versteht das aktuelle Konfigurationsformat der App noch nicht** ([#2](../../issues/2)). Die App sendet ihr komplettes Profil (`profile_name`, `tiles[]` mit `functions`), die Firmware erwartet aber noch `{"pins": {"<pin>": "<sequenz>"}}`. Nach dem Übertragen eines Profils aus der App gelten alle Pins als unbelegt.
> - `TEXT:`-Aktionen, viele Tastennamen der App (Umlaute, Ziffernblock, Satzzeichen) und EC11-Drehencoder wertet die Firmware noch nicht aus.
> - Das Tastaturlayout des Rechners wird noch nicht berücksichtigt; Text kommt daher nur bei US-Layout richtig an ([#5](../../issues/5)).
>
> Details und Pläne stehen in den [Issues](../../issues). Die Firmware wird gerade in Rust neu geschrieben ([#22](../../issues/22)); die erste Version in `flexpad-firmware/` beantwortet die seriellen Befehle und führt mit `RUN_MACRO` gesendete Makros aus und speichert die Konfiguration aus der App, liest aber noch keine Taster.
>
> Keine Verbindung zu Espressif Systems oder Arduino und nicht von ihnen unterstützt.

### Funktionen
- Bis zu 30 Eingänge an einem ESP32-S3: Taster und EC11-Drehencoder
- Makros aus Drücken und Loslassen von Tasten, Pausen, Medientasten und Text
- Grafischer Editor: Kacheln auf einer Arbeitsfläche platzieren, Pins zuordnen und Makros aus Vorlagen oder Schritt für Schritt erstellen
- Profile lokal und auf dem Gerät gespeichert; einmal eingerichtet, arbeitet das Gerät ohne die App
- Kein Treiber, kein Konto, keine Netzwerkzugriffe

### Hardware
| Bauteil | Hinweise |
|---|---|
| ESP32-S3-Board mit nativem USB | Entwickelt mit einem ESP32-S3-Dev-Module mit 16 MB Flash und 8 MB Octal-PSRAM (N16R8) |
| Taster | Eine Seite an einen GPIO-Pin, die andere an GND. Die internen Pull-ups werden genutzt, Widerstände sind nicht nötig. |
| EC11-Drehencoder (optional) | Pins A und B an zwei GPIO-Pins, der mittlere Pin C an GND. Der Taster des Encoders wie ein normaler Taster. |

**Nutzbare Pins:** 1, 2, 4–18, 21, 33–42, 47, 48. GPIO 0, 3, 45 und 46 (Strapping-Pins) sowie 19/20 (USB) sind bewusst ausgenommen. Auf Modulen mit Octal-PSRAM (z. B. N16R8) sind GPIO 35–37 intern belegt und sollten nicht verdrahtet werden, bei Modulen mit Octal-Flash zusätzlich 33 und 34 ([#14](../../issues/14)).

### Installation
Fertige Releases gibt es noch nicht. Die Desktop-App bauen und die Firmware flashen wie unter [Bauen](#bauen) beschrieben.

### Bedienung
1. FlexPad per USB anschließen und die App starten.
2. **Scan Devices**, das Gerät auswählen und **Connect**.
3. **Load Config** liest die auf dem Gerät gespeicherte Konfiguration.
4. Mit **Add Tile** Kacheln hinzufügen (Taster oder EC11-Encoder), an die gewünschte Stelle ziehen und zum Bearbeiten im **Tile Inspector** auswählen:
   - den oder die GPIO-Pins wählen, an denen die Kachel angeschlossen ist,
   - das Makro aus **Presets** übernehmen oder Aktionen (Press, Release, Delay, Media, Text) einzeln hinzufügen. **Advanced View** zeigt den rohen Makro-String.
5. **Save Profile** speichert das Profil auf dem Rechner, **Set Config** überträgt es auf das Gerät.

### Funktionsweise
Die Firmware fragt die konfigurierten Pins ab, entprellt sie und führt das zugeordnete Makro als USB-HID-Tastatur- und Mediensteuerungs-Reports aus. Die Konfiguration liegt als JSON in einer LittleFS-Partition auf dem ESP32-S3 und bleibt so über einen Neustart erhalten.

App und Gerät sprechen über den USB-Seriell-Port des Boards miteinander (115200 Baud, zeilenbasierte Textbefehle, jeweils mit `\n` abgeschlossen):

| Befehl | Antwort |
|---|---|
| `GET_DEVICE_INFO` | `DEVICE_INFO:FlexPad,<Seriennummer>` |
| `GET_CONFIG` | `CONFIG_DATA:<json>` |
| `SET_CONFIG:<json>` | `CONFIG_SAVED`; die Rust-Firmware prüft die Konfiguration zuerst und antwortet `CONFIG_ERROR:<grund>` (z. B. `invalid_json:123`, `unusable_pin:35`, `duplicate_pin:4`), ohne die gespeicherte zu ändern |
| `RUN_MACRO:<makro>` | Prüft das Makro und führt es aus: `MACRO_QUEUED` oder `MACRO_ERROR:<grund>` (z. B. `unknown_key:hyper`, `busy`) (nur Rust-Firmware) |
| `REBOOT_BOOTLOADER` | `REBOOTING:bootloader`, danach startet das Board im Download-Modus neu (nur Rust-Firmware) |

Makros sind kommagetrennte Aktionen: `P:<taste>` drücken, `R:<taste>` loslassen, `D:<ms>` warten, `MEDIA:<aktion>` Medientaste (`vol_up`, `vol_down`, `mute`, `next`, `prev`, `play`), `TEXT:<text>` Text tippen (bisher nur in der App). Beispiel: `P:lctrl,P:c,D:50,R:c,R:lctrl` (Kopieren).

Tastennamen beschreiben die physische Tastenposition auf einer US-Tastatur, mit den Namen von [`KeyboardEvent.code`](https://developer.mozilla.org/en-US/docs/Web/API/UI_Events/Keyboard_event_code_values) (`KeyA`, `Digit1`, `ControlLeft`, `Semicolon`, `IntlBackslash`, `NumpadAdd`, `F13` …). Bei deutschem Layout tippt `Semicolon` daher `ö` und `KeyZ` ein `y`. Die Rust-Firmware versteht auch die bisherigen Namen der App (`lctrl`, `ael`, `kp_1` …). Makros laufen im Hintergrund: Das Gerät beantwortet weiter serielle Befehle, und am Ende eines Makros werden alle Tasten losgelassen.

### Datenschutz
- **Keine Netzwerkzugriffe.** Weder die App noch die Firmware verbinden sich mit dem Internet; es wird nichts abgerufen oder gesendet.
- Die App spricht nur mit seriellen Ports am eigenen Rechner. Um FlexPad-Geräte zu finden, öffnet sie derzeit jeden seriellen Port und sendet `GET_DEVICE_INFO` ([#15](../../issues/15)).
- Profile und Einstellungen liegen im Ordner `FlexPad` im Benutzerverzeichnis. Das Gerät speichert nur die Konfiguration, die man ihm schickt.
- Das Gerät meldet sich mit einer Seriennummer, die aus der MAC-Adresse des Chips gebildet wird.

### Bauen
#### Desktop-App
Voraussetzung ist [Rust](https://www.rust-lang.org/tools/install) 1.85 oder neuer (Edition 2024). Unter Linux braucht die Seriell-Bibliothek zusätzlich `libudev` (z. B. `sudo apt install libudev-dev pkg-config`).

```
cd flexpad-desktop
cargo run --release
```

`cargo test` führt die Unit-Tests aus; die CI prüft Formatierung, `clippy` und Tests unter Windows, Linux und macOS.

#### Firmware
Voraussetzung ist die [Arduino IDE](https://www.arduino.cc/en/software) oder `arduino-cli` mit dem ESP32-Core von Espressif und der Bibliothek [ArduinoJson](https://arduinojson.org/) (Version 7).

1. `esp32-firmware/esp32-firmware.ino` öffnen. `partitions.csv` muss im selben Ordner liegen; sie reserviert zwei App-Partitionen mit je 6 MB und eine 3 MB große `storage`-Partition für die Konfiguration.
2. Im Menü **Werkzeuge** diese Einstellungen wählen:

| Einstellung | Wert |
|---|---|
| Board | ESP32S3 Dev Module |
| USB CDC On Boot | Enabled |
| CPU Frequency | 240 MHz (WiFi) |
| Flash Mode | QIO 80 MHz |
| Flash Size | 16 MB (128 Mb) |
| Partition Scheme | Custom |
| PSRAM | OPI PSRAM |
| USB Mode | USB-OTG (TinyUSB) |
| Upload Mode | USB-OTG CDC (TinyUSB) |
| USB Firmware MSC On Boot / USB DFU On Boot / JTAG Adapter | Disabled |

3. Den Port des Boards wählen und hochladen. Danach meldet sich das Board als USB-Gerät „FlexPad“.

Flash-Größe und PSRAM müssen zum eigenen Modul passen. Startet der Upload nicht, **BOOT** gedrückt halten, **RESET** drücken und **BOOT** loslassen, um in den Download-Modus zu wechseln.

#### Rust-Firmware (in Entwicklung)
Die neue Firmware in `flexpad-firmware/` nutzt [esp-hal](https://github.com/esp-rs/esp-hal) und [Embassy](https://embassy.dev/). Sie braucht die Xtensa-Rust-Toolchain und `espflash`:

```
cargo install espup espflash --locked
espup install --targets esp32s3
```

Danach ein neues Terminal öffnen (Linux und macOS: `. ~/export-esp.sh`) und bauen:

```
cd flexpad-firmware
cargo build --release
cargo +stable test --lib --target <host-target>   # Protokoll-Tests auf dem Rechner, z. B. x86_64-pc-windows-msvc
```

Zum Flashen muss das Board im Download-Modus sein. Läuft bereits die Rust-Firmware, über den seriellen Port `REBOOT_BOOTLOADER` senden; sonst **BOOT** gedrückt halten, **RESET** drücken und **BOOT** loslassen. Dann auf den neuen Port flashen:

```
espflash flash --no-stub --before no-reset --after watchdog-reset --partition-table partitions.csv --port <port> target/xtensa-esp32s3-none-elf/release/flexpad-firmware
```

Nach dem Flashen **RESET** drücken (oder das Board kurz abziehen), damit die Firmware startet. `espflash` meldet beim letzten Schritt `FlashEnd` eventuell einen Fehler; die Firmware ist trotzdem geschrieben. Die Konfiguration liegt in einer eigenen Partition `flexpad` (64 KB, zwei Speicherplätze mit Prüfsumme).

### Lizenz
[MIT](LICENSE)
