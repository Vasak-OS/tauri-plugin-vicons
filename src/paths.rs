//! De dónde se puede leer un icono por ruta.
//!
//! Un `.desktop` puede traer `Icon=/usr/share/loquesea/icono.png`, así que aceptar
//! una ruta y no sólo un nombre de tema es necesario. Lo que no se puede es aceptar
//! **cualquier** ruta.
//!
//! El nombre del icono llega desde el WebView: es un argumento del comando
//! `get_icon`. Sin límite, `getIconSource('/home/quien/.ssh/id_ed25519')` devuelve
//! la clave privada en base64, dentro de un `data:` URL que la página puede leer.
//! Y como este plugin lo usan las once aplicaciones del escritorio, alcanzaba con
//! que una sola cargara contenido ajeno —una previsualización, un iframe, una
//! página de ayuda— para poder leer cualquier archivo del usuario. Comprobado con
//! `/etc/passwd` antes de escribir esto.
//!
//! Tres cercos, y hacen falta los tres:
//!
//! 1. La ruta se canonicaliza **antes** de comparar. Sin eso, un enlace simbólico
//!    dentro de un directorio permitido apunta a donde quiera y el cerco no sirve.
//! 2. Tiene que caer dentro de un directorio de datos, que es donde viven los
//!    iconos. `/etc`, `/proc` y el resto del `$HOME` quedan afuera.
//! 3. La extensión tiene que ser de imagen. Esto es lo que deja afuera una clave
//!    o un `.env` que alguien haya guardado en `~/.local/share/icons`.

use std::path::{Path, PathBuf};

/// Las extensiones que un icono puede tener.
///
/// `xpm` y `svgz` están porque los temas viejos los usan. Nada de esto es una
/// lista de formatos que se sepan decodificar: es una lista de lo que un icono
/// puede llamarse, y su trabajo es dejar afuera todo lo demás.
pub const EXTENSIONES: [&str; 10] = [
    "png", "svg", "svgz", "xpm", "jpg", "jpeg", "gif", "webp", "bmp", "ico",
];

/// Hasta qué tamaño se lee un archivo de icono.
///
/// Un icono de verdad no llega ni al megabyte. El límite está porque el nombre lo
/// elige el WebView: sin él, pedir un archivo de varios gigabytes lo carga entero
/// en memoria y encima lo codifica en base64, que agrega un tercio.
pub const LIMITE_ARCHIVO: u64 = 8 * 1024 * 1024;

/// Si el nombre del archivo termina en una extensión de imagen.
pub fn has_icon_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| {
            let e = e.to_ascii_lowercase();
            EXTENSIONES.contains(&e.as_str())
        })
        .unwrap_or(false)
}

/// Agrega `icons` y `pixmaps` de un directorio de datos.
fn subdirectorios_de_iconos(base: &Path, destino: &mut Vec<PathBuf>) {
    destino.push(base.join("icons"));
    destino.push(base.join("pixmaps"));
}

/// Los directorios de donde se acepta leer un icono.
///
/// Se sigue la especificación de directorios de XDG en lugar de una lista fija:
/// una instalación con `XDG_DATA_DIRS` propio —un Flatpak, un prefijo en `/opt`—
/// tiene sus iconos en otra parte, y una lista fija los dejaría sin icono.
pub fn allowed_roots() -> Vec<PathBuf> {
    let mut raices = Vec::new();

    // Los árboles de datos del sistema, que son públicos por definición.
    let dirs = std::env::var("XDG_DATA_DIRS")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| "/usr/local/share:/usr/share".to_string());
    for parte in dirs.split(':').filter(|p| !p.is_empty()) {
        raices.push(PathBuf::from(parte));
    }

    // Los del usuario: sólo los de iconos, no el `$HOME` entero ni todo
    // `~/.local/share`, que es donde viven credenciales de otras aplicaciones.
    if let Some(datos) = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
    {
        subdirectorios_de_iconos(&datos, &mut raices);
    }
    if let Some(hogar) = std::env::var_os("HOME") {
        raices.push(PathBuf::from(&hogar).join(".icons"));
    }

    raices
}

