use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn linux_multiarch_gnu() -> Option<&'static str> {
    let os = std::env::var("CARGO_CFG_TARGET_OS").ok()?;
    let arch = std::env::var("CARGO_CFG_TARGET_ARCH").ok()?;
    let env_abi = std::env::var("CARGO_CFG_TARGET_ENV").ok()?;

    match (arch.as_str(), os.as_str(), env_abi.as_str()) {
        ("x86_64", "linux", "gnu") => Some("x86_64-linux-gnu"),
        ("aarch64", "linux", "gnu") => Some("aarch64-linux-gnu"),
        _ => None,
    }
}

fn main() {
    // .so location and linker flag
    println!(
        "cargo:rustc-link-search=native=/opt/nvidia/vpi3/lib/{}",
        linux_multiarch_gnu().expect("Unsupported arch")
    );
    println!("cargo:rustc-link-lib=dylib=nvvpi");

    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        .clang_macro_fallback()
        .clang_arg("-I/opt/nvidia/vpi3/include")
        .clang_arg("-I/usr/local/cuda/include")
        // Tell cargo to invalidate the built crate whenever any of the
        // included header files changed.
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .default_macro_constant_type(bindgen::MacroTypeVariation::Unsigned)
        // Finish the builder and generate the bindings.
        .generate()
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
