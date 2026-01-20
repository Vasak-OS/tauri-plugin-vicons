use crate::error::Result;

#[tauri::command]
pub fn get_icon(name: &str) -> Result<String> {
    super::desktop::get_icon_impl(name)
}

#[tauri::command]
pub fn get_symbol(name: &str) -> Result<String> {
    super::desktop::get_symbol_impl(name)
}
