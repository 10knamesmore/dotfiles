//! Claude Code 与 Codex 共用的本地 hook 规则引擎。
//!
//! `cc-hook` 保留 Claude Code adapter，`agent-hook` 提供中立入口与 Codex adapter；
//! 可测的规则判定与协议输出都在本库。
//! 按 hook 事件分模块：事件专属逻辑住事件目录，跨事件共用的住 [`common`]：
//!
//! - [`common::outcome`]：子命令统一返回值（业务函数不做 IO）
//! - [`common::wire`]：stdout/stderr/审计日志的统一落地
//! - [`pretool`]：共享规则、命令词法、匹配引擎与各 harness JSON 信封
//!
//! 铁律：fail-open——任何解析失败都表现为「无意见」（静默放行），绝不阻断正常命令。

pub mod common;
pub mod pretool;
