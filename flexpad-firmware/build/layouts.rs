use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;
use std::fs;
use std::path::{Path, PathBuf};

#[allow(dead_code)]
#[path = "../src/keys.rs"]
pub mod keys;

// Constants

pub const DIRECTORY: &str = "../layouts";
pub const EXTENSION: &str = "layout";
pub const OUTPUT_FILE: &str = "layouts.rs";
pub const DEFAULT_CODE: &str = "us";
pub const TITLE_PREFIX: &str = "# FlexPad keyboard layout: ";
pub const REVIEWED_PREFIX: &str = "# Reviewed by: ";
pub const NAME_KEY: &str = "layout.name";
pub const FALLBACK_KEY: &str = "layout.fallback";
const META_PREFIX: &str = "layout.";
const DEAD_PREFIX: &str = "dead:";
const CODE_POINT_PREFIX: &str = "U+";
const MAX_CODE_LENGTH: usize = 16;
const SHIFT: u8 = 0x02;
const ALT_GR: u8 = 0x40;
const MODIFIERS: [(&str, u8); 2] = [("Shift", SHIFT), ("AltGr", ALT_GR)];
const BUILT_IN: [(char, &str); 3] = [(' ', "Space"), ('\n', "Enter"), ('\t', "Tab")];

// Types

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Stroke {
    pub modifiers: u8,
    pub usage: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Keystrokes {
    pub dead_key: Option<Stroke>,
    pub key: Stroke,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutFile {
    pub code: String,
    pub title: Option<String>,
    pub reviewers: Vec<String>,
    pub name: String,
    pub fallback: Option<String>,
    pub labels: BTreeMap<String, String>,
    pub characters: BTreeMap<char, Keystrokes>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layout {
    pub code: String,
    pub name: String,
    pub labels: BTreeMap<String, String>,
    pub characters: BTreeMap<char, Keystrokes>,
}

// Build

pub fn generate(manifest_dir: &Path, out_dir: &Path) {
    let directory = manifest_dir.join(DIRECTORY);
    println!("cargo:rerun-if-changed={}", directory.display());
    let paths = layout_paths(&directory);
    for path in &paths {
        println!("cargo:rerun-if-changed={}", path.display());
    }
    let files = paths
        .iter()
        .map(|path| read_file(path))
        .collect::<Result<Vec<_>, _>>()
        .unwrap_or_else(|error| panic!("{error}"));
    let layouts = resolve(&files).unwrap_or_else(|error| panic!("{error}"));
    fs::write(out_dir.join(OUTPUT_FILE), source(&layouts))
        .expect("writing the keyboard layouts failed");
}

pub fn layout_paths(directory: &Path) -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("reading {} failed: {error}", directory.display()))
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == EXTENSION)
        })
        .collect();
    paths.sort();
    paths
}

pub fn read_file(path: &Path) -> Result<LayoutFile, String> {
    let code = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .ok_or_else(|| format!("{} has no valid name", path.display()))?;
    let content = fs::read_to_string(path)
        .map_err(|error| format!("reading {} failed: {error}", path.display()))?;
    parse(code, &content).map_err(|error| format!("{}: {error}", path.display()))
}

// Parsing

