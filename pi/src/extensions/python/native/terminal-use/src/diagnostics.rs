//! Bounded operational diagnostics. Terminal bytes and input are deliberately excluded.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_LOG_BYTES: u64 = 1_048_576;

struct Diagnostics {
    path: PathBuf,
}

pub(crate) fn event(session_id: &str, message: &str) {
    static LOGGER: OnceLock<Mutex<Diagnostics>> = OnceLock::new();
    let logger = LOGGER.get_or_init(|| Mutex::new(Diagnostics { path: log_path() }));
    let Ok(logger) = logger.lock() else {
        return;
    };

    let safe_message = message.replace(['\r', '\n'], " ");
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs());
    let line = format!("{timestamp} {session_id} {safe_message}\n");

    if let Ok(metadata) = fs::metadata(&logger.path)
        && metadata.len() >= MAX_LOG_BYTES
    {
        let _ = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&logger.path);
    }

    if let Some(parent) = logger.path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&logger.path)
    {
        let remaining = MAX_LOG_BYTES.saturating_sub(file.metadata().map_or(0, |m| m.len()));
        let bytes = line.as_bytes();
        let length = usize::try_from(remaining).unwrap_or(0).min(bytes.len());
        let _ = file.write_all(&bytes[..length]);
    }
}

fn log_path() -> PathBuf {
    if let Some(path) = std::env::var_os("TERMINAL_USE_LOG")
        && !path.is_empty()
    {
        return PathBuf::from(path);
    }
    std::env::temp_dir().join("terminal-use-sdk.log")
}
