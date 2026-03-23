/// bruecke build script
///
/// Runs wasm-pack ONLY when assets/bruecke_bg.wasm is missing (fallback for
/// `cargo install` / fresh clones). build.sh handles the normal dev case and
/// copies the WASM to assets/ before cargo runs, so build.rs skips it.
fn main() {
    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed=assets/bruecke_bg.wasm");

    // Skip when compiling the library itself to wasm32.
    if std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default() == "wasm32" {
        return;
    }

    // If assets/bruecke_bg.wasm already exists, nothing to do.
    if std::path::Path::new("assets/bruecke_bg.wasm").exists() {
        return;
    }

    // Fallback: try wasm-pack (not available on crates.io, that's fine —
    // the committed assets/bruecke_bg.wasm is used there).
    let ok = std::process::Command::new("wasm-pack")
        .args(["build", "--target", "web", "--release"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if ok {
        std::fs::create_dir_all("assets").ok();
        std::fs::copy("pkg/bruecke_bg.wasm", "assets/bruecke_bg.wasm").ok();
    }
}