/// Si una ruta ya canonicalizada cae dentro de alguna de las raíces.
///
/// Las raíces también se canonicalizan: en una instalación donde `/usr/share` sea
/// un enlace, comparar sin resolver no acierta nunca y se quedan todos los iconos
/// sin cargar.
pub fn is_inside(canonica: &Path, raices: &[PathBuf]) -> bool {
    raices.iter().any(|raiz| {
        let raiz = raiz.canonicalize().unwrap_or_else(|_| raiz.clone());
        canonica.starts_with(&raiz)
    })
}

/// La ruta de la que se puede leer, si el nombre era una ruta aceptable.
///
/// `None` significa «esto no es una ruta permitida», y quien llama sigue con la
/// búsqueda por tema. No se distingue «no existe» de «no se permite» a propósito:
/// contestar distinto convierte esto en una forma de averiguar qué archivos hay.
pub fn readable_icon_path(name: &str, raices: &[PathBuf]) -> Option<PathBuf> {
    // Sólo se considera ruta lo que empieza como una: un nombre de tema como
    // `folder` no tiene por qué tocar el disco.
    if !name.starts_with('/') {
        return None;
    }

    let canonica = Path::new(name).canonicalize().ok()?;
    if !canonica.is_file() || !has_icon_extension(&canonica) || !is_inside(&canonica, raices) {
        return None;
    }

    // El tamaño se mira acá y no al leer: da lo mismo para el resultado, pero
    // evita abrir un archivo enorme para después descartarlo.
    let cabe = std::fs::metadata(&canonica)
        .map(|m| m.len() <= LIMITE_ARCHIVO)
        .unwrap_or(false);

    cabe.then_some(canonica)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Un árbol de mentira: una raíz permitida y un secreto afuera.
    ///
    /// Con nombre propio por prueba. Las pruebas corren en paralelo dentro del
    /// mismo proceso, así que un directorio compartido hace que se borren los
    /// archivos entre ellas — y el fallo aparece y desaparece según el orden.
    fn escenario(quien: &str) -> (PathBuf, Vec<PathBuf>) {
        let base = std::env::temp_dir().join(format!("vicons-prueba-{}-{quien}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(base.join("permitido")).unwrap();
        fs::create_dir_all(base.join("secretos")).unwrap();

        fs::write(base.join("permitido/icono.png"), b"\x89PNG falso").unwrap();
        fs::write(base.join("permitido/notas.txt"), b"no soy un icono").unwrap();
        fs::write(base.join("secretos/id_ed25519"), b"CLAVE PRIVADA").unwrap();
        fs::write(base.join("secretos/robada.png"), b"tampoco").unwrap();

        let raices = vec![base.join("permitido")];
        (base, raices)
    }

    #[test]
    fn un_icono_de_un_directorio_permitido_se_lee() {
        const NOMBRE: &str = "permitido";
        let (base, raices) = escenario(NOMBRE);
        let ruta = base.join("permitido/icono.png");
        assert_eq!(
            readable_icon_path(ruta.to_str().unwrap(), &raices),
            Some(ruta.canonicalize().unwrap())
        );
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn un_archivo_de_afuera_no_se_lee() {
        const NOMBRE: &str = "afuera";
        // Esto es lo que estaba roto: cualquier página del WebView podía pedir
        // cualquier archivo del usuario y recibirlo en base64.
        let (base, raices) = escenario(NOMBRE);
        let secreto = base.join("secretos/id_ed25519");
        assert_eq!(readable_icon_path(secreto.to_str().unwrap(), &raices), None);
        // Y tampoco con extensión de imagen: lo que decide es dónde está.
        let disfrazado = base.join("secretos/robada.png");
        assert_eq!(readable_icon_path(disfrazado.to_str().unwrap(), &raices), None);
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn un_enlace_simbolico_no_saca_nada_de_su_lugar() {
        const NOMBRE: &str = "enlace";
        // Sin canonicalizar antes de comparar, un enlace dentro de un directorio
        // permitido apunta a donde quiera y el cerco no sirve para nada.
        let (base, raices) = escenario(NOMBRE);
        let enlace = base.join("permitido/parece-icono.png");
        std::os::unix::fs::symlink(base.join("secretos/id_ed25519"), &enlace).unwrap();
        assert_eq!(readable_icon_path(enlace.to_str().unwrap(), &raices), None);
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn los_dos_puntos_no_salen_del_directorio() {
        const NOMBRE: &str = "travesia";
        let (base, raices) = escenario(NOMBRE);
        let travesia = format!("{}/permitido/../secretos/id_ed25519", base.display());
        assert_eq!(readable_icon_path(&travesia, &raices), None);
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn un_nombre_de_tema_no_toca_el_disco() {
        const NOMBRE: &str = "tema";
        // `folder` es un nombre, no una ruta: tiene que seguir por la búsqueda del
        // tema, que es el camino normal.
        let (base, raices) = escenario(NOMBRE);
        assert_eq!(readable_icon_path("folder", &raices), None);
        assert_eq!(readable_icon_path("", &raices), None);
        assert_eq!(readable_icon_path("etc/passwd", &raices), None);
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn lo_que_no_es_una_imagen_no_pasa_aunque_este_en_el_lugar_correcto() {
        const NOMBRE: &str = "no-imagen";
        // Alguien puede tener un `.env` o una clave dentro de `~/.icons`.
        let (base, raices) = escenario(NOMBRE);
        let texto = base.join("permitido/notas.txt");
        assert_eq!(readable_icon_path(texto.to_str().unwrap(), &raices), None);
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn un_directorio_no_es_un_icono() {
        const NOMBRE: &str = "directorio";
        let (base, raices) = escenario(NOMBRE);
        let dir = base.join("permitido");
        assert_eq!(readable_icon_path(dir.to_str().unwrap(), &raices), None);
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn un_archivo_enorme_no_se_carga_en_memoria() {
        const NOMBRE: &str = "enorme";
        // El nombre lo elige el WebView: sin límite, pedir un archivo de gigabytes
        // lo carga entero y encima lo codifica en base64, que agrega un tercio.
        let (base, raices) = escenario(NOMBRE);
        let gordo = base.join("permitido/gordo.png");
        let f = fs::File::create(&gordo).unwrap();
        f.set_len(LIMITE_ARCHIVO + 1).unwrap();
        assert_eq!(readable_icon_path(gordo.to_str().unwrap(), &raices), None);

        // Y uno que entra justo sí se lee, para que el límite no sea un rechazo
        // disfrazado.
        let justo = base.join("permitido/justo.png");
        fs::File::create(&justo).unwrap().set_len(LIMITE_ARCHIVO).unwrap();
        assert!(readable_icon_path(justo.to_str().unwrap(), &raices).is_some());
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn las_extensiones_no_dependen_de_las_mayusculas() {
        // Los temas viejos traen `.PNG` y `.SVG`.
        assert!(has_icon_extension(Path::new("a/b/ICONO.PNG")));
        assert!(has_icon_extension(Path::new("a/b/icono.SvG")));
        assert!(!has_icon_extension(Path::new("a/b/clave.pem")));
        assert!(!has_icon_extension(Path::new("a/b/sin-extension")));
        assert!(!has_icon_extension(Path::new("a/b/.png")), "sólo extensión, sin nombre");
    }

    #[test]
    fn las_raices_permitidas_no_incluyen_el_hogar_entero() {
        // `~/.local/share` guarda credenciales de otras aplicaciones; sólo entran
        // sus subdirectorios de iconos.
        let raices = allowed_roots();
        if let Some(hogar) = std::env::var_os("HOME") {
            let hogar = PathBuf::from(hogar);
            assert!(!raices.contains(&hogar), "el hogar entero no puede ser una raíz");
            assert!(!raices.contains(&hogar.join(".local/share")));
            assert!(raices.iter().any(|r| r.ends_with(".icons")));
        }
        assert!(!raices.contains(&PathBuf::from("/")), "la raíz del sistema tampoco");
        assert!(!raices.contains(&PathBuf::from("/etc")));
    }

    #[test]
    fn hay_raices_del_sistema_aunque_falte_el_entorno() {
        // Sin `XDG_DATA_DIRS` la especificación manda usar estos dos; sin ellos, el
        // escritorio se quedaría sin ningún icono del sistema.
        let raices = allowed_roots();
        assert!(
            raices.contains(&PathBuf::from("/usr/share"))
                || std::env::var("XDG_DATA_DIRS").is_ok(),
            "{raices:?}"
        );
    }
}
