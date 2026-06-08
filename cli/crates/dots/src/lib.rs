//! dots —— dotfiles 管理 CLI（库导出，供 main 与集成测试使用）。

pub mod cli;
pub mod exec;
pub mod hooks;
pub mod hosts;
pub mod inject;
pub mod onboard;
pub mod realfs;
pub mod render;
pub mod secret;
pub mod shellenv;
pub mod state;

pub mod lua;

pub mod cmd;

/// 通用 Result 别名（项目约定：统一 color-eyre）。
pub type Result<T> = color_eyre::Result<T>;

/// 生成 ISO 风格时间戳（用于备份目录名）。集中一处便于一致性。
pub fn timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|dur| dur.as_secs())
        .unwrap_or(0);
    format!("ts-{secs}")
}
