# FlexPad

[English](#english) · [Deutsch](#deutsch)

## English

FlexPad is a freely configurable macro pad based on the ESP32-S3. The board acts as a USB keyboard and media controller and runs key sequences when a button is pressed. A cross-platform desktop app written in Rust ([iced](https://iced.rs/)) lets you place buttons and rotary encoders on a canvas, assign GPIO pins and macros, and transfers the configuration to the device over USB serial.

> **Work in progress.** FlexPad is not usable yet. Development is paused until the test hardware is available. Known gaps:
> - **The firmware does not understand the app's current configuration format yet.** The app sends its whole profile (`profile_name`, `tiles[]` with `functions`), while the firmware still expects `{"pins": {"<pin>": "<sequence>"}}`. After uploading a profile from the app, all pins are treated as unassigned.
> - `TEXT:` actions, many key names offered by the app (umlauts, numpad, punctuation) and EC11 rotary encoders are not handled by the firmware yet.
> - The device always reports a US keyboard layout. Text on other layouts is only partly mapped (German only).
>
> See the [issues](../../issues) for details and plans.

### Repository layout
| Folder | Content |
|---|---|
| `esp32-firmware/` | Arduino sketch for the ESP32-S3 (USB HID keyboard and consumer control, configuration stored in LittleFS) and the partition table `partitions.csv` |
| `flexpad-desktop/` | Desktop configurator in Rust (iced, serialport, serde) |

### Hardware
- ESP32-S3 board with native USB, developed with an ESP32-S3 dev module with 16 MB flash and octal PSRAM
- Buttons between a GPIO pin and GND (internal pull-ups are used)
- Monitored pins: 1, 2, 4–18, 21, 33–42, 47, 48. On modules with octal PSRAM or octal flash, some of these pins are used internally (see the issues).

### Building and flashing the firmware
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

Flash size and PSRAM must match your module.

### Building the desktop app
Requires [Rust](https://www.rust-lang.org/tools/install) 1.85 or newer (edition 2024). On Linux, the serial port library also needs `libudev` (e.g. `sudo apt install libudev-dev pkg-config`).

```
cd flexpad-desktop
cargo run --release
```

Profiles and settings are stored in the folder `FlexPad` in your home directory.

### Serial protocol
Line-based text commands at 115200 baud, each terminated with `\n`:

| Command | Response |
|---|---|
| `GET_DEVICE_INFO` | `DEVICE_INFO:FlexPad,<serial number>` |
| `GET_CONFIG` | `CONFIG_DATA:<json>` |
| `SET_CONFIG:<json>` | `CONFIG_SAVED` |

Macros are comma-separated actions: `P:<key>` press, `R:<key>` release, `D:<ms>` delay, `MEDIA:<action>` media key (`vol_up`, `vol_down`, `mute`, `next`, `prev`, `play`), `TEXT:<text>` type text (app only so far). Example: `P:lctrl,P:c,D:50,R:c,R:lctrl` (copy).

### License
[MIT](LICENSE)

## Deutsch

FlexPad ist ein frei konfigurierbares Makropad auf Basis des ESP32-S3. Das Board meldet sich als USB-Tastatur und Mediensteuerung und führt beim Tastendruck Tastenfolgen aus. Eine plattformübergreifende Desktop-App in Rust ([iced](https://iced.rs/)) platziert Taster und Drehencoder auf einer Arbeitsfläche, ordnet GPIO-Pins und Makros zu und überträgt die Konfiguration per USB-Seriell auf das Gerät.

> **In Arbeit.** FlexPad ist noch nicht einsatzbereit. Die Entwicklung ruht, bis die Test-Hardware da ist. Bekannte Lücken:
> - **Die Firmware versteht das aktuelle Konfigurationsformat der App noch nicht.** Die App sendet ihr komplettes Profil (`profile_name`, `tiles[]` mit `functions`), die Firmware erwartet aber noch `{"pins": {"<pin>": "<sequenz>"}}`. Nach dem Übertragen eines Profils aus der App gelten alle Pins als unbelegt.
> - `TEXT:`-Aktionen, viele Tastennamen der App (Umlaute, Ziffernblock, Satzzeichen) und EC11-Drehencoder wertet die Firmware noch nicht aus.
> - Das Gerät meldet sich immer mit US-Tastaturlayout. Text für andere Layouts wird nur teilweise umgesetzt (nur Deutsch).
>
> Details und Pläne stehen in den [Issues](../../issues).

### Aufbau
| Ordner | Inhalt |
|---|---|
| `esp32-firmware/` | Arduino-Sketch für den ESP32-S3 (USB-HID-Tastatur und Mediensteuerung, Konfiguration in LittleFS) und die Partitionstabelle `partitions.csv` |
| `flexpad-desktop/` | Desktop-Konfigurator in Rust (iced, serialport, serde) |

### Hardware
- ESP32-S3-Board mit nativem USB, entwickelt mit einem ESP32-S3-Dev-Module mit 16 MB Flash und Octal-PSRAM
- Taster zwischen einem GPIO-Pin und GND (die internen Pull-ups werden genutzt)
- Überwachte Pins: 1, 2, 4–18, 21, 33–42, 47, 48. Auf Modulen mit Octal-PSRAM oder Octal-Flash sind einige davon intern belegt (siehe Issues).

### Firmware bauen und flashen
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

Flash-Größe und PSRAM müssen zum eigenen Modul passen.

### Desktop-App bauen
Voraussetzung ist [Rust](https://www.rust-lang.org/tools/install) 1.85 oder neuer (Edition 2024). Unter Linux braucht die Seriell-Bibliothek zusätzlich `libudev` (z. B. `sudo apt install libudev-dev pkg-config`).

```
cd flexpad-desktop
cargo run --release
```

Profile und Einstellungen liegen im Ordner `FlexPad` im Benutzerverzeichnis.

### Serielles Protokoll
Zeilenbasierte Textbefehle mit 115200 Baud, jeweils mit `\n` abgeschlossen:

| Befehl | Antwort |
|---|---|
| `GET_DEVICE_INFO` | `DEVICE_INFO:FlexPad,<Seriennummer>` |
| `GET_CONFIG` | `CONFIG_DATA:<json>` |
| `SET_CONFIG:<json>` | `CONFIG_SAVED` |

Makros sind kommagetrennte Aktionen: `P:<taste>` drücken, `R:<taste>` loslassen, `D:<ms>` warten, `MEDIA:<aktion>` Medientaste (`vol_up`, `vol_down`, `mute`, `next`, `prev`, `play`), `TEXT:<text>` Text tippen (bisher nur in der App). Beispiel: `P:lctrl,P:c,D:50,R:c,R:lctrl` (Kopieren).

### Lizenz
[MIT](LICENSE)
