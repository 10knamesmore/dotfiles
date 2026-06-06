//! 当前主机名解析。

use std::fs;

/// 取当前 hostname。
///
/// 优先读 `/etc/hostname`，回退 `HOSTNAME` 环境变量，再回退 `unknown`。
/// 不依赖外部命令，便于离线/测试。
pub fn current() -> String {
    if let Ok(text) = fs::read_to_string("/etc/hostname") {
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            return trimmed.to_owned();
        }
    }
    std::env::var("HOSTNAME").unwrap_or_else(|_| "unknown".to_owned())
}
