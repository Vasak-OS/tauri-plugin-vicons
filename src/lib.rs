use tauri::{
  plugin::{Builder, TauriPlugin},
  Manager, Runtime,
};

pub use models::*;

#[cfg(desktop)]
mod desktop;
#[cfg(mobile)]
mod mobile;

mod commands;
mod error;
mod models;

pub use error::{Error, Result};

#[cfg(desktop)]
use desktop::Vicons;
#[cfg(mobile)]
use mobile::Vicons;

/// Extensions to [`tauri::App`], [`tauri::AppHandle`] and [`tauri::Window`] to access the vicons APIs.
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
    .invoke_handler(tauri::generate_handler![commands::ping])
    .setup(|app, api| {
      #[cfg(mobile)]
      let vicons = mobile::init(app, api)?;
      #[cfg(desktop)]
      let vicons = desktop::init(app, api)?;
      app.manage(vicons);
      Ok(())
    })
    .build()
}
