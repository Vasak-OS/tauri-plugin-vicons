use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::fs;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, SystemTime};

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use gtk::prelude::IconThemeExt;
use tauri::{Manager, plugin::PluginApi, AppHandle, Runtime};

use crate::error::Result;
use crate::models::CacheEntry;

static ICON_CACHE: LazyLock<Mutex<HashMap<String, CacheEntry>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static SYMBOL_CACHE: LazyLock<Mutex<HashMap<String, CacheEntry>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

const CACHE_DURATION: Duration = Duration::from_secs(30 * 60);

fn is_cache_expired(timestamp: SystemTime) -> bool {
    timestamp.elapsed().map_or(true, |d| d > CACHE_DURATION)
}

fn clear_cache_internal() {
    ICON_CACHE.lock().unwrap().clear();
    SYMBOL_CACHE.lock().unwrap().clear();
    crate::logger::info("Icon cache cleared (auto-detected theme change)");
}

pub fn init_theme_monitor() {
    if let Some(themed) = gtk::IconTheme::default() {
        themed.connect_changed(|_theme| {
            crate::logger::info("Icon theme change detected");
            clear_cache_internal();
        });
    } else {
        crate::logger::error("Failed to initialize theme monitor");
    }
}

fn get_cached_icon_data(
    name: &str,
    cache: &Mutex<HashMap<String, CacheEntry>>,
    lookup_flags: gtk::IconLookupFlags,
    icon_type: &str,
) -> Result<String> {
    {
        let mut guard = cache.lock().unwrap();
        match guard.entry(name.to_string()) {
            Entry::Occupied(e) if !is_cache_expired(e.get().timestamp) => {
                return Ok(e.get().data.clone());
            }
            Entry::Occupied(e) => {
                crate::logger::info(&format!("Cache expired for {}: '{}'", icon_type, name));
                e.remove_entry();
            }
            Entry::Vacant(_) => {}
        }
    }

    let themed =
        gtk::IconTheme::default().ok_or(crate::error::Error::ThemeMonitorError)?;

    let mut themed_icon = themed.lookup_icon(name, 64, lookup_flags);

    if themed_icon.is_none() {
        crate::logger::warn(&format!("{} not found: '{}'", icon_type, name));
        themed_icon = themed.lookup_icon("image-missing", 64, lookup_flags);
    }

    let icon = themed_icon
        .ok_or_else(|| crate::error::Error::IconNotFound(name.to_string()))?
        .filename()
        .ok_or_else(|| crate::error::Error::IconNotFound(name.to_string()))?;

    let icon_data = fs::read(icon)?;
    let encoded = STANDARD.encode(icon_data);

    cache
        .lock()
        .unwrap()
        .entry(name.to_string())
        .or_insert(CacheEntry {
            data: encoded.clone(),
            timestamp: SystemTime::now(),
        });

    Ok(encoded)
}

fn read_file_as_base64<P: AsRef<std::path::Path>>(path: P) -> Result<String> {
    let icon_data = fs::read(path.as_ref())?;
    Ok(STANDARD.encode(icon_data))
}

pub fn get_icon_impl(name: &str) -> Result<String> {
    let path = std::path::Path::new(name);
    if path.exists() && path.is_file() {
        crate::logger::info(&format!("Icon from file path: '{}'", name));
        return read_file_as_base64(path);
    }

    get_cached_icon_data(
        name,
        &ICON_CACHE,
        gtk::IconLookupFlags::FORCE_SVG | gtk::IconLookupFlags::FORCE_REGULAR,
        "Icon",
    )
}

pub fn get_symbol_impl(name: &str) -> Result<String> {
    let path = std::path::Path::new(name);
    if path.exists() && path.is_file() {
        crate::logger::info(&format!("Symbol from file path: '{}'", name));
        return read_file_as_base64(path);
    }

    get_cached_icon_data(
        name,
        &SYMBOL_CACHE,
        gtk::IconLookupFlags::FORCE_SYMBOLIC | gtk::IconLookupFlags::FORCE_SVG,
        "Symbol",
    )
}

pub fn init<R: Runtime, C: DeserializeOwned>(
    app: &AppHandle<R>,
    _api: PluginApi<R, C>,
) -> crate::Result<Vicons<R>> {
    let log_dir = app
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."));
    let log_path = log_dir.join("logs").join("icons.log");
    crate::logger::init(&log_path);

    init_theme_monitor();

    Ok(Vicons(app.clone()))
}

pub struct Vicons<R: Runtime>(AppHandle<R>);

impl<R: Runtime> Vicons<R> {}
