// Binary entrypoint.  On desktop, this is unused; the actual
// entrypoint is `keepsake_app_lib::run()` invoked from
// `main.rs`.  This file is required by Tauri 2.x convention
// so that `cargo tauri build` finds the right crate.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    keepsake_app_lib::run();
}
