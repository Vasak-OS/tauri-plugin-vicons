use serde::de::DeserializeOwned;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fs;
use std::sync::{LazyLock, Mutex};
use std::time::SystemTime;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use gtk::prelude::IconThemeExt;
use tauri::{plugin::PluginApi, AppHandle, Emitter, Manager, Runtime};

use crate::cache::{self, bloquear};
use crate::error::Result;
use crate::models::CacheEntry;
use crate::paths;

static ICON_CACHE: LazyLock<Mutex<HashMap<String, CacheEntry>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static SYMBOL_CACHE: LazyLock<Mutex<HashMap<String, CacheEntry>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn is_cache_expired(timestamp: SystemTime) -> bool {
    cache::is_expired(timestamp, SystemTime::now(), cache::CACHE_DURATION)
}

fn clear_cache_internal() {
    bloquear(&ICON_CACHE).clear();
    bloquear(&SYMBOL_CACHE).clear();
    crate::logger::info("Icon cache cleared (auto-detected theme change)");
}

pub fn init_theme_monitor<R: Runtime>(app: &AppHandle<R>) {
    if let Some(themed) = gtk::IconTheme::default() {
        let app_clone = app.clone();
        themed.connect_changed(move |_theme| {
            crate::logger::info("Icon theme change detected");
            clear_cache_internal();
            let _ = app_clone.emit("vicons:theme-changed", ());
        });
    } else {
        crate::logger::error("Failed to initialize theme monitor");
    }
}

/// A qué tamaño se le pide el icono a GTK.
///
/// No es el tamaño al que se dibuja —eso lo decide quien pone el `<img>`—: es el
/// tamaño con el que se elige *qué archivo* del tema se usa. Importa que sea el
/// mismo en la búsqueda y en la pregunta: un tema puede tener el icono en un
/// tamaño y no en otro, y entonces `has_symbol` diría que sí de algo que
/// `get_symbol` no trae.
const TAMANO_DE_BUSQUEDA: i32 = 64;

/// Con qué banderas se busca un icono regular.
///
/// Aparte, y no escritas en cada sitio donde se buscan, porque la pregunta
/// «¿está?» y la búsqueda «traelo» tienen que hacer **exactamente** la misma
/// búsqueda. Si se separan, una contesta por un archivo que la otra no usa.
fn banderas_de_icono() -> gtk::IconLookupFlags {
    gtk::IconLookupFlags::FORCE_SVG | gtk::IconLookupFlags::FORCE_REGULAR
}

/// Con qué banderas se busca un icono simbólico.
///
/// `FORCE_SYMBOLIC` hace que GTK pruebe «<nombre>-symbolic» a lo largo de toda la
/// cadena de herencia antes que el nombre pelado, que es lo que hace que el
/// escritorio dibuje siempre la versión de línea.
fn banderas_de_simbolo() -> gtk::IconLookupFlags {
    gtk::IconLookupFlags::FORCE_SYMBOLIC | gtk::IconLookupFlags::FORCE_SVG
}

