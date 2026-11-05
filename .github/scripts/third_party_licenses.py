#!/usr/bin/env python3
import argparse
import json
import re
import subprocess
import sys
from dataclasses import dataclass, field
from pathlib import Path

# Constants
ALLOWED_LICENSES = [
    "MIT",
    "Apache-2.0",
    "Apache-2.0 WITH LLVM-exception",
    "BSD-3-Clause",
    "BSD-2-Clause",
    "ISC",
    "Zlib",
    "0BSD",
    "Unlicense",
    "BSL-1.0",
    "CC0-1.0",
    "Unicode-3.0",
    "Unicode-DFS-2016",
    "CDLA-Permissive-2.0",
    "MPL-2.0",
]

CONTENT_MARKERS = [
    ("Apache-2.0", "Apache License"),
    ("MIT", "Permission is hereby granted, free of charge"),
    ("MPL-2.0", "Mozilla Public License"),
    ("Unlicense", "This is free and unencumbered software"),
    ("BSL-1.0", "Boost Software License"),
    ("Zlib", "This software is provided 'as-is'"),
    ("Unicode-3.0", "UNICODE LICENSE V3"),
    ("Unicode-DFS-2016", "UNICODE, INC. LICENSE AGREEMENT"),
    ("CDLA-Permissive-2.0", "Community Data License Agreement"),
    ("CC0-1.0", "CC0 1.0 Universal"),
    ("BSD-3-Clause", "Neither the name of"),
    ("BSD-2-Clause", "Redistribution and use in source and binary forms"),
    ("ISC", "Permission to use, copy, modify, and/or distribute this software for any"),
    ("ISC", "Permission to use, copy, modify, and distribute this software for any"),
]

NAME_MARKERS = [
    ("Apache-2.0", "apache"),
    ("MIT", "mit"),
    ("MPL-2.0", "mpl"),
    ("Unlicense", "unlicense"),
    ("BSL-1.0", "bsl"),
    ("BSL-1.0", "boost"),
    ("Zlib", "zlib"),
    ("ISC", "isc"),
    ("BSD-3-Clause", "bsd"),
    ("BSD-2-Clause", "bsd"),
    ("Unicode-3.0", "unicode"),
    ("CDLA-Permissive-2.0", "cdla"),
    ("CC0-1.0", "cc0"),
]

NO_NOTICE_LICENSES = {"0BSD", "BSL-1.0", "CC0-1.0", "Unlicense"}

LICENSE_URL = "https://spdx.org/licenses/{license}.html"

LICENSE_FILE_PATTERN = re.compile(r"^(licen[cs]e|copying|unlicense|copyright)", re.IGNORECASE)
NOTICE_FILE_PATTERN = re.compile(r"^notice", re.IGNORECASE)

MIT_TEMPLATE = """{copyright}

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
"""

NO_NOTICE_TEXT = """The {license} license does not require its text to be included in binary
distributions, and the component ships no license file. Full text: {url}
"""

RULE = "-" * 72


# Types
class LicenseError(Exception):
    pass


@dataclass
class Package:
    name: str
    version: str
    license: str
    repository: str
    authors: list
    directory: Path
    license_file: Path | None = None


@dataclass
class Section:
    license: str
    text: str
    notices: list = field(default_factory=list)
    packages: list = field(default_factory=list)


# Expressions
def tokenize(expression):
    normalized = expression.replace("/", " OR ")
    return re.findall(r"\(|\)|[^\s()]+", normalized)


def parse_expression(expression):
    tokens = tokenize(expression)
    position = 0

    def peek():
        return tokens[position] if position < len(tokens) else None

    def take():
        nonlocal position
        position += 1
        return tokens[position - 1]

    def parse_or():
        options = [parse_and()]
        while peek() == "OR":
            take()
            options.append(parse_and())
        return options[0] if len(options) == 1 else ("OR", options)

    def parse_and():
        parts = [parse_atom()]
        while peek() == "AND":
            take()
            parts.append(parse_atom())
        return parts[0] if len(parts) == 1 else ("AND", parts)

    def parse_atom():
        token = peek()
        if token is None:
            raise LicenseError(f"Incomplete license expression: {expression}")
        if token == "(":
            take()
            node = parse_or()
            if peek() != ")":
                raise LicenseError(f"Unbalanced license expression: {expression}")
            take()
            return node
        take()
        if peek() == "WITH":
            take()
            return f"{token} WITH {take()}"
        return token

    node = parse_or()
    if peek() is not None:
        raise LicenseError(f"Unexpected token in license expression: {expression}")
    return node


