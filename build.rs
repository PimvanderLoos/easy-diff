/// Build script: runs `tauri_build::build()` when the `gui` feature is active
/// so that `tauri::generate_context!()` in `src/gui/mod.rs` can locate and
/// embed `tauri.conf.json` at compile time.
fn main() {
    // tauri-build is only included as a dependency when the "gui" feature is
    // enabled, so this call is only reachable in that configuration.
    #[cfg(feature = "gui")]
    tauri_build::build();
}
