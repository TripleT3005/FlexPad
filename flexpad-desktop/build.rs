use std::env;
use std::fmt::Write;
use std::fs;
use std::path::{Path, PathBuf};

#[path = "../flexpad-firmware/build/layouts.rs"]
mod layouts;

// Constants
const FIRMWARE_IMAGE_VARIABLE: &str = "FLEXPAD_FIRMWARE_IMAGE";
const ICON_DIRECTORY: &str = "../assets/icons";
const LANGUAGE_DIRECTORY: &str = "lang";
const LANGUAGE_EXTENSION: &str = "lang";
const ENGLISH_FILE: &str = "en.lang";
const COMPONENT_DIRECTORY: &str = "components";
const COMPONENT_EXTENSION: &str = "json";
const COMPONENT_TEXT_PREFIXES: [&str; 4] = ["component.", "pin.", "function.", "parameter."];
const RT_ICON: u16 = 3;
const RT_GROUP_ICON: u16 = 14;
const RT_VERSION: u16 = 16;
const RT_MANIFEST: u16 = 24;
const LANGUAGE_EN_US: u16 = 0x0409;
const CODE_PAGE_UNICODE: u16 = 1200;
const MEMORY_FLAGS: u16 = 0x1030;
const ICNS_ENTRIES: [(&[u8; 4], u16); 5] = [
    (b"icp4", 16),
    (b"icp5", 32),
    (b"icp6", 64),
    (b"ic07", 128),
    (b"ic08", 256),
];
const MANIFEST: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <application xmlns="urn:schemas-microsoft-com:asm.v3">
    <windowsSettings>
      <dpiAware xmlns="http://schemas.microsoft.com/SMI/2005/WindowsSettings">true/pm</dpiAware>
      <dpiAwareness xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">PerMonitorV2</dpiAwareness>
    </windowsSettings>
  </application>
</assembly>
"#;

// Build
fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={ICON_DIRECTORY}");
    println!("cargo:rerun-if-changed=Cargo.toml");
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let is_msvc = env::var("CARGO_CFG_TARGET_ENV").is_ok_and(|target_env| target_env == "msvc");
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR is set by Cargo"));
    embed_firmware_image(&out_dir);
    embed_languages(&out_dir);
    embed_components(&out_dir);
    embed_layouts(&out_dir);
    export_firmware_version();
    if target_os == "macos" {
        fs::write(out_dir.join("FlexPad.icns"), icns_file()).expect("writing the icon file failed");
    }
    if target_os != "windows" || !is_msvc {
        return;
    }
    let path = out_dir.join("flexpad.res");
    fs::write(&path, resources()).expect("writing the resource file failed");
    println!("cargo:rustc-link-arg-bins={}", path.display());
}

// Keyboard layouts
fn embed_layouts(out_dir: &Path) {
    println!("cargo:rerun-if-changed=../flexpad-firmware/build/layouts.rs");
    println!("cargo:rerun-if-changed=../flexpad-firmware/src/keys.rs");
    let manifest_dir =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is set by Cargo"));
    layouts::generate(&manifest_dir, out_dir);
}

// Firmware version
fn export_firmware_version() {
    let manifest = "../flexpad-firmware/Cargo.toml";
    println!("cargo:rerun-if-changed={manifest}");
    let content = fs::read_to_string(manifest).expect("reading the firmware manifest failed");
    let version = content
        .lines()
        .skip_while(|line| line.trim() != "[package]")
        .find_map(|line| {
            line.trim()
                .strip_prefix("version")?
                .trim_start()
                .strip_prefix('=')?
                .trim()
                .strip_prefix('"')?
                .strip_suffix('"')
                .map(str::to_string)
        })
        .expect("the firmware manifest has no package version");
    println!("cargo:rustc-env=FLEXPAD_FIRMWARE_VERSION={version}");
}

// Bundled files
fn embed_directory(out_dir: &Path, directory: &str, extension: &str, output: &str) -> PathBuf {
    println!("cargo:rerun-if-changed={directory}");
    let manifest_dir =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is set by Cargo"));
    let directory = manifest_dir.join(directory);
    let mut files: Vec<PathBuf> = fs::read_dir(&directory)
        .unwrap_or_else(|error| panic!("reading {} failed: {error}", directory.display()))
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|found| found == extension))
        .collect();
    files.sort();
    let mut bundled = String::from("pub const BUNDLED: &[(&str, &str)] = &[\n");
    for path in &files {
        println!("cargo:rerun-if-changed={}", path.display());
        let name = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or_else(|| panic!("{} has no valid name", path.display()));
        let _ = writeln!(
            bundled,
            "    ({name:?}, include_str!({:?})),",
            path.display().to_string()
        );
    }
    bundled.push_str("];\n");
    fs::write(out_dir.join(output), bundled)
        .unwrap_or_else(|error| panic!("writing {output} failed: {error}"));
    directory
}

