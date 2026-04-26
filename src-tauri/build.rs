// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "console")]

fn main() {
    tauri_build::build()
}