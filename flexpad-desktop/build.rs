#[path = "src/icon.rs"]
#[allow(dead_code)]
mod icon;

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

// Constants
const FIRMWARE_IMAGE_VARIABLE: &str = "FLEXPAD_FIRMWARE_IMAGE";
const ICON_SIZES: [u16; 7] = [16, 20, 24, 32, 48, 64, 256];
const RT_ICON: u16 = 3;
const RT_GROUP_ICON: u16 = 14;
const RT_VERSION: u16 = 16;
const RT_MANIFEST: u16 = 24;
const LANGUAGE_EN_US: u16 = 0x0409;
const CODE_PAGE_UNICODE: u16 = 1200;
const MEMORY_FLAGS: u16 = 0x1030;
const ICNS_ENTRIES: [(&[u8; 4], u16); 6] = [
    (b"icp4", 16),
    (b"icp5", 32),
    (b"icp6", 64),
    (b"ic07", 128),
    (b"ic08", 256),
    (b"ic09", 512),
];
const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1A, b'\n'];
const MAX_STORED_BLOCK: usize = 65_535;
const LINUX_ICON_SIZES: [u16; 8] = [16, 24, 32, 48, 64, 128, 256, 512];
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
    println!("cargo:rerun-if-changed=src/icon.rs");
    println!("cargo:rerun-if-changed=Cargo.toml");
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let is_msvc = env::var("CARGO_CFG_TARGET_ENV").is_ok_and(|target_env| target_env == "msvc");
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR is set by Cargo"));
    embed_firmware_image(&out_dir);
    export_firmware_version();
    if target_os == "macos"
        && let Some(profile_dir) = out_dir.ancestors().nth(3)
    {
        fs::write(profile_dir.join("FlexPad.icns"), icns_file())
            .expect("writing the icon file failed");
    }
    if target_os == "linux"
        && let Some(profile_dir) = out_dir.ancestors().nth(3)
    {
        let icons = profile_dir.join("icons");
        fs::create_dir_all(&icons).expect("creating the icon folder failed");
        for size in LINUX_ICON_SIZES {
            fs::write(icons.join(format!("{size}.png")), png_image(size))
                .expect("writing an icon failed");
        }
    }
    if target_os != "windows" || !is_msvc {
        return;
    }
    let path = out_dir.join("flexpad.res");
    fs::write(&path, resources()).expect("writing the resource file failed");
    println!("cargo:rustc-link-arg-bins={}", path.display());
    if let Some(profile_dir) = out_dir.ancestors().nth(3) {
        fs::write(profile_dir.join("FlexPad.ico"), icon_file())
            .expect("writing the icon file failed");
    }
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

// Icon file
fn icon_file() -> Vec<u8> {
    let images: Vec<(u16, Vec<u8>)> = ICON_SIZES
        .into_iter()
        .map(|size| (size, icon_image(size)))
        .collect();
    let mut file = Vec::new();
    push_u16(&mut file, 0);
    push_u16(&mut file, 1);
    push_u16(&mut file, u16::try_from(images.len()).unwrap_or(0));
    let mut offset = 6 + 16 * images.len();
    for (size, image) in &images {
        let dimension = u8::try_from(*size).unwrap_or(0);
        file.extend_from_slice(&[dimension, dimension, 0, 0]);
        push_u16(&mut file, 1);
        push_u16(&mut file, 32);
        push_u32(&mut file, u32::try_from(image.len()).unwrap_or(0));
        push_u32(&mut file, u32::try_from(offset).unwrap_or(0));
        offset += image.len();
    }
    for (_, image) in &images {
        file.extend_from_slice(image);
    }
    file
}

// macOS icon
fn icns_file() -> Vec<u8> {
    let mut entries = Vec::new();
    for (kind, size) in ICNS_ENTRIES {
        let png = png_image(size);
        entries.extend_from_slice(kind);
        entries.extend_from_slice(&u32::try_from(png.len() + 8).unwrap_or(0).to_be_bytes());
        entries.extend_from_slice(&png);
    }
    let mut file = b"icns".to_vec();
    file.extend_from_slice(&u32::try_from(entries.len() + 8).unwrap_or(0).to_be_bytes());
    file.extend_from_slice(&entries);
    file
}

