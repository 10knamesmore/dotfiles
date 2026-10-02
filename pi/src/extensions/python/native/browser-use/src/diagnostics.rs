//! Log browser operations without page text, URLs, selectors, input or endpoint tokens.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) fn event(operation: &str, outcome: &str) {
    static LOCK: Mutex<()> = Mutex::new(());
    let Ok(_guard) = LOCK.lock() else { return };
    let path = std::env::var_os("BROWSER_USE_LOG")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::env::temp_dir().join(format!("browser-use-{}.log", std::process::id()))
        });
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let truncate = fs::metadata(&path).is_ok_and(|metadata| metadata.len() >= 1_048_576);
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .write(true)
        .append(!truncate)
        .truncate(truncate)
        .open(path)
    {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let _ = writeln!(file, "{timestamp} {operation} {outcome}");
    }
}
