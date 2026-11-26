# Changelog: FlexPad firmware

All notable changes to the FlexPad firmware for the ESP32-S3, grouped by version in the style of [Keep a Changelog](https://keepachangelog.com/). The section of each version is used as its release notes (tag `firmware-v<version>`). Changes on `main` that are not released yet are collected under [Unreleased] and move into the section of the next version. Up to 1.1.0, firmware and app were released together; those versions are described in the [changelog of the app](../flexpad-desktop/CHANGELOG.md).

Alle wichtigen Änderungen an der FlexPad-Firmware für den ESP32-S3, gegliedert nach Version im Stil von [Keep a Changelog](https://keepachangelog.com/de/). Der Abschnitt einer Version wird als ihre Release-Notizen verwendet (Tag `firmware-v<Version>`). Änderungen auf `main`, die noch nicht veröffentlicht sind, stehen unter [Unreleased] und wandern mit der nächsten Version in deren Abschnitt. Bis 1.1.0 erschienen Firmware und App gemeinsam; diese Versionen beschreibt der [Changelog der App](../flexpad-desktop/CHANGELOG.md).

## [Unreleased]

## [1.5.0] – 2026-11-26

### English

#### Added
- Long press for buttons and the push button of an encoder (function `long_press`, time `long_press_ms`, 200–2000 ms, default 500). With a long-press macro, a short press runs on release and a long press as soon as the time is reached; without one, a button still runs its macro right when it is pressed.
- Repeat while held (`repeat`, `repeat_delay_ms` 100–2000 ms, default 500, `repeat_interval_ms` 20–1000 ms, default 100): the macro of a button runs again and again until it is released, e.g. for volume or arrow keys. Repeat only applies without a long-press macro.

### Deutsch

#### Neu
- Langes Drücken für Taster und den Drucktaster eines Encoders (Funktion `long_press`, Zeit `long_press_ms`, 200–2000 ms, Standard 500). Mit Makro für langes Drücken läuft das kurze Drücken beim Loslassen und das lange, sobald die Zeit erreicht ist; ohne löst ein Taster wie bisher sofort beim Drücken aus.
- Wiederholen beim Halten (`repeat`, `repeat_delay_ms` 100–2000 ms, Standard 500, `repeat_interval_ms` 20–1000 ms, Standard 100): Das Makro eines Tasters läuft immer wieder, bis er losgelassen wird, z. B. für Lautstärke oder Pfeiltasten. Wiederholen gilt nur ohne Makro für langes Drücken.

## [1.4.0] – 2026-11-25

### English

#### Added
- Toggle switches (kind `switch`): one macro when the switch is turned on and one when it is turned off, debounced like a button. At start, the firmware only takes note of the position and runs no macro.

### Deutsch

#### Neu
- Kippschalter (Art `switch`): ein Makro beim Einschalten und eines beim Ausschalten, entprellt wie ein Taster. Beim Start merkt sich die Firmware nur die Stellung und führt kein Makro aus.

## [1.3.0] – 2026-11-24

### English

#### Added
- Text actions can be typed for the keyboard layouts English (UK) (`en-GB`), German (Switzerland) (`de-CH`) and French (`fr`), next to English (US) (`us`) and German (`de`).

#### Changed
- The keyboard layouts are built from the files in the folder `layouts`; English (US) and German type exactly as before.

### Deutsch

#### Neu
- Textaktionen lassen sich für die Tastaturlayouts Englisch (UK) (`en-GB`), Deutsch (Schweiz) (`de-CH`) und Französisch (`fr`) tippen, neben Englisch (US) (`us`) und Deutsch (`de`).

#### Geändert
- Die Tastaturlayouts entstehen aus den Dateien im Ordner `layouts`; Englisch (US) und Deutsch tippen genau wie bisher.

## [1.2.1] – 2026-11-24

### English

#### Added
- Each firmware release includes a file with the licenses of all third-party components.

### Deutsch

#### Neu
- Jedes Firmware-Release enthält eine Datei mit den Lizenzen aller Fremdkomponenten.

## [1.2.0] – 2026-11-01

### English

#### Added
- `GET_DEVICE_INFO` also reports the protocol version (`DEVICE_INFO:FlexPad,<serial number>,<firmware version>,1`), so the app can tell whether it understands a device without comparing version numbers.

### Deutsch

#### Neu
- `GET_DEVICE_INFO` meldet zusätzlich die Protokollversion (`DEVICE_INFO:FlexPad,<Seriennummer>,<Firmware-Version>,1`), damit die App erkennt, ob sie ein Gerät versteht, ohne Versionsnummern zu vergleichen.
