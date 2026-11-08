import subprocess
import tempfile
import unittest
from pathlib import Path

import dependency_updates as updates
from third_party_licenses import Package

# Helpers
UREQ_MESSAGE = """Bump ureq from 3.5.0 to 3.5.1 in /companion

Bumps [ureq](https://github.com/algesten/ureq) from 3.5.0 to 3.5.1.

---
updated-dependencies:
- dependency-name: ureq
  dependency-version: 3.5.1
  dependency-type: direct:production
  update-type: version-update:semver-patch
...

Signed-off-by: dependabot[bot] <support@github.com>"""

GROUP_MESSAGE = """Bump the rust group in /companion with 2 updates

---
updated-dependencies:
- dependency-name: serde
  dependency-version: 1.0.230
  dependency-type: direct:production
  update-type: version-update:semver-patch
  dependency-group: rust
- dependency-name: tempfile
  dependency-version: 3.30.0
  dependency-type: direct:development
  update-type: version-update:semver-minor
  dependency-group: rust
..."""

CHANGELOG = """# Changelog

## [Unreleased]

## [1.5.0] – 2026-11-10

### English

#### Changed
- Updated third-party components: `ureq` 3.5.1, serde 1.0.230.

### Deutsch

#### Geändert
- Fremdkomponenten aktualisiert: ureq 3.5.1.

## [1.4.1] – 2026-11-01

### English

- ureq 3.5.0
"""


def package(name, version):
    return Package(name=name, version=version, license="MIT", repository="", authors=[], directory=Path("."))


# Commit messages
class ParseUpdatesTests(unittest.TestCase):
    def test_reads_single_update(self):
        self.assertEqual(updates.parse_updates(UREQ_MESSAGE), [updates.Update("ureq", "3.5.1", "direct:production")])

    def test_reads_grouped_updates(self):
        self.assertEqual(
            updates.parse_updates(GROUP_MESSAGE),
            [
                updates.Update("serde", "1.0.230", "direct:production"),
                updates.Update("tempfile", "3.30.0", "direct:development"),
            ],
        )

    def test_ignores_message_without_metadata(self):
        self.assertEqual(updates.parse_updates("Bump ureq from 3.5.0 to 3.5.1"), [])


class CollectUpdatesTests(unittest.TestCase):
    def test_keeps_newest_version_and_skips_development(self):
        newer = UREQ_MESSAGE.replace("3.5.1", "3.6.0")
        result = updates.collect_updates([UREQ_MESSAGE, GROUP_MESSAGE, newer])
        self.assertEqual(sorted(result), ["serde", "ureq"])
        self.assertEqual(result["ureq"].version, "3.6.0")


class ShippedUpdatesTests(unittest.TestCase):
    def test_keeps_only_runtime_packages(self):
        collected = updates.collect_updates([UREQ_MESSAGE, GROUP_MESSAGE])
        result = updates.shipped_updates(collected, [package("ureq", "3.5.1"), package("libc", "0.2.180")])
        self.assertEqual(result, [updates.Update("ureq", "3.5.1")])

    def test_uses_locked_version_after_manual_update(self):
        collected = updates.collect_updates([UREQ_MESSAGE])
        result = updates.shipped_updates(collected, [package("ureq", "3.7.0")])
        self.assertEqual(result, [updates.Update("ureq", "3.7.0")])


# Changelog
class ChangelogTests(unittest.TestCase):
    def test_finds_version_section(self):
        section = updates.changelog_section(CHANGELOG, "1.5.0")
        self.assertIn("serde 1.0.230", section)
        self.assertNotIn("3.5.0", section)
        self.assertIsNone(updates.changelog_section(CHANGELOG, "2.0.0"))

    def test_reports_missing_language(self):
        section = updates.changelog_section(CHANGELOG, "1.5.0")
        shipped = [updates.Update("serde", "1.0.230"), updates.Update("ureq", "3.5.1")]
        self.assertEqual(updates.missing_entries(section, shipped), [("Deutsch", updates.Update("serde", "1.0.230"))])

    def test_matches_whole_name_and_version(self):
        self.assertTrue(updates.mentions("- Updated `ureq` 3.5.1.", updates.Update("ureq", "3.5.1")))
        self.assertTrue(updates.mentions("ureq v3.5.1, serde", updates.Update("ureq", "3.5.1")))
        self.assertFalse(updates.mentions("ureq 3.5.10", updates.Update("ureq", "3.5.1")))
        self.assertFalse(updates.mentions("ureq 3.5.1.2", updates.Update("ureq", "3.5.1")))
        self.assertFalse(updates.mentions("my-ureq 3.5.1", updates.Update("ureq", "3.5.1")))

    def test_suggests_entry_in_both_languages(self):
        entry = updates.suggested_entry([updates.Update("ureq", "3.5.1")])
        self.assertIn("### English", entry)
        self.assertIn("### Deutsch", entry)
        self.assertEqual(entry.count("ureq 3.5.1"), 2)


# Git
class GitTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.path = Path(self.directory.name)
        self.run_git("init", "--quiet")
        (self.path / "app").mkdir()
        self.commit("Initial", "Someone", "lock 1", tag="v1.0.0")

    def run_git(self, *arguments):
        subprocess.run(["git", *arguments], cwd=self.path, check=True, capture_output=True)

    def commit(self, message, author, lock, tag=None, path="app/Cargo.lock"):
        (self.path / path).write_text(lock, encoding="utf-8")
        self.run_git("add", "--all")
        self.run_git(
            "-c", f"user.name={author}", "-c", "user.email=bot@example.com", "commit", "--quiet", "-m", message
        )
        if tag:
            self.run_git("tag", tag)

    def test_reads_dependabot_commits_since_tag(self):
        self.commit(UREQ_MESSAGE, "dependabot[bot]", "lock 2")
        self.commit(GROUP_MESSAGE, "Someone", "lock 3")
        self.commit(UREQ_MESSAGE.replace("ureq", "toml"), "dependabot[bot]", "actions", path="workflow.yml")
        self.assertEqual(updates.latest_tag("v*", self.path), "v1.0.0")
        messages = updates.dependabot_messages("v1.0.0", "app/Cargo.lock", self.path)
        self.assertEqual(list(updates.collect_updates(messages)), ["ureq"])

    def test_matches_tag_pattern(self):
        self.commit("Firmware", "Someone", "lock 2", tag="firmware-v2.0.0")
        self.assertEqual(updates.latest_tag("v*", self.path), "v1.0.0")
        self.assertEqual(updates.latest_tag("firmware-v*", self.path), "firmware-v2.0.0")

    def test_returns_no_tag_without_release(self):
        self.run_git("tag", "--delete", "v1.0.0")
        self.assertIsNone(updates.latest_tag("v*", self.path))


if __name__ == "__main__":
    unittest.main()
