use serde::de::DeserializeOwned;
use tauri::{plugin::PluginApi, AppHandle, Runtime};
use std::collections::HashMap;
use std::time::{SystemTime, Duration};
use parking_lot::Mutex;
use once_cell::sync::Lazy;
use gtk::prelude::IconThemeExt;
use std::fs;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;

use crate::models::CacheEntry;
use crate::error::Result;

// Caché global para iconos
static ICON_CACHE: Lazy<Mutex<HashMap<String, CacheEntry>>> = Lazy::new(|| Mutex::new(HashMap::new()));
static SYMBOL_CACHE: Lazy<Mutex<HashMap<String, CacheEntry>>> = Lazy::new(|| Mutex::new(HashMap::new()));

// Duración del caché: 30 minutos
const CACHE_DURATION: Duration = Duration::from_secs(30 * 60);

/// Verifica si una entrada de caché ha expirado.
/// En caso de error al obtener el tiempo transcurrido (ej: problemas con el reloj del sistema),
/// la entrada se considera expirada para mayor seguridad.
fn is_cache_expired(timestamp: SystemTime) -> bool {
	timestamp.elapsed().map_or(true, |d| d > CACHE_DURATION)
}

/// Limpia la caché de ambos tipos
fn clear_cache_internal() {
    ICON_CACHE.lock().clear();
    SYMBOL_CACHE.lock().clear();
    eprintln!("[tauri-plugin-vicons] Icon cache cleared (auto-detected theme change)");
}

/// Inicializa el monitor de cambios de tema.
pub fn init_theme_monitor() {
    if let Some(themed) = gtk::IconTheme::default() {
        themed.connect_changed(|_theme| {
            eprintln!("[tauri-plugin-vicons] Icon theme change detected");
            clear_cache_internal();
        });
    } else {
        eprintln!("[tauri-plugin-vicons] Failed to initialize theme monitor");
    }
}

/// Helper para obtener datos de icono con caché
/// Maneja: verificación de caché, búsqueda en tema, fallback a image-missing,
/// lectura de archivo, codificación base64 y almacenamiento en caché
fn get_cached_icon_data(
    name: &str,
    cache: &Mutex<HashMap<String, CacheEntry>>,
    lookup_flags: gtk::IconLookupFlags,
    icon_type: &str,
) -> Result<String> {
    // Verificar caché primero
    {
        let mut cache_guard = cache.lock();
        if let Some(entry) = cache_guard.get(name) {
            if !is_cache_expired(entry.timestamp) {
                return Ok(entry.data.clone());
            } else {
                cache_guard.remove(name);
            }
        }
    }

    let themed = gtk::IconTheme::default().ok_or(crate::error::Error::ThemeMonitorError)?;

    let mut themed_icon = themed.lookup_icon(name, 64, lookup_flags);

    if themed_icon.is_none() {
        eprintln!("[tauri-plugin-vicons] {} not found: '{}'", icon_type, name);
        themed_icon = themed.lookup_icon("image-missing", 64, lookup_flags);
    }

    let icon = themed_icon
        .ok_or(crate::error::Error::IconNotFound(name.to_string()))?
        .filename()
        .ok_or(crate::error::Error::IconNotFound(name.to_string()))?;

    let icon_data = fs::read(icon)?;
    let encoded = STANDARD.encode(icon_data);

    // Guardar en caché
    cache.lock().insert(
        name.to_string(),
        CacheEntry {
            data: encoded.clone(),
            timestamp: SystemTime::now(),
        },
    );

    Ok(encoded)
}

/// Obtiene un icono, verificando caché primero
pub fn get_icon_impl(name: &str) -> Result<String> {
    get_cached_icon_data(
        name,
        &ICON_CACHE,
        gtk::IconLookupFlags::FORCE_SVG | gtk::IconLookupFlags::FORCE_REGULAR,
        "Icon",
    )
}

/// Obtiene un símbolo, verificando caché primero
pub fn get_symbol_impl(name: &str) -> Result<String> {
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
    // Inicializar el monitor de cambios de tema
    init_theme_monitor();
    
    Ok(Vicons(app.clone()))
}

/// Access to the vicons APIs.
pub struct Vicons<R: Runtime>(AppHandle<R>);

impl<R: Runtime> Vicons<R> {}
