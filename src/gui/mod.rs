//! Tauri GUI entry point, compiled only when the `gui` feature is enabled.
//!
//! # Example
//!
//! ```no_run
//! #[cfg(feature = "gui")]
//! easy_diff::gui::run();
//! ```
//!
//! This module owns the `tauri::Builder` setup and exposes a `run()` function
//! that `main.rs` calls when the binary is launched in GUI mode.  All Tauri
//! command handlers will be registered here in later PRs; this PR provides the
//! minimal scaffold needed to make the window appear.

/// Launch the Tauri GUI application.
///
/// # Panics
///
/// Panics if Tauri cannot initialize the application (e.g. missing WebView
/// runtime).  This mirrors the idiomatic Tauri pattern of calling `.expect()`
/// on the final `run()` call.
pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
