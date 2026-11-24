use std::env;
use std::path::PathBuf;

#[path = "build/layouts.rs"]
mod layouts;

// Build

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=build/layouts.rs");
    println!("cargo:rerun-if-changed=src/keys.rs");
    let manifest_dir =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is set by Cargo"));
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR is set by Cargo"));
    layouts::generate(&manifest_dir, &out_dir);
    let target = env::var("TARGET").unwrap_or_default();
    if target.starts_with("xtensa-") {
        println!("cargo:rustc-link-arg=-Tlinkall.x");
    }
}
