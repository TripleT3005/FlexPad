#[cfg(test)]
mod tests {
    // Constants

    const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
    const APP_CHANGELOG: &str = include_str!("../CHANGELOG.md");
    const FIRMWARE_MANIFEST: &str = include_str!("../../flexpad-firmware/Cargo.toml");
    const FIRMWARE_CHANGELOG: &str = include_str!("../../flexpad-firmware/CHANGELOG.md");
    const LAST_SHARED_VERSION: &str = "1.1.0";
    const UNRELEASED: &str = "Unreleased";

    // Helpers

    fn package_version(manifest: &str) -> Option<&str> {
        manifest
            .lines()
            .skip_while(|line| line.trim() != "[package]")
            .skip(1)
            .take_while(|line| !line.trim_start().starts_with('['))
            .find_map(|line| {
                let value = line.trim().strip_prefix("version")?.trim_start();
                value
                    .strip_prefix('=')?
                    .trim()
                    .strip_prefix('"')?
                    .strip_suffix('"')
            })
    }

    fn changelog_headings(changelog: &str) -> impl Iterator<Item = &str> {
        changelog
            .lines()
            .filter_map(|line| line.strip_prefix("## [")?.split_once(']'))
            .map(|(name, _)| name)
    }

    fn newest_changelog_version(changelog: &str) -> Option<&str> {
        changelog_headings(changelog).find(|&name| name != UNRELEASED)
    }

    fn changelog_section<'a>(changelog: &'a str, version: &str) -> Option<&'a str> {
        let heading = format!("## [{version}]");
        let start = changelog.find(&heading)? + heading.len();
        let rest = &changelog[start..];
        let end = rest.find("\n## [").unwrap_or(rest.len());
        Some(rest[..end].trim())
    }

    fn assert_both_languages(section: &str) {
        assert!(section.contains("### English"), "{section}");
        assert!(section.contains("### Deutsch"), "{section}");
    }

    fn assert_unreleased_is_valid(changelog: &str) {
        let position = changelog_headings(changelog).position(|name| name == UNRELEASED);
        assert!(matches!(position, None | Some(0)), "{position:?}");

        let section = changelog_section(changelog, UNRELEASED).unwrap_or_default();
        if !section.is_empty() {
            assert_both_languages(section);
        }
    }

    // App

    #[test]
    fn newest_app_changelog_section_has_the_app_version() {
        assert_eq!(newest_changelog_version(APP_CHANGELOG), Some(APP_VERSION));
        assert_both_languages(changelog_section(APP_CHANGELOG, APP_VERSION).unwrap_or_default());
    }

    #[test]
    fn app_changelog_keeps_unreleased_on_top() {
        assert_unreleased_is_valid(APP_CHANGELOG);
    }

    // Firmware

    #[test]
    fn newest_firmware_changelog_section_has_the_firmware_version() {
        let version = package_version(FIRMWARE_MANIFEST).expect("firmware version");
        match newest_changelog_version(FIRMWARE_CHANGELOG) {
            Some(newest) => {
                assert_eq!(newest, version);
                assert_both_languages(
                    changelog_section(FIRMWARE_CHANGELOG, version).unwrap_or_default(),
                );
            }
            None => assert_eq!(version, LAST_SHARED_VERSION),
        }
    }

    #[test]
    fn firmware_changelog_keeps_unreleased_on_top() {
        assert_unreleased_is_valid(FIRMWARE_CHANGELOG);
    }

    #[test]
    fn app_embeds_the_current_firmware_version() {
        assert_eq!(
            package_version(FIRMWARE_MANIFEST),
            Some(crate::flasher::FIRMWARE_VERSION)
        );
    }

    // Helpers

    #[test]
    fn helpers_read_versions() {
        let manifest =
            "[package]\nname = \"x\"\nversion = \"1.2.3\"\n\n[dependencies]\nversion = \"9\"\n";
        let changelog =
            "# Changelog\n\n## [1.2.3] – 2026-01-01\n\ntext\n\n## [1.2.2] – 2025-12-01\n\nold\n";
        let with_unreleased = "# Changelog\n\n## [Unreleased]\n\nnew\n\n## [1.2.3] – 2026-01-01\n";

        assert_eq!(package_version(manifest), Some("1.2.3"));
        assert_eq!(newest_changelog_version(changelog), Some("1.2.3"));
        assert_eq!(newest_changelog_version(with_unreleased), Some("1.2.3"));
        assert_eq!(
            newest_changelog_version("# Changelog\n\n## [Unreleased]\n"),
            None
        );
        assert_eq!(changelog_section(with_unreleased, UNRELEASED), Some("new"));
        assert_eq!(
            changelog_section(changelog, "1.2.3"),
            Some("– 2026-01-01\n\ntext")
        );
        assert_eq!(changelog_section(changelog, "0.9.0"), None);
    }
}