pub fn parse(code: &str, content: &str) -> Result<LayoutFile, String> {
    if !is_valid_code(code) {
        return Err(format!("{code:?} is not a valid layout code"));
    }
    let mut file = LayoutFile {
        code: code.to_owned(),
        title: None,
        reviewers: Vec::new(),
        name: String::new(),
        fallback: None,
        labels: BTreeMap::new(),
        characters: BTreeMap::new(),
    };
    let mut strokes = BTreeMap::new();
    for (index, line) in content.trim_start_matches('\u{feff}').lines().enumerate() {
        let number = index + 1;
        let line = line.trim();
        if let Some(title) = line.strip_prefix(TITLE_PREFIX) {
            file.title = Some(title.trim().to_owned());
        }
        if let Some(reviewers) = line.strip_prefix(REVIEWED_PREFIX) {
            file.reviewers = reviewers
                .split(',')
                .filter_map(|name| name.trim().strip_prefix('@'))
                .map(str::to_owned)
                .collect();
        }
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (key, value) = split_line(line).ok_or_else(|| format!("line {number} has no \"=\""))?;
        if key.is_empty() || value.is_empty() {
            return Err(format!("line {number} needs a key and a value"));
        }
        let error = |message: String| format!("line {number}: {message}");
        if let Some(character) = character_key(key).map_err(error)? {
            let strokes_value = keystrokes(value).map_err(error)?;
            if file.characters.insert(character, strokes_value).is_some() {
                return Err(error(format!("{character:?} is defined twice")));
            }
            if let Some(other) = strokes.insert(strokes_value, character) {
                return Err(error(format!(
                    "{character:?} uses the same keys as {other:?}"
                )));
            }
        } else if key.starts_with(META_PREFIX) {
            match key {
                NAME_KEY => file.name = value.to_owned(),
                FALLBACK_KEY => file.fallback = Some(value.to_owned()),
                _ => return Err(error(format!("unknown setting {key}"))),
            }
        } else {
            keys::key_from_name(key).ok_or_else(|| error(format!("unknown key {key}")))?;
            if file
                .labels
                .insert(key.to_owned(), value.to_owned())
                .is_some()
            {
                return Err(error(format!("the label of {key} is defined twice")));
            }
        }
    }
    if file.name.is_empty() {
        return Err(format!("{NAME_KEY} is missing"));
    }
    Ok(file)
}

fn split_line(line: &str) -> Option<(&str, &str)> {
    let first = line.chars().next()?.len_utf8();
    let position = first + line[first..].find('=')?;
    Some((line[..position].trim(), line[position + 1..].trim()))
}

fn character_key(key: &str) -> Result<Option<char>, String> {
    if let Some(hex) = key.strip_prefix(CODE_POINT_PREFIX) {
        return u32::from_str_radix(hex, 16)
            .ok()
            .and_then(char::from_u32)
            .map(Some)
            .ok_or_else(|| format!("{key} is not a valid character"));
    }
    let mut characters = key.chars();
    Ok(match (characters.next(), characters.next()) {
        (Some(character), None) => Some(character),
        _ => None,
    })
}

fn keystrokes(value: &str) -> Result<Keystrokes, String> {
    match value.strip_prefix(DEAD_PREFIX) {
        Some(rest) => {
            let mut parts = rest.split_whitespace();
            let (Some(dead_key), Some(key), None) = (parts.next(), parts.next(), parts.next())
            else {
                return Err(format!("{value:?} needs a dead key and one key after it"));
            };
            Ok(Keystrokes {
                dead_key: Some(stroke(dead_key)?),
                key: stroke(key)?,
            })
        }
        None => Ok(Keystrokes {
            dead_key: None,
            key: stroke(value)?,
        }),
    }
}

fn stroke(value: &str) -> Result<Stroke, String> {
    let mut parts: Vec<&str> = value.split('+').map(str::trim).collect();
    let name = parts.pop().unwrap_or_default();
    let mut modifiers = 0;
    for part in parts {
        let (_, modifier) = MODIFIERS
            .iter()
            .find(|(known, _)| known.eq_ignore_ascii_case(part))
            .ok_or_else(|| format!("unknown modifier {part:?} in {value:?}"))?;
        modifiers |= modifier;
    }
    match keys::key_from_name(name) {
        Some(keys::Key::Usage(usage)) => Ok(Stroke { modifiers, usage }),
        Some(keys::Key::Modifier(_)) => Err(format!("{name} is a modifier and types no character")),
        None => Err(format!("unknown key {name:?} in {value:?}")),
    }
}

pub fn is_valid_code(code: &str) -> bool {
    !code.is_empty()
        && code.len() <= MAX_CODE_LENGTH
        && code
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-')
}

// Resolution

pub fn resolve(files: &[LayoutFile]) -> Result<Vec<Layout>, String> {
    let mut codes = BTreeSet::new();
    for file in files {
        if !codes.insert(file.code.to_ascii_lowercase()) {
            return Err(format!("the layout {} exists twice", file.code));
        }
    }
    if !files.iter().any(|file| file.code == DEFAULT_CODE) {
        return Err(format!("the default layout {DEFAULT_CODE} is missing"));
    }
    let mut layouts: Vec<Layout> = files
        .iter()
        .map(|file| resolve_layout(files, file, &mut Vec::new()))
        .collect::<Result<_, _>>()?;
    layouts.sort_by(|left, right| left.code.cmp(&right.code));
    Ok(layouts)
}

