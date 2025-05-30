const COMMANDS: &[&str] = &["get_icon", "get_symbol"];

fn main() {
  tauri_plugin::Builder::new(COMMANDS)
    .android_path("android")
    .ios_path("ios")
    .build();
}