fn png_image(size: u16) -> Vec<u8> {
    let rgba = icon::icon_pixels(size);
    let row = usize::from(size) * 4;
    let mut raw = Vec::with_capacity((row + 1) * usize::from(size));
    for line in rgba.chunks(row) {
        raw.push(0);
        raw.extend_from_slice(line);
    }
    let mut header = Vec::new();
    header.extend_from_slice(&u32::from(size).to_be_bytes());
    header.extend_from_slice(&u32::from(size).to_be_bytes());
    header.extend_from_slice(&[8, 6, 0, 0, 0]);
    let mut png = PNG_SIGNATURE.to_vec();
    png_chunk(&mut png, *b"IHDR", &header);
    png_chunk(&mut png, *b"IDAT", &zlib_stored(&raw));
    png_chunk(&mut png, *b"IEND", &[]);
    png
}

fn png_chunk(png: &mut Vec<u8>, kind: [u8; 4], data: &[u8]) {
    png.extend_from_slice(&u32::try_from(data.len()).unwrap_or(0).to_be_bytes());
    let start = png.len();
    png.extend_from_slice(&kind);
    png.extend_from_slice(data);
    let checksum = crc32(&png[start..]);
    png.extend_from_slice(&checksum.to_be_bytes());
}

fn zlib_stored(data: &[u8]) -> Vec<u8> {
    let mut output = vec![0x78, 0x01];
    let blocks: Vec<&[u8]> = data.chunks(MAX_STORED_BLOCK).collect();
    for (index, block) in blocks.iter().enumerate() {
        output.push(u8::from(index + 1 == blocks.len()));
        let length = u16::try_from(block.len()).unwrap_or(u16::MAX);
        output.extend_from_slice(&length.to_le_bytes());
        output.extend_from_slice(&(!length).to_le_bytes());
        output.extend_from_slice(block);
    }
    output.extend_from_slice(&adler32(data).to_be_bytes());
    output
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFF_u32;
    for byte in data {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

fn adler32(data: &[u8]) -> u32 {
    let (mut first, mut second) = (1_u32, 0_u32);
    for byte in data {
        first = (first + u32::from(*byte)) % 65_521;
        second = (second + first) % 65_521;
    }
    (second << 16) | first
}

// Resources
fn resources() -> Vec<u8> {
    let mut file = Vec::new();
    push_resource(&mut file, 0, 0, &[], 0, 0);
    let mut group = Vec::new();
    push_u16(&mut group, 0);
    push_u16(&mut group, 1);
    push_u16(&mut group, u16::try_from(ICON_SIZES.len()).unwrap_or(0));
    for (index, size) in ICON_SIZES.into_iter().enumerate() {
        let id = u16::try_from(index + 1).unwrap_or(u16::MAX);
        let image = icon_image(size);
        let dimension = u8::try_from(size).unwrap_or(0);
        group.extend_from_slice(&[dimension, dimension, 0, 0]);
        push_u16(&mut group, 1);
        push_u16(&mut group, 32);
        push_u32(&mut group, u32::try_from(image.len()).unwrap_or(0));
        push_u16(&mut group, id);
        push_resource(&mut file, RT_ICON, id, &image, MEMORY_FLAGS, LANGUAGE_EN_US);
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

fn icon_image(size: u16) -> Vec<u8> {
    let rgba = icon::icon_pixels(size);
    let width = usize::from(size);
    let mask_row = width.div_ceil(32) * 4;
    let mut image = Vec::new();
    push_u32(&mut image, 40);
    push_u32(&mut image, u32::from(size));
    push_u32(&mut image, u32::from(size) * 2);
    push_u16(&mut image, 1);
    push_u16(&mut image, 32);
    push_u32(&mut image, 0);
    push_u32(
        &mut image,
        u32::try_from(width * width * 4 + mask_row * width).unwrap_or(0),
    );
    image.extend_from_slice(&[0; 16]);
    for row in rgba.chunks(width * 4).rev() {
        for pixel in row.chunks(4) {
            image.extend_from_slice(&[pixel[2], pixel[1], pixel[0], pixel[3]]);
        }
    }
    image.resize(image.len() + mask_row * width, 0);
    image
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
