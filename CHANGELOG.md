# Changelog

All notable changes to FlexPad, grouped by version in the style of [Keep a Changelog](https://keepachangelog.com/). The section of each version is used as its release notes.

Alle wichtigen Änderungen an FlexPad, gegliedert nach Version im Stil von [Keep a Changelog](https://keepachangelog.com/de/). Der Abschnitt einer Version wird als ihre Release-Notizen verwendet.

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
