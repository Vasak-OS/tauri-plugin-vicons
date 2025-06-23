use tauri::{
    plugin::{Builder, TauriPlugin},
    Manager, Runtime,
};

mod desktop;

mod commands;
mod error;
mod models;

pub use error::{Error, Result};

use desktop::Vicons;

pub trait ViconsExt<R: Runtime> {
    fn vicons(&self) -> &Vicons<R>;
}

impl<R: Runtime, T: Manager<R>> crate::ViconsExt<R> for T {
    fn vicons(&self) -> &Vicons<R> {
        self.state::<Vicons<R>>().inner()
    }
}

/// Initializes the plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("vicons")
        .invoke_handler(tauri::generate_handler![
            commands::get_icon,
            commands::get_symbol
        ])
        .setup(|app, api| {
            let vicons = desktop::init(app, api)?;
            app.manage(vicons);
            Ok(())
        })
        .build()
}
