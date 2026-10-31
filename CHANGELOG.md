# Changelog

All notable changes to FlexPad, grouped by version in the style of [Keep a Changelog](https://keepachangelog.com/). The section of each version is used as its release notes. Changes on `main` that are not released yet are collected under [Unreleased] and move into the section of the next version.

Alle wichtigen Änderungen an FlexPad, gegliedert nach Version im Stil von [Keep a Changelog](https://keepachangelog.com/de/). Der Abschnitt einer Version wird als ihre Release-Notizen verwendet. Änderungen auf `main`, die noch nicht veröffentlicht sind, stehen unter [Unreleased] und wandern mit der nächsten Version in deren Abschnitt.

## [Unreleased]

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
- Settings and profiles are stored in the usual folder of each system (`%APPDATA%FlexPad` on Windows, `~/Library/Application Support/FlexPad` on macOS, `~/.config/flexpad` and `~/.local/share/flexpad` on Linux) instead of a visible `FlexPad` folder in the home directory. Existing files are moved once on start.

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
- Einstellungen und Profile liegen im üblichen Ordner des jeweiligen Systems (`%APPDATA%FlexPad` unter Windows, `~/Library/Application Support/FlexPad` unter macOS, `~/.config/flexpad` und `~/.local/share/flexpad` unter Linux) statt in einem sichtbaren Ordner `FlexPad` im Benutzerverzeichnis. Vorhandene Dateien werden beim Start einmalig verschoben.

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
