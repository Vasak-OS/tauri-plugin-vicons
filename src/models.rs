use std::time::SystemTime;

/// Estructura para almacenar datos en caché
#[derive(Clone, Debug)]
pub struct CacheEntry {
    pub data: String,
    pub timestamp: SystemTime,
}