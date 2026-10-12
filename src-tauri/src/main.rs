// The binary target is a shim: everything lives in the library so the crate can
// also be loaded as a library, which is how Tauri hosts an application on the
// platforms that require it. windows_subsystem keeps the console window from
// appearing behind the app in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    fm_radio::run()
}
