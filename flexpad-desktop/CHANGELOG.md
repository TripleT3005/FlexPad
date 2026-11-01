# Changelog: FlexPad configurator

All notable changes to the FlexPad configurator (desktop app), grouped by version in the style of [Keep a Changelog](https://keepachangelog.com/). The section of each version is used as its release notes (tag `v<version>`). Changes on `main` that are not released yet are collected under [Unreleased] and move into the section of the next version. Up to 1.1.0, app and firmware were released together, so those sections also list firmware changes; the firmware has its own [changelog](../flexpad-firmware/CHANGELOG.md) since then.

Alle wichtigen Änderungen am FlexPad-Konfigurator (Desktop-App), gegliedert nach Version im Stil von [Keep a Changelog](https://keepachangelog.com/de/). Der Abschnitt einer Version wird als ihre Release-Notizen verwendet (Tag `v<Version>`). Änderungen auf `main`, die noch nicht veröffentlicht sind, stehen unter [Unreleased] und wandern mit der nächsten Version in deren Abschnitt. Bis 1.1.0 erschienen App und Firmware gemeinsam, diese Abschnitte nennen daher auch Änderungen an der Firmware; seitdem hat die Firmware ihren eigenen [Changelog](../flexpad-firmware/CHANGELOG.md).

## [Unreleased]

## [1.2.0] – 2026-11-01

### English

#### Changed
- The configurator uses the green of the logo for buttons, selections and tiles; backgrounds stay neutral gray in light and dark mode. All colors are defined in one place (`flexpad-desktop/src/theme.rs`).
- The connection state in the status bar is a colored dot instead of an emoji, and the arrows of the function list no longer show as blue emoji on Windows.
- App and firmware have their own version numbers and release notes. The note about an outdated firmware compares the device with the firmware built into the app, and a device with a newer protocol than the app understands asks for an app update.

### Deutsch

#### Geändert
- Der Konfigurator nutzt das Grün des Logos für Knöpfe, Auswahl und Kacheln; Hintergründe bleiben im hellen und dunklen Modus neutral grau. Alle Farben sind an einer Stelle festgelegt (`flexpad-desktop/src/theme.rs`).
- Der Verbindungsstatus in der Statusleiste ist ein farbiger Punkt statt eines Emojis, und die Pfeile der Funktionsliste erscheinen unter Windows nicht mehr als blaue Emojis.
- App und Firmware haben eigene Versionsnummern und Release-Notizen. Der Hinweis auf eine veraltete Firmware vergleicht das Gerät mit der in die App eingebauten Firmware, und ein Gerät mit neuerem Protokoll, als die App versteht, bittet um ein Update der App.

## [1.1.0] – 2026-11-01

### English

#### Added
- The window remembers its position and size (also maximized) in `window.json` in the local app data of the computer, separate from the roaming settings. On first start it opens centered at its minimum size; if the saved position is no longer on any screen, e.g. after unplugging a second monitor, it falls back to that. On Windows the position is restored, on macOS and Linux only the size.

#### Changed
- **Scan** is only available while no device is connected, **Connect** only with a selected device, and **Load Config** and **Set Config** only while connected.
- The update check asks GitHub only for the latest release instead of a list of releases.

#### Fixed
- Drop-down lists no longer look broken for a moment when their content changes: the window is drawn with the graphics card again, with software drawing as fallback.
- "Advanced View" no longer touches the "Tile Inspector" title.

### Deutsch

#### Neu
- Das Fenster merkt sich Position und Größe (auch maximiert) in `window.json` in den lokalen App-Daten des Rechners, getrennt von den mitwandernden Einstellungen. Beim ersten Start öffnet es sich mittig in seiner Mindestgröße; liegt die gespeicherte Position auf keinem Bildschirm mehr, etwa nach dem Abstecken eines zweiten Monitors, fällt es darauf zurück. Unter Windows wird die Position wiederhergestellt, unter macOS und Linux nur die Größe.

#### Geändert
- **Scan** ist nur ohne verbundenes Gerät verfügbar, **Connect** nur mit ausgewähltem Gerät, **Load Config** und **Set Config** nur bei bestehender Verbindung.
- Die Update-Prüfung fragt bei GitHub nur noch das neueste Release ab statt einer Liste von Releases.

#### Behoben
- Auswahllisten sehen nicht mehr kurz kaputt aus, wenn sich ihr Inhalt ändert: Das Fenster wird wieder mit der Grafikkarte gezeichnet, Software-Zeichnen bleibt als Rückfall.
- „Advanced View“ klebt nicht mehr am Titel „Tile Inspector“.

## [1.0.0] – 2026-11-01

### English

#### Added
- The configurator checks GitHub for a newer release at start and every 24 hours and shows "Version x.y.z available" in the status bar with a link to the release page. Nothing is downloaded. The check can be switched off with "Check for updates" in the status bar; then no request is made.
- The configurator points out when the firmware of the connected device is older than the app.
- "Test" button next to each function in the Tile Inspector: runs the macro once on the connected device without saving and shows the answer, e.g. an unknown key, at the function.
- Wiring test: while connected, pressing a button or turning an encoder briefly highlights its tile on the canvas; pins without a tile are named in the status bar, e.g. "GPIO 12 pressed – not assigned".
- "Update firmware" next to the outdated-firmware note writes the firmware built into the app to the FlexPad, with a progress bar, and restarts it. The profile stored on the device is kept.
- A board in bootloader mode (e.g. a new board, or after holding BOOT) is found by "Scan" and can be set up with "Install firmware".

#### Changed
- Tidier window: device and profile each have one toolbar row at the top, notes about the firmware appear in a bar below them only when needed, and the connection state, messages, the update notice and "Check for updates" are in a status bar at the bottom. "Scan Devices" is now "Scan".
- The configurator draws its window in software instead of through the graphics card and uses about a third fewer third-party packages.
- The configurator reads from the device in the background all the time, so device events and command answers no longer get in each other's way; unplugging is noticed right away.

### Deutsch

#### Neu
- Der Konfigurator prüft beim Start und alle 24 Stunden bei GitHub, ob es ein neueres Release gibt, und zeigt in der Statusleiste „Version x.y.z available“ mit Link zur Release-Seite an. Es wird nichts heruntergeladen. Die Prüfung lässt sich mit „Check for updates“ in der Statusleiste abschalten; dann erfolgt keine Anfrage.
- Der Konfigurator weist darauf hin, wenn die Firmware des verbundenen Geräts älter ist als die App.
- Knopf „Test“ neben jeder Funktion im Tile Inspector: führt das Makro einmal auf dem verbundenen Gerät aus, ohne zu speichern, und zeigt die Antwort, z. B. eine unbekannte Taste, an der Funktion an.
- Verdrahtungstest: Solange die App verbunden ist, hebt ein Tastendruck oder eine Encoder-Drehung die Kachel kurz auf der Arbeitsfläche hervor; Pins ohne Kachel nennt die Statusleiste, z. B. „GPIO 12 pressed – not assigned“.
- „Update firmware“ neben dem Hinweis auf veraltete Firmware schreibt die in die App eingebaute Firmware mit Fortschrittsbalken auf den FlexPad und startet ihn neu. Das auf dem Gerät gespeicherte Profil bleibt erhalten.
- Ein Board im Bootloader-Modus (z. B. ein neues Board oder nach Halten von BOOT) findet „Scan“, und „Install firmware“ richtet es ein.

#### Geändert
- Aufgeräumteres Fenster: Gerät und Profil haben oben je eine Werkzeugzeile, Hinweise zur Firmware erscheinen nur bei Bedarf in einer Leiste darunter, und Verbindungsstatus, Meldungen, Update-Hinweis und „Check for updates“ stehen in einer Statusleiste unten. „Scan Devices“ heißt jetzt „Scan“.
- Der Konfigurator zeichnet sein Fenster per Software statt über die Grafikkarte und nutzt rund ein Drittel weniger Fremdpakete.
- Der Konfigurator liest im Hintergrund ständig vom Gerät, sodass sich Ereignisse und Antworten auf Befehle nicht mehr in die Quere kommen; das Abziehen des Geräts wird sofort bemerkt.

## [0.2.0] – 2026-10-31

### English

#### Added
- Text actions are typed by the device, for the keyboard layout chosen in the profile (English (US) or German), including umlauts, AltGr symbols such as `@` and `€` and accented letters via dead keys. Characters the layout cannot type are skipped.
- The configurator lists the characters of a text action that the keyboard layout of the profile cannot type, e.g. umlauts on a US layout.
- Keyboard layout in the profile (English (US) or German): keys in the configurator are shown with the labels printed on that layout, e.g. "Z" for the key next to "T" on a German keyboard.

#### Changed
- The window keeps responding while the app talks to the device. Scan Devices, Connect, Load Config and Set Config are disabled until the running access is finished, and the status line shows what is running.
- Keys are stored as physical key positions (`KeyA`, `ControlLeft`, `Semicolon`, …), the same names the firmware uses. Older profiles are converted automatically when loaded.
- Macros are stored in the profile as a list of actions instead of a comma-separated string. Older profiles are converted when loaded, and the firmware still runs profiles stored on the device before the update. **Advanced View** now shows the macro as JSON.
- Text in macros is stored exactly as typed; umlauts are no longer replaced by `ae`, `oe` etc.
- Settings and profiles are stored in the usual folder of each system (`%APPDATA%\FlexPad` on Windows, `~/Library/Application Support/FlexPad` on macOS, `~/.config/flexpad` and `~/.local/share/flexpad` on Linux) instead of a visible `FlexPad` folder in the home directory. Existing files are moved once on start.

#### Fixed
- Text actions no longer change with every edit of the macro (`y` and `z` swapped back and forth).
- Commas, backslashes and quotes in text actions no longer break the macro.
- Profiles whose name contains a space are selected again on start and after saving.

#### Security
- Profile file names only use letters, digits, `-` and `_`. A profile name with `/`, `\` or `..`, for example one read from a device, can no longer write outside the profiles folder; names that Windows does not allow are saved under an adjusted file name.

### Deutsch

#### Neu
- Textaktionen werden vom Gerät getippt, passend zum im Profil gewählten Tastaturlayout (English (US) oder German), einschließlich Umlauten, AltGr-Zeichen wie `@` und `€` und Buchstaben mit Akzent über Tottasten. Zeichen, die das Layout nicht tippen kann, werden übersprungen.
- Der Konfigurator listet die Zeichen einer Textaktion auf, die das Tastaturlayout des Profils nicht tippen kann, z. B. Umlaute bei US-Layout.
- Tastaturlayout im Profil (English (US) oder German): Tasten werden im Konfigurator mit den Beschriftungen dieses Layouts angezeigt, z. B. „Z“ für die Taste neben „T“ auf einer deutschen Tastatur.

#### Geändert
- Das Fenster reagiert weiter, während die App mit dem Gerät spricht. Scan Devices, Connect, Load Config und Set Config sind gesperrt, bis der laufende Zugriff fertig ist, und die Statuszeile zeigt, was gerade läuft.
- Tasten werden als physische Tastenpositionen gespeichert (`KeyA`, `ControlLeft`, `Semicolon`, …), mit denselben Namen wie in der Firmware. Ältere Profile werden beim Laden automatisch umgestellt.
- Makros werden im Profil als Liste von Aktionen statt als kommagetrennter String gespeichert. Ältere Profile werden beim Laden umgewandelt, und die Firmware führt Profile, die vor dem Update auf dem Gerät gespeichert wurden, weiterhin aus. **Advanced View** zeigt das Makro jetzt als JSON.
- Text in Makros wird genau so gespeichert, wie er eingegeben wurde; Umlaute werden nicht mehr durch `ae`, `oe` usw. ersetzt.
- Einstellungen und Profile liegen im üblichen Ordner des jeweiligen Systems (`%APPDATA%\FlexPad` unter Windows, `~/Library/Application Support/FlexPad` unter macOS, `~/.config/flexpad` und `~/.local/share/flexpad` unter Linux) statt in einem sichtbaren Ordner `FlexPad` im Benutzerverzeichnis. Vorhandene Dateien werden beim Start einmalig verschoben.

#### Behoben
- Textaktionen ändern sich nicht mehr bei jeder Bearbeitung des Makros (`y` und `z` wurden hin- und hergetauscht).
- Kommas, Backslashes und Anführungszeichen in Textaktionen zerstören das Makro nicht mehr.
- Profile mit Leerzeichen im Namen werden beim Start und nach dem Speichern wieder ausgewählt.

#### Sicherheit
- Dateinamen von Profilen bestehen nur aus Buchstaben, Ziffern, `-` und `_`. Ein Profilname mit `/`, `\` oder `..`, etwa von einem Gerät gelesen, kann nicht mehr außerhalb des Profilordners schreiben; Namen, die Windows nicht erlaubt, werden unter einem angepassten Dateinamen gespeichert.

## [0.1.0] – 2026-10-31

### English

#### Added
- First release of the FlexPad configurator for Windows, macOS and Linux and of the FlexPad firmware for the ESP32-S3.
- Place push buttons and EC11 rotary encoders on a canvas, choose their GPIO pins and assign macros to pressing, turning right and turning left.
- Macros from presets or step by step: press and release keys, delays and media keys (volume, mute, play, next, previous).
- Debounce time, steps per detent and turning direction adjustable per tile.
- Profiles are stored on your computer and on the device; **Load Config** shows the complete profile on any computer. Profiles of earlier development versions are converted when loaded.
- The device works without the app once configured, checks every configuration before saving it and keeps the previous one if saving is interrupted.
- The firmware is available as a single image to flash with `espflash`.

### Deutsch

#### Neu
- Erstes Release des FlexPad-Konfigurators für Windows, macOS und Linux und der FlexPad-Firmware für den ESP32-S3.
- Taster und EC11-Drehencoder auf einer Arbeitsfläche platzieren, ihre GPIO-Pins wählen und Makros für Drücken, Rechtsdrehen und Linksdrehen zuordnen.
- Makros aus Vorlagen oder Schritt für Schritt: Tasten drücken und loslassen, Pausen und Medientasten (Lautstärke, Stumm, Wiedergabe, Weiter, Zurück).
- Entprellzeit, Schritte pro Raste und Drehrichtung pro Kachel einstellbar.
- Profile werden auf dem Rechner und auf dem Gerät gespeichert; **Load Config** zeigt an jedem Rechner das vollständige Profil. Profile früherer Entwicklungsstände werden beim Laden übernommen.
- Einmal eingerichtet, arbeitet das Gerät ohne die App, prüft jede Konfiguration vor dem Speichern und behält die vorherige, wenn das Speichern unterbrochen wird.
- Die Firmware gibt es als einzelnes Image zum Flashen mit `espflash`.
