#[cfg(test)]
mod tests {
    // Constants

    const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
    const FIRMWARE_MANIFEST: &str = include_str!("../../flexpad-firmware/Cargo.toml");
    const CHANGELOG: &str = include_str!("../../CHANGELOG.md");

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

    fn newest_changelog_version(changelog: &str) -> Option<&str> {
        changelog
            .lines()
            .find_map(|line| line.strip_prefix("## [")?.split_once(']'))
            .map(|(version, _)| version)
    }

    fn changelog_section<'a>(changelog: &'a str, version: &str) -> Option<&'a str> {
        let heading = format!("## [{version}]");
        let start = changelog.find(&heading)? + heading.len();
        let rest = &changelog[start..];
        let end = rest.find("\n## [").unwrap_or(rest.len());
        Some(rest[..end].trim())
    }

    // Versions

    #[test]
    fn firmware_has_the_app_version() {
        assert_eq!(package_version(FIRMWARE_MANIFEST), Some(APP_VERSION));
    }

    #[test]
    fn newest_changelog_section_has_the_app_version() {
        assert_eq!(newest_changelog_version(CHANGELOG), Some(APP_VERSION));
    }

    #[test]
    fn changelog_section_has_both_languages() {
        let section = changelog_section(CHANGELOG, APP_VERSION).unwrap_or_default();

        assert!(section.contains("### English"), "{section}");
        assert!(section.contains("### Deutsch"), "{section}");
    }

    // Helpers

    #[test]
    fn helpers_read_versions() {
        let manifest =
            "[package]\nname = \"x\"\nversion = \"1.2.3\"\n\n[dependencies]\nversion = \"9\"\n";
        let changelog =
            "# Changelog\n\n## [1.2.3] – 2026-01-01\n\ntext\n\n## [1.2.2] – 2025-12-01\n\nold\n";

        assert_eq!(package_version(manifest), Some("1.2.3"));
        assert_eq!(newest_changelog_version(changelog), Some("1.2.3"));
        assert_eq!(
            changelog_section(changelog, "1.2.3"),
            Some("– 2026-01-01\n\ntext")
        );
        assert_eq!(changelog_section(changelog, "0.9.0"), None);
    }
}