def choose_licenses(expression):
    def choose(node):
        if isinstance(node, str):
            return [node] if node in ALLOWED_LICENSES else None
        operator, children = node
        if operator == "AND":
            chosen = []
            for child in children:
                part = choose(child)
                if part is None:
                    return None
                chosen.extend(part)
            return chosen
        candidates = [choice for choice in (choose(child) for child in children) if choice is not None]
        if not candidates:
            return None
        return min(candidates, key=lambda choice: max(ALLOWED_LICENSES.index(item) for item in choice))

    chosen = choose(parse_expression(expression))
    if chosen is None:
        raise LicenseError(f"No allowed license in: {expression}")
    return list(dict.fromkeys(chosen))


# License texts
def base_license(license_id):
    return license_id.split(" WITH ")[0]


def detect_licenses(path):
    text = path.read_text(encoding="utf-8", errors="replace")
    found = {license_id for license_id, marker in CONTENT_MARKERS if marker.lower() in text.lower()}
    if "BSD-3-Clause" in found:
        found.discard("BSD-2-Clause")
    if "Apache-2.0" in found and "Permission is hereby granted" not in text:
        found.discard("MIT")
    name = path.name.lower()
    by_name = {license_id for license_id, marker in NAME_MARKERS if marker in name}
    if by_name and found & by_name:
        return found & by_name
    return found


def license_files(package):
    files = []
    if package.license_file and package.license_file.is_file():
        files.append(package.license_file)
    if package.directory.is_dir():
        files.extend(
            path
            for path in sorted(package.directory.iterdir())
            if path.is_file() and LICENSE_FILE_PATTERN.match(path.name) and path not in files
        )
    return files


def notice_files(package):
    if not package.directory.is_dir():
        return []
    return [path for path in sorted(package.directory.iterdir()) if path.is_file() and NOTICE_FILE_PATTERN.match(path.name)]


def read_text(path):
    return path.read_text(encoding="utf-8", errors="replace").replace("\r\n", "\n").strip() + "\n"


def mit_text(package):
    authors = ", ".join(re.sub(r"\s*<[^>]*>", "", author) for author in package.authors) or f"The {package.name} authors"
    return MIT_TEMPLATE.format(copyright=f"Copyright (c) {authors}")


def find_text(package, license_id):
    files = license_files(package)
    wanted = base_license(license_id)
    matches = [path for path in files if wanted in detect_licenses(path)]
    if matches:
        return read_text(matches[0])
    if wanted == "MIT":
        return mit_text(package)
    return None


# Cargo
def cargo_metadata(manifest_path, platforms):
    command = ["cargo", "metadata", "--format-version", "1", "--locked", "--manifest-path", str(manifest_path)]
    for platform in platforms:
        command.extend(["--filter-platform", platform])
    result = subprocess.run(command, check=True, capture_output=True, text=True, encoding="utf-8")
    return json.loads(result.stdout)


def runtime_packages(metadata):
    nodes = {node["id"]: node for node in metadata["resolve"]["nodes"]}
    packages = {package["id"]: package for package in metadata["packages"]}
    pending = list(metadata["workspace_members"])
    seen = set()
    while pending:
        package_id = pending.pop()
        if package_id in seen:
            continue
        seen.add(package_id)
        for dependency in nodes[package_id]["deps"]:
            if any(kind["kind"] is None for kind in dependency["dep_kinds"]):
                pending.append(dependency["pkg"])
    result = []
    for package_id in seen - set(metadata["workspace_members"]):
        package = packages[package_id]
        directory = Path(package["manifest_path"]).parent
        license_file = package.get("license_file")
        result.append(
            Package(
                name=package["name"],
                version=package["version"],
                license=package.get("license") or "",
                repository=package.get("repository") or f"https://crates.io/crates/{package['name']}",
                authors=package.get("authors") or [],
                directory=directory,
                license_file=directory / license_file if license_file else None,
            )
        )
    return sorted(result, key=lambda package: (package.name, package.version))