fn get_cached_icon_data(
    name: &str,
    cache: &Mutex<HashMap<String, CacheEntry>>,
    lookup_flags: gtk::IconLookupFlags,
    icon_type: &str,
) -> Result<String> {
    {
        let mut guard = bloquear(cache);
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

    let themed = gtk::IconTheme::default().ok_or(crate::error::Error::ThemeMonitorError)?;

    let mut themed_icon = themed.lookup_icon(name, TAMANO_DE_BUSQUEDA, lookup_flags);

    if themed_icon.is_none() {
        crate::logger::warn(&format!("{} not found: '{}'", icon_type, name));
        themed_icon = themed.lookup_icon("image-missing", TAMANO_DE_BUSQUEDA, lookup_flags);
    }

    let icon = themed_icon
        .ok_or_else(|| crate::error::Error::IconNotFound(name.to_string()))?
        .filename()
        .ok_or_else(|| crate::error::Error::IconNotFound(name.to_string()))?;

    let icon_data = fs::read(icon)?;
    let encoded = STANDARD.encode(icon_data);

    {
        let mut guard = bloquear(cache);
        guard.entry(name.to_string()).or_insert(CacheEntry {
            data: encoded.clone(),
            timestamp: SystemTime::now(),
        });
        // Con techo: la clave la elige quien pide el icono, así que sin esto la
        // caché crece hasta donde la dejen.
        cache::evict_oldest(&mut guard, cache::LIMITE);
    }

    Ok(encoded)
}

fn read_file_as_base64<P: AsRef<std::path::Path>>(path: P) -> Result<String> {
    let icon_data = fs::read(path.as_ref())?;
    Ok(STANDARD.encode(icon_data))
}

pub fn get_icon_impl(name: &str) -> Result<String> {
    // Una ruta, sólo si cae en un directorio de iconos y es una imagen. El nombre
    // llega desde el WebView: sin este cerco, cualquier página cargada en cualquier
    // aplicación del escritorio leía cualquier archivo del usuario. Ver `paths`.
    if let Some(ruta) = paths::readable_icon_path(name, &paths::allowed_roots()) {
        crate::logger::info(&format!("Icon from file path: '{}'", name));
        return read_file_as_base64(ruta);
    }

    get_cached_icon_data(name, &ICON_CACHE, banderas_de_icono(), "Icon")
}

pub fn get_symbol_impl(name: &str) -> Result<String> {
    if let Some(ruta) = paths::readable_icon_path(name, &paths::allowed_roots()) {
        crate::logger::info(&format!("Symbol from file path: '{}'", name));
        return read_file_as_base64(ruta);
    }

    get_cached_icon_data(name, &SYMBOL_CACHE, banderas_de_simbolo(), "Symbol")
}

/// Si el tema tiene con qué dibujar este nombre, sin traer el archivo.
///
/// Existe porque `get_icon_impl` y `get_symbol_impl` **nunca** fallan por un
/// nombre que no está: cuando GTK no lo encuentra, devuelven `image-missing` —el
/// cuadrito de imagen rota— como si fuera el icono pedido. Quien llama recibe un
/// base64 perfectamente válido y no tiene forma de distinguirlo del icono de
/// verdad, así que lo dibuja.
///
/// Eso se vio en «Cuentas en Línea» de `vasak-settings`: Google, Microsoft y
/// Nextcloud salían los tres con el cuadrito, y la vista no podía ni saberlo
/// —su `v-if` sobre el resultado nunca era falso— ni elegir otra cosa.
///
/// La búsqueda es la misma que haría `get_*`: mismo tamaño, mismas banderas. Si
/// no lo fuera, esto contestaría por un archivo que la otra no usa, que es peor
/// que no preguntar.
fn has_impl(name: &str, lookup_flags: gtk::IconLookupFlags) -> Result<bool> {
    // Una ruta cuenta como que está si el cerco la acepta y el archivo existe:
    // es de donde `get_*` lo leería. Ver `paths`.
    if paths::readable_icon_path(name, &paths::allowed_roots()).is_some() {
        return Ok(true);
    }

    let themed = gtk::IconTheme::default().ok_or(crate::error::Error::ThemeMonitorError)?;

    // Sin `GENERIC_FALLBACK` ni `USE_BUILTIN`, `lookup_icon` contesta `None`
    // cuando no hay nada: es el único punto del plugin donde el faltante se
    // puede ver antes de que lo tape `image-missing`.
    Ok(themed
        .lookup_icon(name, TAMANO_DE_BUSQUEDA, lookup_flags)
        .is_some())
}

/// Si `get_icon_impl` va a devolver este icono y no el cuadrito.
pub fn has_icon_impl(name: &str) -> Result<bool> {
    has_impl(name, banderas_de_icono())
}

/// Si `get_symbol_impl` va a devolver este símbolo y no el cuadrito.
pub fn has_symbol_impl(name: &str) -> Result<bool> {
    has_impl(name, banderas_de_simbolo())
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

    init_theme_monitor(app);

    Ok(Vicons(app.clone()))
}

pub struct Vicons<R: Runtime>(AppHandle<R>);

impl<R: Runtime> Vicons<R> {}

#[cfg(test)]
mod tests {
    use super::*;

    /// El texto de un archivo, en base64, para comparar.
    fn en_base64(ruta: &str) -> Option<String> {
        fs::read(ruta).ok().map(|b| STANDARD.encode(b))
    }

    /// Las raíces de verdad de esta máquina, que son las que usa el plugin.
    fn raices() -> Vec<std::path::PathBuf> {
        paths::allowed_roots()
    }

    // Ojo: acá no se puede llamar a `get_icon_impl` con un nombre que **no** sea
    // una ruta permitida. La búsqueda por tema llama a `gtk::IconTheme::default()`,
    // que entra en pánico si GTK no está inicializado —en la aplicación lo
    // inicializa Tauri, en una prueba no—. Así que el cerco se comprueba donde
    // vive, y que `get_icon_impl` lo consulte se comprueba con el caso legítimo,
    // que devuelve antes de tocar GTK.

    #[test]
    fn preguntar_y_traer_hacen_la_misma_busqueda() {
        // Lo único que hace útil a `has_symbol` es que conteste por el mismo
        // archivo que `get_symbol` va a traer. Si alguna de las dos cambia de
        // banderas por su cuenta, la respuesta pasa a ser sobre otro icono: diría
        // que sí de algo que después sale como el cuadrito, que es exactamente el
        // problema que vino a resolver.
        //
        // Acá se comprueba la intención de cada juego —que el simbólico fuerce la
        // versión de línea y el regular no—; que `get_*` y `has_*` usen éstas y no
        // otras lo garantiza que no queden banderas escritas en ningún otro sitio.
        let simbolo = banderas_de_simbolo();
        assert!(simbolo.contains(gtk::IconLookupFlags::FORCE_SYMBOLIC));
        assert!(simbolo.contains(gtk::IconLookupFlags::FORCE_SVG));
        assert!(!simbolo.contains(gtk::IconLookupFlags::FORCE_REGULAR));

        let icono = banderas_de_icono();
        assert!(icono.contains(gtk::IconLookupFlags::FORCE_REGULAR));
        assert!(icono.contains(gtk::IconLookupFlags::FORCE_SVG));
        assert!(!icono.contains(gtk::IconLookupFlags::FORCE_SYMBOLIC));
    }

    #[test]
    fn un_icono_que_esta_en_el_disco_se_responde_sin_preguntarle_a_gtk() {
        // Un `.desktop` puede traer `Icon=` con una ruta absoluta, y de ahí es de
        // donde `get_*` lo leería: la pregunta tiene que contestar por lo mismo.
        //
        // Sirve además para probar `has_*` sin GTK inicializado, que en una prueba
        // no lo está: este camino devuelve antes de tocarlo.
        let candidatos = [
            "/usr/share/icons/VasakOS/apps/scalable/folder.svg",
            "/usr/share/icons/VasakOS/devices/16/cpu.svg",
            "/usr/share/pixmaps/archlinux-logo.png",
        ];
        let Some(existente) = candidatos
            .iter()
            .find(|r| std::path::Path::new(r).is_file())
        else {
            return;
        };

        assert!(has_icon_impl(existente).unwrap());
        assert!(has_symbol_impl(existente).unwrap());
    }

    #[test]
    fn un_nombre_de_icono_no_puede_ser_cualquier_archivo() {
        // Regresión de un agujero real: `get_icon` recibe el nombre desde el
        // WebView y aceptaba cualquier ruta existente, así que
        // `getIconSource('/etc/passwd')` devolvía el archivo en base64 dentro de un
        // `data:` URL. Comprobado sobre esta máquina antes de arreglarlo.
        for archivo in [
            "/etc/passwd",
            "/etc/hostname",
            "/etc/fstab",
            "/proc/self/environ",
        ] {
            if !std::path::Path::new(archivo).exists() {
                continue;
            }
            assert_eq!(
                paths::readable_icon_path(archivo, &raices()),
                None,
                "{archivo} se aceptó como icono"
            );
        }
    }

    #[test]
    fn una_clave_privada_del_hogar_no_se_puede_pedir() {
        // El caso que importa de verdad. Se crea una de mentira para no depender de
        // que la máquina tenga claves, y se prueba también disfrazada de imagen:
        // lo que decide es el directorio, no la extensión.
        let base = std::env::temp_dir().join(format!("vicons-clave-{}", std::process::id()));
        let _ = fs::create_dir_all(&base);

        for nombre in ["id_ed25519", "id_ed25519.png", ".env"] {
            let ruta = base.join(nombre);
            fs::write(&ruta, b"-----BEGIN OPENSSH PRIVATE KEY-----").unwrap();
            assert_eq!(
                paths::readable_icon_path(ruta.to_str().unwrap(), &raices()),
                None,
                "{nombre} se aceptó como icono"
            );
        }

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn un_icono_de_verdad_del_tema_instalado_si_se_lee_por_ruta() {
        // El cerco no sirve si rompe el caso legítimo: un `.desktop` puede traer
        // `Icon=` con una ruta absoluta. Y esto sí pasa por `get_icon_impl`, que es
        // lo que ata el cerco al comando.
        let candidatos = [
            "/usr/share/icons/VasakOS/apps/scalable/folder.svg",
            "/usr/share/icons/VasakOS/devices/16/cpu.svg",
            "/usr/share/pixmaps/archlinux-logo.png",
        ];
        let Some(existente) = candidatos
            .iter()
            .find(|r| std::path::Path::new(r).is_file())
        else {
            // En una máquina sin esos temas no hay nada que comprobar.
            return;
        };

        let leido = get_icon_impl(existente).expect("un icono del sistema tiene que leerse");
        assert_eq!(Some(leido), en_base64(existente));
    }
}
