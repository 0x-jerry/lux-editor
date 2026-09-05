use std::env;

fn main() {
    if env::var("CARGO_CFG_WINDOWS").is_err() {
        return;
    }
    // muda/tray-icon link TaskDialogIndirect & friend only in ComCtl32 v6;
    // the v6 side-by-side assembly must be requested by the binary's manifest.
    let manifest = env::var("CARGO_MANIFEST_DIR").unwrap() + "\\assets\\lux-editor.manifest";
    println!("cargo::rustc-link-arg=/MANIFEST:EMBED");
    println!("cargo::rustc-link-arg=/MANIFESTINPUT:{manifest}");
}