use std::fs::{self, File, OpenOptions};
use std::io::{LineWriter, Write};
use std::path::Path;
use std::sync::{LazyLock, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

static LOG_FILE: LazyLock<Mutex<Option<LineWriter<File>>>> = LazyLock::new(|| Mutex::new(None));

pub fn init(path: &Path) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map(LineWriter::new)
        .ok();
    *LOG_FILE.lock().unwrap() = file;
    info("Logger initialized");
}

pub fn info(msg: &str) {
    log("INFO", msg);
}

pub fn warn(msg: &str) {
    log("WARN", msg);
}

pub fn error(msg: &str) {
    log("ERROR", msg);
}

fn log(level: &str, msg: &str) {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    if let Some(f) = LOG_FILE.lock().unwrap().as_mut() {
        let _ = writeln!(f, "[{ts}] [{level}] {msg}");
    }
}
