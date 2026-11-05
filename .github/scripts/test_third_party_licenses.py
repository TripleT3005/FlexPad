import tempfile
import unittest
from pathlib import Path

import third_party_licenses as licenses

# Helpers
APACHE_TEXT = "Apache License\nVersion 2.0, January 2004\n"
MIT_TEXT = "Copyright (c) Someone\n\nPermission is hereby granted, free of charge, to any person\n"


def make_package(directory, name="example", license_expression="MIT", files=None):
    package_directory = Path(directory) / name
    package_directory.mkdir()
    for file_name, text in (files or {}).items():
        (package_directory / file_name).write_text(text, encoding="utf-8")
    return licenses.Package(
        name=name,
        version="1.0.0",
        license=license_expression,
        repository=f"https://example.com/{name}",
        authors=["Jane Doe <jane@example.com>"],
        directory=package_directory,
    )


# Expressions
class ChooseLicensesTests(unittest.TestCase):
    def test_prefers_mit_from_dual_license(self):
        self.assertEqual(licenses.choose_licenses("MIT OR Apache-2.0"), ["MIT"])
        self.assertEqual(licenses.choose_licenses("Apache-2.0/MIT"), ["MIT"])

    def test_picks_allowed_alternative_to_gpl(self):
        self.assertEqual(licenses.choose_licenses("MIT OR GPL-3.0-only"), ["MIT"])
        self.assertEqual(licenses.choose_licenses("Apache-2.0 OR GPL-2.0-only"), ["Apache-2.0"])

    def test_requires_every_part_of_and(self):
        self.assertEqual(licenses.choose_licenses("(MIT OR Apache-2.0) AND Unicode-3.0"), ["MIT", "Unicode-3.0"])

    def test_keeps_exception(self):
        self.assertEqual(
            licenses.choose_licenses("Apache-2.0 WITH LLVM-exception OR Zlib"),
            ["Apache-2.0 WITH LLVM-exception"],
        )

    def test_rejects_copyleft_only(self):
        for expression in ["AGPL-3.0-only", "GPL-3.0-or-later", "MIT AND GPL-2.0-only"]:
            with self.subTest(expression=expression), self.assertRaises(licenses.LicenseError):
                licenses.choose_licenses(expression)

    def test_rejects_malformed_expression(self):
        with self.assertRaises(licenses.LicenseError):
            licenses.choose_licenses("(MIT OR Apache-2.0")


# Sections
class BuildSectionsTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)

    def test_uses_matching_file_of_dual_license(self):
        package = make_package(
            self.directory.name,
            license_expression="MIT OR Apache-2.0",
            files={"LICENSE-APACHE": APACHE_TEXT, "LICENSE-MIT": MIT_TEXT},
        )
        sections = licenses.build_sections([package])
        self.assertEqual(len(sections), 1)
        self.assertEqual(sections[0].license, "MIT")
        self.assertIn("Copyright (c) Someone", sections[0].text)

    def test_generates_mit_text_without_file(self):
        package = make_package(self.directory.name)
        sections = licenses.build_sections([package])
        self.assertIn("Copyright (c) Jane Doe\n", sections[0].text)

    def test_shares_apache_text_with_other_component(self):
        with_file = make_package(self.directory.name, "first", "Apache-2.0", {"LICENSE": APACHE_TEXT})
        without_file = make_package(self.directory.name, "second", "Apache-2.0")
        sections = licenses.build_sections([with_file, without_file])
        self.assertEqual(len(sections), 1)
        self.assertEqual([package.name for package in sections[0].packages], ["first", "second"])

    def test_lists_no_notice_license_without_file(self):
        package = make_package(self.directory.name, license_expression="CC0-1.0")
        sections = licenses.build_sections([package])
        self.assertIn("does not require", sections[0].text)

    def test_includes_notice_file(self):
        package = make_package(
            self.directory.name,
            license_expression="Apache-2.0",
            files={"LICENSE": APACHE_TEXT, "NOTICE": "Example notice\n"},
        )
        sections = licenses.build_sections([package])
        self.assertEqual(sections[0].notices, ["Example notice\n"])

    def test_reports_missing_text_and_license(self):
        packages = [
            make_package(self.directory.name, "unlicensed", ""),
            make_package(self.directory.name, "textless", "ISC"),
            make_package(self.directory.name, "copyleft", "AGPL-3.0-only"),
        ]
        with self.assertRaises(licenses.LicenseError) as context:
            licenses.build_sections(packages)
        message = str(context.exception)
        self.assertIn("unlicensed 1.0.0: no license declared", message)
        self.assertIn("textless 1.0.0: no ISC license text found", message)
        self.assertIn("copyleft 1.0.0: No allowed license", message)


# Report
class RenderTests(unittest.TestCase):
    def test_renders_header_sources_and_notes(self):
        with tempfile.TemporaryDirectory() as directory:
            package = make_package(directory, "serialport", "MPL-2.0", {"LICENSE": "Mozilla Public License, v. 2.0\n"})
            notes = {"serialport": "used unmodified"}
            output = licenses.render("Example", licenses.build_sections([package]), notes)
        self.assertTrue(output.startswith("Example - Third-party licenses\n"))
        self.assertIn("serialport 1.0.0\nLicense: MPL-2.0\nSource: https://example.com/serialport\n", output)
        self.assertIn("Note:   serialport: used unmodified\n", output)


if __name__ == "__main__":
    unittest.main()
