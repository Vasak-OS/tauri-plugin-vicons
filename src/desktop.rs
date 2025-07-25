use serde::de::DeserializeOwned;
use tauri::{plugin::PluginApi, AppHandle, Runtime};

pub fn init<R: Runtime, C: DeserializeOwned>(
  app: &AppHandle<R>,
  _api: PluginApi<R, C>,
) -> crate::Result<Vicons<R>> {
  Ok(Vicons(app.clone()))
}

/// Access to the vicons APIs.
pub struct Vicons<R: Runtime>(AppHandle<R>);

impl<R: Runtime> Vicons<R> {

}
