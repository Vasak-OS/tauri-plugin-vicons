//! La caché de iconos.
//!
//! Existe porque una lista de aplicaciones pide ciento veintiocho iconos de golpe,
//! y cada uno es una búsqueda en el tema más una lectura de disco. Lo que sigue son
//! las dos cosas que le faltaban: un techo y una expiración que se pueda probar.

use crate::models::CacheEntry;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

/// Cuánto vale una entrada antes de volver a buscarla.
pub const CACHE_DURATION: Duration = Duration::from_secs(30 * 60);

/// Cuántas entradas se guardan como máximo.
///
/// La caché no tenía techo y la clave es el nombre que pide el WebView. Con la
/// búsqueda fallida guardando el icono de reemplazo bajo el nombre pedido,
/// cualquier bucle que pidiera nombres distintos la hacía crecer sin freno, y cada
/// entrada es un icono en base64. Quinientas doce alcanzan de sobra: el menú de
/// aplicaciones de esta máquina usa ciento veintiocho.
pub const LIMITE: usize = 512;

/// Si una entrada ya no vale.
///
/// Toma «ahora» como argumento para poder probarla: con `elapsed()` adentro, la
/// única forma de comprobar la expiración era esperar media hora.
///
/// Un instante en el futuro —el reloj que se corrigió hacia atrás— cuenta como
/// expirado. Es el lado seguro: se vuelve a leer el icono, que es barato, en lugar
/// de servir para siempre algo que quedó guardado con una fecha imposible.
pub fn is_expired(timestamp: SystemTime, ahora: SystemTime, duracion: Duration) -> bool {
    ahora.duration_since(timestamp).map_or(true, |d| d > duracion)
}

/// Toma el candado sin morirse si quedó envenenado.
///
/// Un pánico mientras alguien tiene el candado lo envenena, y a partir de ahí
/// **todas** las búsquedas de iconos entrarían en pánico para siempre: en el panel
/// del escritorio eso es quedarse sin ningún icono hasta reiniciar la sesión. Esto
/// es una caché; recuperarla es estrictamente mejor que morir con ella.
pub fn bloquear<T>(candado: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    candado.lock().unwrap_or_else(|envenenado| envenenado.into_inner())
}

/// Deja la caché por debajo del techo, sacando primero lo más viejo.
///
/// Lo más viejo y no una entrada al azar: lo que se pidió hace rato es lo que menos
/// probable es que se vuelva a pedir, y sacar al azar puede tirar justo el icono
/// que se está mostrando.
pub fn evict_oldest(cache: &mut HashMap<String, CacheEntry>, limite: usize) {
    while cache.len() > limite {
        let mas_vieja = cache
            .iter()
            .min_by_key(|(_, entrada)| entrada.timestamp)
            .map(|(clave, _)| clave.clone());

        match mas_vieja {
            Some(clave) => {
                cache.remove(&clave);
            }
            // Sin entradas no hay nada que sacar; sin esto el `while` no
            // terminaría si el mapa quedara vacío con el límite en cero.
            None => break,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entrada(segundos: u64, datos: &str) -> CacheEntry {
        CacheEntry {
            data: datos.to_string(),
            timestamp: SystemTime::UNIX_EPOCH + Duration::from_secs(segundos),
        }
    }

    #[test]
    fn una_entrada_reciente_sigue_valiendo() {
        let ahora = SystemTime::UNIX_EPOCH + Duration::from_secs(1000);
        let guardada = SystemTime::UNIX_EPOCH + Duration::from_secs(900);
        assert!(!is_expired(guardada, ahora, Duration::from_secs(200)));
    }

    #[test]
    fn una_entrada_vieja_expira() {
        let ahora = SystemTime::UNIX_EPOCH + Duration::from_secs(1000);
        let guardada = SystemTime::UNIX_EPOCH + Duration::from_secs(700);
        assert!(is_expired(guardada, ahora, Duration::from_secs(200)));
    }

    #[test]
    fn el_borde_exacto_todavia_vale() {
        // Justo en la duración no expira: expira **pasada** la duración. Da igual
        // para el uso, pero define el comportamiento en lugar de dejarlo al azar.
        let ahora = SystemTime::UNIX_EPOCH + Duration::from_secs(1000);
        let guardada = SystemTime::UNIX_EPOCH + Duration::from_secs(800);
        assert!(!is_expired(guardada, ahora, Duration::from_secs(200)));
    }

    #[test]
    fn una_fecha_del_futuro_cuenta_como_expirada() {
        // Pasa cuando el reloj se corrige hacia atrás. Servir para siempre algo
        // guardado con una fecha imposible sería peor que volver a leerlo.
        let ahora = SystemTime::UNIX_EPOCH + Duration::from_secs(100);
        let guardada = SystemTime::UNIX_EPOCH + Duration::from_secs(5000);
        assert!(is_expired(guardada, ahora, Duration::from_secs(200)));
    }

    #[test]
    fn la_cache_no_pasa_del_techo() {
        // Sin techo, cualquier bucle que pida nombres distintos la hace crecer sin
        // freno, y la clave la elige el WebView.
        let mut cache = HashMap::new();
        for i in 0..20 {
            cache.insert(format!("icono-{i}"), entrada(i, "datos"));
        }
        evict_oldest(&mut cache, 5);
        assert_eq!(cache.len(), 5);
    }

    #[test]
    fn se_saca_primero_lo_mas_viejo() {
        let mut cache = HashMap::new();
        cache.insert("vieja".to_string(), entrada(10, "a"));
        cache.insert("media".to_string(), entrada(20, "b"));
        cache.insert("nueva".to_string(), entrada(30, "c"));

        evict_oldest(&mut cache, 2);
        assert!(!cache.contains_key("vieja"), "salió la más vieja");
        assert!(cache.contains_key("media"));
        assert!(cache.contains_key("nueva"));
    }

    #[test]
    fn por_debajo_del_techo_no_se_toca_nada() {
        let mut cache = HashMap::new();
        cache.insert("una".to_string(), entrada(1, "a"));
        evict_oldest(&mut cache, 10);
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn un_techo_de_cero_no_deja_el_bucle_colgado() {
        // El `while` con el mapa vacío y el techo en cero no terminaría sin el
        // corte: un cuelgue en el hilo que dibuja la interfaz.
        let mut cache = HashMap::new();
        cache.insert("una".to_string(), entrada(1, "a"));
        evict_oldest(&mut cache, 0);
        assert!(cache.is_empty());
        evict_oldest(&mut cache, 0);
    }

    #[test]
    fn un_candado_envenenado_se_recupera() {
        // Si esto no funciona, un pánico en cualquier parte deja al escritorio sin
        // ningún icono hasta reiniciar la sesión.
        let candado = std::sync::Arc::new(Mutex::new(vec![1, 2, 3]));
        let otro = candado.clone();
        let _ = std::thread::spawn(move || {
            let _guardia = otro.lock().unwrap();
            panic!("envenena el candado");
        })
        .join();

        assert!(candado.lock().is_err(), "el candado tiene que estar envenenado");
        assert_eq!(*bloquear(&candado), vec![1, 2, 3]);
    }

    #[test]
    fn el_techo_y_la_duracion_son_los_que_se_documentaron() {
        assert_eq!(LIMITE, 512);
        assert_eq!(CACHE_DURATION, Duration::from_secs(30 * 60));
    }
}
