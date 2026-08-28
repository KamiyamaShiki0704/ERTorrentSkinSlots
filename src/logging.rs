use std::{
    fs::OpenOptions,
    io::Write,
    path::PathBuf,
    sync::{Mutex, OnceLock},
};

static LOG_PATH: OnceLock<Option<PathBuf>> = OnceLock::new();
static LOG_LOCK: Mutex<()> = Mutex::new(());

pub fn initialize(path: Option<PathBuf>) {
    let _ = LOG_PATH.set(path);
}

pub fn write(message: impl AsRef<str>) {
    let Some(path) = LOG_PATH.get().and_then(Option::as_ref) else {
        return;
    };
    let Ok(_guard) = LOG_LOCK.lock() else {
        return;
    };
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{}", message.as_ref());
    }
}