# Report
def rust_std_package():
    return Package(
        name="Rust standard library",
        version="",
        license="MIT OR Apache-2.0",
        repository="https://github.com/rust-lang/rust",
        authors=["The Rust Project Contributors"],
        directory=Path("<rust-std>"),
    )


def build_sections(packages):
    sections = {}
    errors = []
    missing = []
    for package in packages:
        if not package.license:
            errors.append(f"{package.name} {package.version}: no license declared")
            continue
        try:
            chosen = choose_licenses(package.license)
        except LicenseError as error:
            errors.append(f"{package.name} {package.version}: {error}")
            continue
        for license_id in chosen:
            text = find_text(package, license_id)
            if text is None:
                missing.append((package, license_id))
                continue
            key = (license_id, text)
            section = sections.setdefault(key, Section(license=license_id, text=text))
            section.packages.append(package)
            for notice in notice_files(package):
                notice_text = read_text(notice)
                if notice_text not in section.notices:
                    section.notices.append(notice_text)
    for package, license_id in missing:
        if license_id in NO_NOTICE_LICENSES:
            text = NO_NOTICE_TEXT.format(license=license_id, url=LICENSE_URL.format(license=license_id))
            sections.setdefault((license_id, text), Section(license=license_id, text=text)).packages.append(package)
            continue
        shared = [section for (section_license, _), section in sections.items() if section_license == license_id]
        if license_id == "Apache-2.0" and shared:
            max(shared, key=lambda section: len(section.packages)).packages.append(package)
            continue
        errors.append(f"{package.name} {package.version}: no {license_id} license text found")
    if errors:
        raise LicenseError("\n".join(errors))
    for section in sections.values():
        section.packages.sort(key=lambda package: (package.name, package.version))
    return sorted(sections.values(), key=lambda section: (ALLOWED_LICENSES.index(section.license), section.packages[0].name))


def package_label(package):
    return f"{package.name} {package.version}".strip()


def render(title, sections, source_notes):
    lines = [
        f"{title} - Third-party licenses",
        "=" * len(f"{title} - Third-party licenses"),
        "",
        "This program contains the following third-party components.",
        'For components offered under a choice of licenses (for example "MIT OR Apache-2.0"),',
        "the license listed here is the one chosen.",
        "",
    ]
    for section in sections:
        lines.append(RULE)
        lines.append(", ".join(package_label(package) for package in section.packages))
        lines.append(f"License: {section.license}")
        sources = sorted({package.repository for package in section.packages})
        for index, source in enumerate(sources):
            lines.append(f"{'Source:' if index == 0 else '       '} {source}")
        for package in section.packages:
            note = source_notes.get(package.name)
            if note:
                lines.append(f"Note:   {package.name}: {note}")
        lines.append("")
        lines.append(section.text.rstrip())
        for notice in section.notices:
            lines.append("")
            lines.append(notice.rstrip())
        lines.append("")
    return "\n".join(lines).rstrip() + "\n"


# Main
def main(arguments=None):
    parser = argparse.ArgumentParser(description="Check and collect the licenses of third-party Rust crates.")
    parser.add_argument("--manifest-path", type=Path, default=Path("Cargo.toml"))
    parser.add_argument("--platform", action="append", default=[])
    parser.add_argument("--title", default="This program")
    parser.add_argument("--output", type=Path)
    parser.add_argument("--rust-std", action="store_true")
    parser.add_argument("--source-note", action="append", default=[], metavar="CRATE=TEXT")
    parser.add_argument("--append", action="append", default=[], type=Path, metavar="FILE")
    options = parser.parse_args(arguments)

    source_notes = dict(note.split("=", 1) for note in options.source_note)
    packages = runtime_packages(cargo_metadata(options.manifest_path, options.platform))
    if options.rust_std:
        packages.append(rust_std_package())
    try:
        sections = build_sections(packages)
    except LicenseError as error:
        print(f"License check failed:\n{error}", file=sys.stderr)
        return 1

    if options.output:
        options.output.parent.mkdir(parents=True, exist_ok=True)
        report = render(options.title, sections, source_notes)
        for appended in options.append:
            report += "\n" + "=" * 72 + "\n\n" + read_text(appended)
        options.output.write_text(report, encoding="utf-8", newline="\n")
    print(f"License check passed: {len(packages)} components")
    return 0


if __name__ == "__main__":
    sys.exit(main())
