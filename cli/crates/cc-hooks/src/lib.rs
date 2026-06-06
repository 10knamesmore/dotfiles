//! Claude Code hooks 统一入口的引擎库。
//!
//! bin（`cc-hook`）只做 clap 分发与 stdin/stdout 管道；可测逻辑全在本库：
//!
//! - [`envelope`]：hook JSON 边界（stdin 解析 / `hookSpecificOutput` 渲染）
//! - [`argv`]：链式命令切段 + shlex 分词 + 短旗标收集
//! - [`rules`]：bash-guard 规则的 TOML schema
//! - [`engine`]：规则匹配
//!
//! 铁律：fail-open——任何解析失败都表现为「无意见」（静默放行），绝不阻断正常命令。

pub mod argv;
pub mod engine;
pub mod envelope;
pub mod rules;