// Components
fn embed_components(out_dir: &Path) {
    embed_directory(
        out_dir,
        COMPONENT_DIRECTORY,
        COMPONENT_EXTENSION,
        "components.rs",
    );
}

// Languages
fn embed_languages(out_dir: &Path) {
    let directory = embed_directory(
        out_dir,
        LANGUAGE_DIRECTORY,
        LANGUAGE_EXTENSION,
        "languages.rs",
    );

    let english = fs::read_to_string(directory.join(ENGLISH_FILE))
        .expect("reading the English language file failed");
    let mut keys = String::new();
    for key in english
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#') && !line.starts_with(';'))
        .filter_map(|line| line.split_once('='))
        .map(|(key, _)| key.trim())
        .filter(|key| {
            !COMPONENT_TEXT_PREFIXES
                .iter()
                .any(|prefix| key.starts_with(prefix))
        })
    {
        let name: String = key
            .chars()
            .map(|character| {
                if character.is_ascii_alphanumeric() {
                    character.to_ascii_uppercase()
                } else {
                    '_'
                }
            })
            .collect();
        let _ = writeln!(keys, "pub const {name}: &str = {key:?};");
    }
    fs::write(out_dir.join("text_keys.rs"), keys).expect("writing the text keys failed");
}

// Firmware image
fn embed_firmware_image(out_dir: &Path) {
    println!("cargo:rerun-if-env-changed={FIRMWARE_IMAGE_VARIABLE}");
    let image = match env::var_os(FIRMWARE_IMAGE_VARIABLE).filter(|path| !path.is_empty()) {
        Some(path) => {
            println!("cargo:rerun-if-changed={}", path.to_string_lossy());
            fs::read(&path).unwrap_or_else(|e| {
                panic!(
                    "reading the firmware image {} failed: {e}",
                    path.to_string_lossy()
                )
            })
        }
        None => Vec::new(),
    };
    fs::write(out_dir.join("firmware.bin"), image).expect("writing the firmware image failed");
}

// Icon
fn icon_images() -> Vec<([u8; 8], Vec<u8>)> {
    let file =
        fs::read(format!("{ICON_DIRECTORY}/icon.ico")).expect("reading the icon file failed");
    let read_u32 = |offset: usize| {
        let bytes = file
            .get(offset..offset + 4)
            .and_then(|bytes| <[u8; 4]>::try_from(bytes).ok())
            .expect("the icon file is truncated");
        usize::try_from(u32::from_le_bytes(bytes)).expect("the icon file is too large")
    };
    let count = file
        .get(4..6)
        .map(|bytes| usize::from(u16::from_le_bytes([bytes[0], bytes[1]])))
        .expect("the icon file has no header");
    assert!(count > 0, "the icon file contains no images");
    (0..count)
        .map(|index| {
            let entry = 6 + index * 16;
            let mut header = [0; 8];
            header.copy_from_slice(
                file.get(entry..entry + 8)
                    .expect("the icon file is truncated"),
            );
            let (length, offset) = (read_u32(entry + 8), read_u32(entry + 12));
            let image = file
                .get(offset..offset + length)
                .expect("the icon file is truncated")
                .to_vec();
            (header, image)
        })
        .collect()
}

// macOS icon
fn icns_file() -> Vec<u8> {
    let mut entries = Vec::new();
    for (kind, size) in ICNS_ENTRIES {
        let png =
            fs::read(format!("{ICON_DIRECTORY}/icon-{size}.png")).expect("reading an icon failed");
        entries.extend_from_slice(kind);
        entries.extend_from_slice(&u32::try_from(png.len() + 8).unwrap_or(0).to_be_bytes());
        entries.extend_from_slice(&png);
    }
    let mut file = b"icns".to_vec();
    file.extend_from_slice(&u32::try_from(entries.len() + 8).unwrap_or(0).to_be_bytes());
    file.extend_from_slice(&entries);
    file
}

