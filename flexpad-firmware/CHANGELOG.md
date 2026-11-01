# Changelog: FlexPad firmware

All notable changes to the FlexPad firmware for the ESP32-S3, grouped by version in the style of [Keep a Changelog](https://keepachangelog.com/). The section of each version is used as its release notes (tag `firmware-v<version>`). Changes on `main` that are not released yet are collected under [Unreleased] and move into the section of the next version. Up to 1.1.0, firmware and app were released together; those versions are described in the [changelog of the app](../flexpad-desktop/CHANGELOG.md).

Alle wichtigen Änderungen an der FlexPad-Firmware für den ESP32-S3, gegliedert nach Version im Stil von [Keep a Changelog](https://keepachangelog.com/de/). Der Abschnitt einer Version wird als ihre Release-Notizen verwendet (Tag `firmware-v<Version>`). Änderungen auf `main`, die noch nicht veröffentlicht sind, stehen unter [Unreleased] und wandern mit der nächsten Version in deren Abschnitt. Bis 1.1.0 erschienen Firmware und App gemeinsam; diese Versionen beschreibt der [Changelog der App](../flexpad-desktop/CHANGELOG.md).

## [Unreleased]

## [1.2.0] – 2026-11-01

### English

#### Added
- `GET_DEVICE_INFO` also reports the protocol version (`DEVICE_INFO:FlexPad,<serial number>,<firmware version>,1`), so the app can tell whether it understands a device without comparing version numbers.

### Deutsch

#### Neu
- `GET_DEVICE_INFO` meldet zusätzlich die Protokollversion (`DEVICE_INFO:FlexPad,<Seriennummer>,<Firmware-Version>,1`), damit die App erkennt, ob sie ein Gerät versteht, ohne Versionsnummern zu vergleichen.
