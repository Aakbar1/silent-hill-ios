// SPDX-License-Identifier: GPL-3.0-only
use std::{
    fs::{File, OpenOptions},
    io::Write,
    path::PathBuf,
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

struct FileLogger(Mutex<File>);
impl log::Log for FileLogger {
    fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
        metadata.level() <= log::Level::Warn
            || (metadata.target().starts_with("silenthill_shell")
                && metadata.level() <= log::Level::Info)
    }
    fn log(&self, record: &log::Record<'_>) {
        if !self.enabled(record.metadata()) {
            return;
        }
        if let Ok(mut file) = self.0.lock() {
            let time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let _ = writeln!(file, "{time} {} {}", record.level(), record.args());
            let _ = file.flush();
        }
    }
    fn flush(&self) {
        if let Ok(mut file) = self.0.lock() {
            let _ = file.flush();
        }
    }
}

pub fn init() -> Result<(), Box<dyn std::error::Error>> {
    // On iOS HOME is the app sandbox, not the user's desktop home.
    #[cfg(target_os = "ios")]
    let documents = PathBuf::from(std::env::var("HOME")?).join("Documents");
    #[cfg(not(target_os = "ios"))]
    let documents = std::env::var_os("SHELL_DOCUMENTS")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::var_os("USERPROFILE")
                .or_else(|| std::env::var_os("HOME"))
                .map(PathBuf::from)
                .expect("home directory")
                .join("Documents")
                .join("SilentHillPort")
        });
    std::fs::create_dir_all(&documents)?;
    let path = documents.join("shell.log");
    // Bound previous sessions so a long touch test cannot grow the log indefinitely.
    if std::fs::metadata(&path).is_ok_and(|m| m.len() > 2 * 1024 * 1024) {
        let backup = documents.join("shell.previous.log");
        if backup.exists() {
            std::fs::remove_file(&backup)?;
        }
        std::fs::rename(&path, backup)?;
    }
    let logger = Box::leak(Box::new(FileLogger(Mutex::new(
        OpenOptions::new().create(true).append(true).open(path)?,
    ))));
    log::set_logger(logger).map_err(|e| e.to_string())?;
    log::set_max_level(log::LevelFilter::Info);
    Ok(())
}
