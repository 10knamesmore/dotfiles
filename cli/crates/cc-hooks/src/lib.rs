//! Claude Code hooks 统一入口的引擎库。
//!
//! bin（`cc-hook`）只做 clap 分发与统一 wire（stdout/stderr/exit）；可测逻辑全在本库。
//! 按 hook 事件分模块：事件专属逻辑住事件目录，跨事件共用的住 [`common`]：
//!
//! - [`common::outcome`]：子命令统一返回值（业务函数不做 IO）
//! - [`pretool`]：PreToolUse 事件（规则 TOML schema / 命令词法 / 匹配引擎 / JSON 信封）
//!
//! 铁律：fail-open——任何解析失败都表现为「无意见」（静默放行），绝不阻断正常命令。

pub mod common;
pub mod pretool;
