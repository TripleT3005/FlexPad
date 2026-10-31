# <img src="assets/logo.svg" alt="" width="40" align="center"> FlexPad

[English](#english) · [Deutsch](#deutsch)

## English

FlexPad is a freely configurable macro pad based on the ESP32-S3. The board acts as a USB keyboard and media controller and runs key sequences when a button is pressed or an encoder is turned. A cross-platform desktop app written in Rust ([iced](https://iced.rs/)) lets you place buttons and rotary encoders on a canvas, assign GPIO pins and macros, and transfers the configuration to the device over USB serial. The firmware is written in Rust as well ([esp-hal](https://github.com/esp-rs/esp-hal), [Embassy](https://embassy.dev/)).

> **Work in progress.** Buttons and EC11 encoders work, but FlexPad is not finished yet. Known gaps:
> - `TEXT:` actions are not typed yet, and commas inside text break the macro ([#6](../../issues/6)).
> - The keyboard layout of the computer is not taken into account yet, so text only comes out right on a US layout ([#5](../../issues/5)).
>
> See the [issues](../../issues) for details and plans.
>
> Not affiliated with or endorsed by Espressif Systems.

### Features
- Up to 25 inputs on one ESP32-S3: push buttons and EC11 rotary encoders
- Macros made of key presses and releases, delays, media keys and text
- Visual editor: place tiles on a canvas, assign pins and build macros from presets or step by step
- Debounce time, encoder resolution and direction adjustable per tile
- Profiles stored locally and on the device; the device works without the app once configured
- No driver, no account, no network access

### Hardware
| Part | Notes |
|---|---|
| ESP32-S3 board with native USB | Developed with an ESP32-S3 dev module with 16 MB flash and 8 MB octal PSRAM (N16R8) |
| Push buttons | One side to a GPIO pin, the other side to GND. Internal pull-ups are used, no resistors needed. |
| EC11 rotary encoder (optional) | Pins A and B to two GPIO pins, the middle pin C to GND. The push button of the encoder like a normal button. |

**Usable pins:** 1, 2, 4–18, 21, 38–42, 47, 48 (25 inputs). Left out on purpose: GPIO 0, 3, 45 and 46 (strapping pins), 19/20 (USB) and 33–37, which modules with octal PSRAM or octal flash (such as N16R8) use internally.

### Installation
Download the app and the firmware from [Releases](../../releases).

#### App
**Windows:** run `FlexPad-<version>-windows-x64-setup.exe`. It installs for the current user without administrator rights (to `%LOCALAPPDATA%\Programs\FlexPad`); "Install for all users" installs to `C:\Program Files\FlexPad` and asks for administrator rights. Options: Start menu entry (on), desktop shortcut (off). Instead of the installer you can use the portable archive `…-windows-x64-portable.zip`.

**macOS:** open `…-macos-arm64.dmg` (Apple silicon) and drag FlexPad into the Applications folder.

**Linux:** on Debian, Ubuntu, Linux Mint and similar, install `flexpad_<version>_amd64.deb` (e.g. `sudo apt install ./flexpad_0.1.0_amd64.deb`); on Fedora, openSUSE and similar, install `flexpad-<version>-1.x86_64.rpm` (e.g. `sudo dnf install ./flexpad-0.1.0-1.x86_64.rpm`). Both add FlexPad to the application menu. For other distributions, extract `…-linux-x64.tar.gz` and start `flexpad`; it needs `libudev`. To access the USB serial port, your user must be in the group `dialout` (Debian, Ubuntu, Fedora) or `uucp` (Arch): `sudo usermod -aG dialout $USER`, then log in again.

The app is not signed, so the system may warn on the first start:
- **Windows:** SmartScreen → "More info" → "Run anyway"
- **macOS:** the app is not notarized by Apple. On the first start macOS refuses to open it; then choose System Settings → Privacy & Security → "Open Anyway" (on older versions: right-click the app → "Open"). This is needed only once.

Each download has a `.sha256` file to verify it.

#### Installer parameters
| Parameter | Effect |
|---|---|
| `/CURRENTUSER` | Install for the current user only (default) |
| `/ALLUSERS` | Install for all users (asks for administrator rights) |
| `/DIR="C:\Path"` | Installation folder |
| `/SILENT`, `/VERYSILENT` | Without wizard (with or without progress window) |
| `/SUPPRESSMSGBOXES` | No message boxes in silent mode |
| `/TASKS="desktopicon"` | Add a desktop shortcut; without the parameter there is none |
| `/NOICONS` | No Start menu entry |
| `/LOG="C:\Path\setup.log"` | Write a log file |

Example: `FlexPad-0.1.0-windows-x64-setup.exe /VERYSILENT /SUPPRESSMSGBOXES /TASKS="desktopicon"`

Profiles and settings are stored per user and are not set by the installer. Uninstalling removes the program and its shortcuts but keeps your profiles in the folder `FlexPad` in your home directory.

#### Firmware
`FlexPad-firmware-<version>-esp32s3.bin` contains bootloader, partition table and firmware in one image for an ESP32-S3 with at least 8 MB flash. It is written with [espflash](https://github.com/esp-rs/espflash) (prebuilt downloads on its releases page, or `cargo install espflash`):

1. Put the board into download mode: hold **BOOT**, press **RESET**, release **BOOT**. If the FlexPad firmware is already running, sending `REBOOT_BOOTLOADER` over its serial port does the same.
2. `espflash write-bin --chip esp32s3 --no-stub --before no-reset --after no-reset --port <port> 0x0 FlexPad-firmware-<version>-esp32s3.bin`
3. Press **RESET** (or unplug the board briefly). The board then shows up as "FlexPad".

A profile already stored on the device is kept: the image ends before the profile partition.

### Usage
1. Connect the FlexPad via USB and start the app.
2. **Scan Devices**, select the device and **Connect**.
3. **Load Config** reads the configuration stored on the device.
4. Add tiles with **Add Tile** (button or EC11 encoder), drag them into place and select one to edit it in the **Tile Inspector**:
   - **Pins**: choose the GPIO pin for each connection of the component (only free, usable pins are offered),
   - **Parameters**: debounce time, and for encoders the steps per detent and the direction,
   - **Functions** (e.g. press, turn right, turn left): open one to edit its macro,
   - in a function, build the macro from **Presets** or add actions (Press, Release, Delay, Media, Text) one by one. **Advanced View** shows the raw macro string.
5. **Save Profile** stores the profile on your computer, **Set Config** sends it to the device.

### How it works
The firmware polls the configured pins every millisecond, debounces buttons, decodes encoders and runs the assigned macro as USB HID keyboard and consumer control (media keys) reports. The profile is stored as JSON in its own flash partition with two slots and a checksum, so it survives a restart and an interrupted save.

The app and the device talk over the USB serial port of the board (line-based text commands, each terminated with `\n`). The complete protocol and profile format are described in [docs/protocol.md](docs/protocol.md); overview:

| Command | Response |
|---|---|
| `GET_DEVICE_INFO` | `DEVICE_INFO:FlexPad,<serial number>,<firmware version>` |
| `GET_CONFIG` | `CONFIG_DATA:<profile JSON>` |
| `SET_CONFIG:<profile JSON>` | `CONFIG_SAVED`, or `CONFIG_ERROR:<reason>` (e.g. `unusable_pin:35`, `duplicate_pin:4`) without changing the stored profile |
| `RUN_MACRO:<macro>` | Runs a macro for testing: `MACRO_QUEUED`, or `MACRO_ERROR:<reason>` |
| `REBOOT_BOOTLOADER` | `REBOOTING:bootloader`, then the board restarts in download mode for flashing |
| – | Sent by the device: `EVENT:PIN:<pin>:PRESSED`/`RELEASED` and `EVENT:ENCODER:<pin a>:RIGHT`/`LEFT` |

Macros are comma-separated actions: `P:<key>` press, `R:<key>` release, `D:<ms>` delay, `MEDIA:<action>` media key, `TEXT:<text>` type text (not executed yet). Example: `P:ControlLeft,P:KeyC,D:50,R:KeyC,R:ControlLeft` (copy).

Key names describe the physical key position on a US keyboard, using the names of [`KeyboardEvent.code`](https://developer.mozilla.org/en-US/docs/Web/API/UI_Events/Keyboard_event_code_values) (`KeyA`, `Digit1`, `ControlLeft`, `Semicolon`, `IntlBackslash`, `NumpadAdd`, `F13` …). On a German layout, `Semicolon` therefore types `ö` and `KeyZ` types `y`. The firmware also accepts the older names the app still uses (`lctrl`, `ael`, `kp_1` …). Macros run in the background: the device keeps answering serial commands, and all keys are released when a macro ends.

### Privacy
- **No network access.** Neither the app nor the firmware connects to the internet; nothing is downloaded or sent.
- The app only talks to serial ports on your computer. To find FlexPad devices it only opens USB serial ports whose USB serial number starts with `FlexPad-` (Espressif vendor ID `303A`) and asks them with `GET_DEVICE_INFO`; other devices are not touched.
- Profiles and settings are stored in the folder `FlexPad` in your home directory. The device stores the complete profile you send to it, so **Load Config** shows the same profile on any computer. Older profiles are converted to the current format when loaded.
- The device identifies itself with a serial number derived from the chip's MAC address.

### Building
#### Desktop app
Requires [Rust](https://www.rust-lang.org/tools/install) 1.88 or newer (edition 2024). On Linux, the serial port library also needs `libudev` (e.g. `sudo apt install libudev-dev pkg-config`).

```
cd flexpad-desktop
cargo run --release
```

`cargo test` runs the unit tests; CI checks formatting, `clippy` and tests on Windows, Linux and macOS.

#### Firmware
The firmware in `flexpad-firmware/` needs the Xtensa Rust toolchain and `espflash`:

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

To flash, the board must be in download mode. If the FlexPad firmware is already running, send `REBOOT_BOOTLOADER` over the serial port; otherwise hold **BOOT**, press **RESET** and release **BOOT**. Then flash to the new port:

```
espflash flash --no-stub --before no-reset --after watchdog-reset --partition-table partitions.csv --port <port> target/xtensa-esp32s3-none-elf/release/flexpad-firmware
```

After flashing, press **RESET** (or unplug the board briefly) to start the firmware. `espflash` may report an error for the final `FlashEnd` step; the firmware is written anyway. The profile is stored in its own partition `flexpad` (64 KB) and is kept when a new firmware is flashed.

#### Releases
A new version in `flexpad-desktop/Cargo.toml` and `flexpad-firmware/Cargo.toml` (a test and the release workflow check that both and the newest changelog section match) together with a section for it in [CHANGELOG.md](CHANGELOG.md) creates a release when merged into `main`; the changelog section becomes the release notes. Changes without a new version are collected under `[Unreleased]`; a release fails while that section still has entries, so they have to be moved into the section of the new version. Versions `0.x` and versions with a suffix like `1.1.0-beta.1` are published as pre-releases. The repository secret `VT_API_KEY` (VirusTotal) is optional.

### License
[MIT](LICENSE)

## Deutsch

FlexPad ist ein frei konfigurierbares Makropad auf Basis des ESP32-S3. Das Board meldet sich als USB-Tastatur und Mediensteuerung und führt Tastenfolgen aus, wenn ein Taster gedrückt oder ein Encoder gedreht wird. Eine plattformübergreifende Desktop-App in Rust ([iced](https://iced.rs/)) platziert Taster und Drehencoder auf einer Arbeitsfläche, ordnet GPIO-Pins und Makros zu und überträgt die Konfiguration per USB-Seriell auf das Gerät. Auch die Firmware ist in Rust geschrieben ([esp-hal](https://github.com/esp-rs/esp-hal), [Embassy](https://embassy.dev/)).

> **In Arbeit.** Taster und EC11-Encoder funktionieren, FlexPad ist aber noch nicht fertig. Bekannte Lücken:
> - `TEXT:`-Aktionen werden noch nicht getippt, und Kommas im Text zerstückeln das Makro ([#6](../../issues/6)).
> - Das Tastaturlayout des Rechners wird noch nicht berücksichtigt; Text kommt daher nur bei US-Layout richtig an ([#5](../../issues/5)).
>
> Details und Pläne stehen in den [Issues](../../issues).
>
> Keine Verbindung zu Espressif Systems und nicht von Espressif unterstützt.

### Funktionen
- Bis zu 25 Eingänge an einem ESP32-S3: Taster und EC11-Drehencoder
- Makros aus Drücken und Loslassen von Tasten, Pausen, Medientasten und Text
- Grafischer Editor: Kacheln auf einer Arbeitsfläche platzieren, Pins zuordnen und Makros aus Vorlagen oder Schritt für Schritt erstellen
- Entprellzeit, Encoder-Auflösung und Drehrichtung pro Kachel einstellbar
- Profile lokal und auf dem Gerät gespeichert; einmal eingerichtet, arbeitet das Gerät ohne die App
- Kein Treiber, kein Konto, keine Netzwerkzugriffe

### Hardware
| Bauteil | Hinweise |
|---|---|
| ESP32-S3-Board mit nativem USB | Entwickelt mit einem ESP32-S3-Dev-Module mit 16 MB Flash und 8 MB Octal-PSRAM (N16R8) |
| Taster | Eine Seite an einen GPIO-Pin, die andere an GND. Die internen Pull-ups werden genutzt, Widerstände sind nicht nötig. |
| EC11-Drehencoder (optional) | Pins A und B an zwei GPIO-Pins, der mittlere Pin C an GND. Der Taster des Encoders wie ein normaler Taster. |

**Nutzbare Pins:** 1, 2, 4–18, 21, 38–42, 47, 48 (25 Eingänge). Bewusst ausgenommen: GPIO 0, 3, 45 und 46 (Strapping-Pins), 19/20 (USB) und 33–37, die Module mit Octal-PSRAM oder Octal-Flash (z. B. N16R8) intern nutzen.

### Installation
App und Firmware gibt es unter [Releases](../../releases).

#### App
**Windows:** `FlexPad-<Version>-windows-x64-setup.exe` ausführen. Es installiert für den aktuellen Benutzer ohne Administratorrechte (nach `%LOCALAPPDATA%\Programs\FlexPad`); „Für alle Benutzer installieren“ installiert nach `C:\Program Files\FlexPad` und fragt nach Administratorrechten. Optionen: Startmenü-Eintrag (an), Desktop-Verknüpfung (aus). Statt des Installers geht auch das portable Archiv `…-windows-x64-portable.zip`.

**macOS:** `…-macos-arm64.dmg` (Apple Silicon) öffnen und FlexPad in den Ordner Programme ziehen.

**Linux:** Unter Debian, Ubuntu, Linux Mint und ähnlichen `flexpad_<Version>_amd64.deb` installieren (z. B. `sudo apt install ./flexpad_0.1.0_amd64.deb`), unter Fedora, openSUSE und ähnlichen `flexpad-<Version>-1.x86_64.rpm` (z. B. `sudo dnf install ./flexpad-0.1.0-1.x86_64.rpm`). Beide tragen FlexPad ins Anwendungsmenü ein. Für andere Distributionen `…-linux-x64.tar.gz` entpacken und `flexpad` starten; es braucht `libudev`. Für den Zugriff auf den USB-Seriell-Port muss der eigene Benutzer in der Gruppe `dialout` (Debian, Ubuntu, Fedora) bzw. `uucp` (Arch) sein: `sudo usermod -aG dialout $USER`, danach neu anmelden.

Die Anwendung ist nicht signiert, deshalb warnt das System beim ersten Start eventuell:
- **Windows:** SmartScreen → „Weitere Informationen“ → „Trotzdem ausführen“
- **macOS:** Die App ist nicht von Apple beglaubigt. Beim ersten Start verweigert macOS das Öffnen; dann unter Systemeinstellungen → Datenschutz & Sicherheit „Dennoch öffnen“ wählen (bei älteren Versionen: Rechtsklick auf die App → „Öffnen“). Das ist nur einmal nötig.

Zu jedem Download gibt es eine `.sha256`-Datei, um ihn zu prüfen.

#### Installer-Parameter
| Parameter | Wirkung |
|---|---|
| `/CURRENTUSER` | Nur für den aktuellen Benutzer installieren (Standard) |
| `/ALLUSERS` | Für alle Benutzer installieren (fragt nach Administratorrechten) |
| `/DIR="C:\Pfad"` | Installationsordner |
| `/SILENT`, `/VERYSILENT` | Ohne Assistent (mit oder ohne Fortschrittsfenster) |
| `/SUPPRESSMSGBOXES` | Keine Meldungsfenster bei stiller Installation |
| `/TASKS="desktopicon"` | Desktop-Verknüpfung anlegen; ohne den Parameter gibt es keine |
| `/NOICONS` | Kein Startmenü-Eintrag |
| `/LOG="C:\Pfad\setup.log"` | Protokolldatei schreiben |

Beispiel: `FlexPad-0.1.0-windows-x64-setup.exe /VERYSILENT /SUPPRESSMSGBOXES /TASKS="desktopicon"`

Profile und Einstellungen gelten pro Benutzer und werden vom Installer nicht gesetzt. Die Deinstallation entfernt Programm und Verknüpfungen, lässt die Profile im Ordner `FlexPad` im Benutzerverzeichnis aber stehen.

#### Firmware
`FlexPad-firmware-<Version>-esp32s3.bin` enthält Bootloader, Partitionstabelle und Firmware in einem Image für einen ESP32-S3 mit mindestens 8 MB Flash. Geschrieben wird es mit [espflash](https://github.com/esp-rs/espflash) (fertige Downloads auf dessen Release-Seite oder `cargo install espflash`):

1. Das Board in den Download-Modus bringen: **BOOT** gedrückt halten, **RESET** drücken, **BOOT** loslassen. Läuft bereits die FlexPad-Firmware, geht das auch, indem man über ihren seriellen Port `REBOOT_BOOTLOADER` sendet.
2. `espflash write-bin --chip esp32s3 --no-stub --before no-reset --after no-reset --port <port> 0x0 FlexPad-firmware-<Version>-esp32s3.bin`
3. **RESET** drücken (oder das Board kurz abziehen). Danach meldet sich das Board als „FlexPad“.

Ein bereits gespeichertes Profil bleibt erhalten: Das Image endet vor der Profil-Partition.

### Bedienung
1. FlexPad per USB anschließen und die App starten.
2. **Scan Devices**, das Gerät auswählen und **Connect**.
3. **Load Config** liest die auf dem Gerät gespeicherte Konfiguration.
4. Mit **Add Tile** Kacheln hinzufügen (Taster oder EC11-Encoder), an die gewünschte Stelle ziehen und zum Bearbeiten im **Tile Inspector** auswählen:
   - **Pins**: für jeden Anschluss des Bauteils den GPIO-Pin wählen (angeboten werden nur freie, nutzbare Pins),
   - **Parameter**: Entprellzeit, bei Encodern zusätzlich Schritte pro Raste und Drehrichtung,
   - **Funktionen** (z. B. Drücken, Rechts drehen, Links drehen): aufklappen, um das Makro zu bearbeiten,
   - in einer Funktion das Makro aus **Presets** übernehmen oder Aktionen (Press, Release, Delay, Media, Text) einzeln hinzufügen. **Advanced View** zeigt den rohen Makro-String.
5. **Save Profile** speichert das Profil auf dem Rechner, **Set Config** überträgt es auf das Gerät.

### Funktionsweise
Die Firmware fragt die konfigurierten Pins jede Millisekunde ab, entprellt Taster, wertet Encoder aus und führt das zugeordnete Makro als USB-HID-Tastatur- und Mediensteuerungs-Reports aus. Das Profil liegt als JSON in einer eigenen Flash-Partition mit zwei Speicherplätzen und Prüfsumme; es übersteht so einen Neustart und ein unterbrochenes Speichern.

App und Gerät sprechen über den USB-Seriell-Port des Boards miteinander (zeilenbasierte Textbefehle, jeweils mit `\n` abgeschlossen). Das vollständige Protokoll und das Profilformat beschreibt [docs/protocol.md](docs/protocol.md) (auf Englisch); Überblick:

| Befehl | Antwort |
|---|---|
| `GET_DEVICE_INFO` | `DEVICE_INFO:FlexPad,<Seriennummer>,<Firmware-Version>` |
| `GET_CONFIG` | `CONFIG_DATA:<Profil-JSON>` |
| `SET_CONFIG:<Profil-JSON>` | `CONFIG_SAVED` oder `CONFIG_ERROR:<grund>` (z. B. `unusable_pin:35`, `duplicate_pin:4`), ohne das gespeicherte Profil zu ändern |
| `RUN_MACRO:<makro>` | Führt ein Makro zum Testen aus: `MACRO_QUEUED` oder `MACRO_ERROR:<grund>` |
| `REBOOT_BOOTLOADER` | `REBOOTING:bootloader`, danach startet das Board zum Flashen im Download-Modus neu |
| – | Vom Gerät gesendet: `EVENT:PIN:<pin>:PRESSED`/`RELEASED` und `EVENT:ENCODER:<pin a>:RIGHT`/`LEFT` |

Makros sind kommagetrennte Aktionen: `P:<taste>` drücken, `R:<taste>` loslassen, `D:<ms>` warten, `MEDIA:<aktion>` Medientaste, `TEXT:<text>` Text tippen (wird noch nicht ausgeführt). Beispiel: `P:ControlLeft,P:KeyC,D:50,R:KeyC,R:ControlLeft` (Kopieren).

Tastennamen beschreiben die physische Tastenposition auf einer US-Tastatur, mit den Namen von [`KeyboardEvent.code`](https://developer.mozilla.org/en-US/docs/Web/API/UI_Events/Keyboard_event_code_values) (`KeyA`, `Digit1`, `ControlLeft`, `Semicolon`, `IntlBackslash`, `NumpadAdd`, `F13` …). Bei deutschem Layout tippt `Semicolon` daher `ö` und `KeyZ` ein `y`. Die Firmware versteht auch die älteren Namen, die die App noch verwendet (`lctrl`, `ael`, `kp_1` …). Makros laufen im Hintergrund: Das Gerät beantwortet weiter serielle Befehle, und am Ende eines Makros werden alle Tasten losgelassen.

### Datenschutz
- **Keine Netzwerkzugriffe.** Weder die App noch die Firmware verbinden sich mit dem Internet; es wird nichts abgerufen oder gesendet.
- Die App spricht nur mit seriellen Ports am eigenen Rechner. Um FlexPad-Geräte zu finden, öffnet sie nur USB-Seriell-Ports, deren USB-Seriennummer mit `FlexPad-` beginnt (Espressif-Hersteller-ID `303A`), und fragt sie mit `GET_DEVICE_INFO` ab; andere Geräte bleiben unberührt.
- Profile und Einstellungen liegen im Ordner `FlexPad` im Benutzerverzeichnis. Das Gerät speichert das komplette Profil, das man ihm schickt; **Load Config** zeigt daher an jedem Rechner dasselbe Profil. Ältere Profile werden beim Laden ins aktuelle Format übernommen.
- Das Gerät meldet sich mit einer Seriennummer, die aus der MAC-Adresse des Chips gebildet wird.

### Bauen
#### Desktop-App
Voraussetzung ist [Rust](https://www.rust-lang.org/tools/install) 1.88 oder neuer (Edition 2024). Unter Linux braucht die Seriell-Bibliothek zusätzlich `libudev` (z. B. `sudo apt install libudev-dev pkg-config`).

```
cd flexpad-desktop
cargo run --release
```

`cargo test` führt die Unit-Tests aus; die CI prüft Formatierung, `clippy` und Tests unter Windows, Linux und macOS.

#### Firmware
Die Firmware in `flexpad-firmware/` braucht die Xtensa-Rust-Toolchain und `espflash`:

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

Zum Flashen muss das Board im Download-Modus sein. Läuft bereits die FlexPad-Firmware, über den seriellen Port `REBOOT_BOOTLOADER` senden; sonst **BOOT** gedrückt halten, **RESET** drücken und **BOOT** loslassen. Dann auf den neuen Port flashen:

```
espflash flash --no-stub --before no-reset --after watchdog-reset --partition-table partitions.csv --port <port> target/xtensa-esp32s3-none-elf/release/flexpad-firmware
```

Nach dem Flashen **RESET** drücken (oder das Board kurz abziehen), damit die Firmware startet. `espflash` meldet beim letzten Schritt `FlashEnd` eventuell einen Fehler; die Firmware ist trotzdem geschrieben. Das Profil liegt in einer eigenen Partition `flexpad` (64 KB) und bleibt beim Flashen einer neuen Firmware erhalten.

#### Releases
Eine neue Version in `flexpad-desktop/Cargo.toml` und `flexpad-firmware/Cargo.toml` (ein Test und der Release-Workflow prüfen, dass beide und der neueste Changelog-Abschnitt übereinstimmen) zusammen mit einem Abschnitt dafür in der [CHANGELOG.md](CHANGELOG.md) erzeugt beim Merge in `main` ein Release; der Changelog-Abschnitt wird zu den Release-Notizen. Änderungen ohne neue Version stehen unter `[Unreleased]`; ein Release schlägt fehl, solange dieser Abschnitt noch Einträge hat, sie müssen also in den Abschnitt der neuen Version verschoben werden. Versionen `0.x` und Versionen mit Zusatz wie `1.1.0-beta.1` erscheinen als Vorabversion. Das Repository-Secret `VT_API_KEY` (VirusTotal) ist optional.

### Lizenz
[MIT](LICENSE)
