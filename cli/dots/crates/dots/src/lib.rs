//! dots —— dotfiles 管理 CLI（库导出，供 main 与集成测试使用）。

pub mod cli;
pub mod exec;
pub mod hooks;
pub mod inject;
pub mod managed_block;
pub mod realfs;
pub mod reconciliation;
pub mod render;
pub mod state;

pub mod lua;

pub mod cmd;

/// 通用 Result 别名（项目约定：统一 color-eyre）。
pub type Result<T> = color_eyre::Result<T>;
