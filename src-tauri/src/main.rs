// Prevents additional console window on Windows in release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    red_green_light_lib::run()
}
