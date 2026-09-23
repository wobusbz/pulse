// Release builds are double-clicked from Explorer, so they must be a GUI
// subsystem binary — otherwise Windows opens a console window alongside the app.
// Debug builds keep the console so `println!`/`eprintln!` remain visible.
#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

fn main() {
    pulse::app::run();
}
