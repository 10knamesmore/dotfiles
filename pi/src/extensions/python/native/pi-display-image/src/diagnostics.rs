//! Record bounded operational metadata without image data, paths, or exception text.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_LOG_BYTES: u64 = 1_048_576;
static LOG_LOCK: Mutex<()> = Mutex::new(());

pub(crate) fn event(message: &str) {
    let Ok(_guard) = LOG_LOCK.lock() else {
        return;
    };
    let path = std::env::var_os("PI_DISPLAY_IMAGE_LOG")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("pi-display-image-sdk.log"));
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let truncate = fs::metadata(&path).is_ok_and(|metadata| metadata.len() >= MAX_LOG_BYTES);
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .write(true)
        .append(!truncate)
        .truncate(truncate)
        .open(path)
    {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_secs());
        let _ = writeln!(file, "{timestamp} {message}");
    }
}
