//! Record operation names and outcomes without desktop or input content.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) fn event(operation: &str, outcome: &str) {
    let path = std::env::var_os("COMPUTER_USE_LOG")
        .filter(|value| !value.is_empty())
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("computer-use-sdk.log"));
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let truncate = fs::metadata(&path).is_ok_and(|m| m.len() >= 1_048_576);
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
            .as_secs();
        let _ = writeln!(
            file,
            "{timestamp} pid={} {operation} {outcome}",
            std::process::id()
        );
    }
}

pub(crate) fn result<T>(operation: &str, result: pyo3::PyResult<T>) -> pyo3::PyResult<T> {
    event(operation, if result.is_ok() { "ok" } else { "failed" });
    result
}