// Resources
fn resources() -> Vec<u8> {
    let mut file = Vec::new();
    push_resource(&mut file, 0, 0, &[], 0, 0);
    let images = icon_images();
    let mut group = Vec::new();
    push_u16(&mut group, 0);
    push_u16(&mut group, 1);
    push_u16(&mut group, u16::try_from(images.len()).unwrap_or(0));
    for (index, (header, image)) in images.iter().enumerate() {
        let id = u16::try_from(index + 1).unwrap_or(u16::MAX);
        group.extend_from_slice(header);
        push_u32(&mut group, u32::try_from(image.len()).unwrap_or(0));
        push_u16(&mut group, id);
        push_resource(&mut file, RT_ICON, id, image, MEMORY_FLAGS, LANGUAGE_EN_US);
    }
    push_resource(
        &mut file,
        RT_GROUP_ICON,
        1,
        &group,
        MEMORY_FLAGS,
        LANGUAGE_EN_US,
    );
    push_resource(
        &mut file,
        RT_VERSION,
        1,
        &version_info(),
        MEMORY_FLAGS,
        LANGUAGE_EN_US,
    );
    push_resource(
        &mut file,
        RT_MANIFEST,
        1,
        MANIFEST.as_bytes(),
        MEMORY_FLAGS,
        LANGUAGE_EN_US,
    );
    file
}

fn push_resource(file: &mut Vec<u8>, kind: u16, id: u16, data: &[u8], flags: u16, language: u16) {
    push_u32(file, u32::try_from(data.len()).unwrap_or(0));
    push_u32(file, 32);
    for value in [0xFFFF, kind, 0xFFFF, id] {
        push_u16(file, value);
    }
    push_u32(file, 0);
    push_u16(file, flags);
    push_u16(file, language);
    push_u32(file, 0);
    push_u32(file, 0);
    file.extend_from_slice(data);
    align(file);
}

// Version
fn version_info() -> Vec<u8> {
    let version = env::var("CARGO_PKG_VERSION").expect("CARGO_PKG_VERSION is set by Cargo");
    let parts: Vec<u16> = version
        .split(['.', '-', '+'])
        .take(3)
        .map(|part| part.parse().unwrap_or(0))
        .collect();
    let part = |index: usize| u32::from(parts.get(index).copied().unwrap_or(0));
    let mut fixed = Vec::new();
    for value in [
        0xFEEF_04BD,
        0x0001_0000,
        (part(0) << 16) | part(1),
        part(2) << 16,
        (part(0) << 16) | part(1),
        part(2) << 16,
        0x3F,
        0,
        0x0004_0004,
        1,
        0,
        0,
        0,
    ] {
        push_u32(&mut fixed, value);
    }

    let strings = [
        ("CompanyName", "TripleT3005"),
        ("FileDescription", "FlexPad Configurator"),
        ("FileVersion", version.as_str()),
        ("InternalName", "FlexPad"),
        ("LegalCopyright", "Copyright (c) TripleT3005"),
        ("OriginalFilename", "FlexPad.exe"),
        ("ProductName", "FlexPad"),
        ("ProductVersion", version.as_str()),
    ];
    let entries: Vec<Vec<u8>> = strings
        .iter()
        .map(|(key, value)| {
            let text = utf16(value);
            block(
                key,
                &text,
                u16::try_from(text.len() / 2).unwrap_or(0),
                1,
                &[],
            )
        })
        .collect();
    let table = block(
        &format!("{LANGUAGE_EN_US:04x}{CODE_PAGE_UNICODE:04x}"),
        &[],
        0,
        1,
        &entries,
    );
    let string_info = block("StringFileInfo", &[], 0, 1, &[table]);
    let mut translation = Vec::new();
    push_u16(&mut translation, LANGUAGE_EN_US);
    push_u16(&mut translation, CODE_PAGE_UNICODE);
    let var_info = block(
        "VarFileInfo",
        &[],
        0,
        1,
        &[block("Translation", &translation, 4, 0, &[])],
    );
    block("VS_VERSION_INFO", &fixed, 52, 0, &[string_info, var_info])
}

fn block(key: &str, value: &[u8], value_length: u16, kind: u16, children: &[Vec<u8>]) -> Vec<u8> {
    let mut data = vec![0, 0];
    push_u16(&mut data, value_length);
    push_u16(&mut data, kind);
    data.extend_from_slice(&utf16(key));
    align(&mut data);
    data.extend_from_slice(value);
    for child in children {
        align(&mut data);
        data.extend_from_slice(child);
    }
    let length = u16::try_from(data.len()).unwrap_or(u16::MAX).to_le_bytes();
    data[0..2].copy_from_slice(&length);
    data
}

// Helpers
fn utf16(text: &str) -> Vec<u8> {
    text.encode_utf16()
        .chain(std::iter::once(0))
        .flat_map(u16::to_le_bytes)
        .collect()
}

fn align(data: &mut Vec<u8>) {
    while !data.len().is_multiple_of(4) {
        data.push(0);
    }
}

fn push_u16(data: &mut Vec<u8>, value: u16) {
    data.extend_from_slice(&value.to_le_bytes());
}

fn push_u32(data: &mut Vec<u8>, value: u32) {
    data.extend_from_slice(&value.to_le_bytes());
}
