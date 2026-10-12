use std::path::PathBuf;
use std::process::Command;

fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        link_clang_runtime();
    }
    tauri_build::build()
}

/// ggml's Metal code asks for the macOS version with `@available`, which calls
/// `__isPlatformVersionAtLeast` from clang's own runtime library. rustc links without the C
/// compiler's default libraries, and the release build's LTO drops Rust std's copy of that
/// function, so the release link failed on Apple Silicon. whisper-rs-sys linked the library
/// the same way before 0.7.
fn link_clang_runtime() {
    let Ok(output) = Command::new("clang").arg("--print-resource-dir").output() else {
        return;
    };
    let dir = PathBuf::from(String::from_utf8_lossy(&output.stdout).trim()).join("lib/darwin");
    if dir.join("libclang_rt.osx.a").is_file() {
        println!("cargo:rustc-link-search=native={}", dir.display());
        println!("cargo:rustc-link-lib=clang_rt.osx");
    }
}