fn resolve_layout<'a>(
    files: &'a [LayoutFile],
    file: &'a LayoutFile,
    visited: &mut Vec<&'a str>,
) -> Result<Layout, String> {
    if visited.contains(&file.code.as_str()) {
        return Err(format!(
            "the fallback of {} leads back to itself",
            file.code
        ));
    }
    visited.push(&file.code);
    let mut layout = Layout {
        code: file.code.clone(),
        name: file.name.clone(),
        labels: file.labels.clone(),
        characters: file.characters.clone(),
    };
    add_uppercase_letters(&mut layout.characters);
    for (character, name) in BUILT_IN {
        add_unused(&mut layout.characters, character, keystrokes(name)?);
    }
    if let Some(code) = &file.fallback {
        let fallback = files
            .iter()
            .find(|other| other.code.eq_ignore_ascii_case(code))
            .ok_or_else(|| format!("{} falls back to the unknown layout {code}", file.code))?;
        let fallback = resolve_layout(files, fallback, visited)?;
        for (key, label) in fallback.labels {
            layout.labels.entry(key).or_insert(label);
        }
        for (character, strokes) in fallback.characters {
            add_unused(&mut layout.characters, character, strokes);
        }
    }
    visited.pop();
    Ok(layout)
}

fn add_uppercase_letters(characters: &mut BTreeMap<char, Keystrokes>) {
    let lowercase: Vec<(char, Keystrokes)> = characters
        .iter()
        .filter(|(character, strokes)| character.is_lowercase() && strokes.key.modifiers == 0)
        .map(|(character, strokes)| (*character, *strokes))
        .collect();
    for (character, strokes) in lowercase {
        let mut uppercase = character.to_uppercase();
        if let (Some(upper), None) = (uppercase.next(), uppercase.next())
            && upper != character
        {
            let shifted = Keystrokes {
                dead_key: strokes.dead_key,
                key: Stroke {
                    modifiers: SHIFT,
                    usage: strokes.key.usage,
                },
            };
            add_unused(characters, upper, shifted);
        }
    }
}

fn add_unused(characters: &mut BTreeMap<char, Keystrokes>, character: char, strokes: Keystrokes) {
    if !characters.contains_key(&character) && !characters.values().any(|used| *used == strokes) {
        characters.insert(character, strokes);
    }
}

// Source

pub fn source(layouts: &[Layout]) -> String {
    let default = layouts
        .iter()
        .position(|layout| layout.code == DEFAULT_CODE)
        .expect("the default layout exists");
    let mut text = String::new();
    let _ = writeln!(
        text,
        "pub const DEFAULT: KeyboardLayout = KeyboardLayout({default});\n"
    );
    let all: Vec<String> = (0..layouts.len())
        .map(|index| format!("KeyboardLayout({index})"))
        .collect();
    let _ = writeln!(
        text,
        "pub const ALL: &[KeyboardLayout] = &[{}];\n",
        all.join(", ")
    );
    text.push_str("const LAYOUTS: &[Layout] = &[\n");
    for layout in layouts {
        let _ = writeln!(
            text,
            "    Layout {{\n        code: {:?},\n        name: {:?},\n        labels: &[",
            layout.code, layout.name
        );
        for (key, label) in &layout.labels {
            let _ = writeln!(text, "            ({key:?}, {label:?}),");
        }
        text.push_str("        ],\n        characters: &[\n");
        for (character, strokes) in &layout.characters {
            let dead_key = strokes.dead_key.map_or_else(
                || "None".to_owned(),
                |dead_key| format!("Some({})", stroke_source(dead_key)),
            );
            let _ = writeln!(
                text,
                "            ({character:?}, Keystrokes {{ dead_key: {dead_key}, key: {} }}),",
                stroke_source(strokes.key)
            );
        }
        text.push_str("        ],\n    },\n");
    }
    text.push_str("];\n");
    text
}

fn stroke_source(stroke: Stroke) -> String {
    format!("stroke(0x{:02X}, 0x{:02X})", stroke.modifiers, stroke.usage)
}
