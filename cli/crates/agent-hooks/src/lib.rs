//! 多个 coding agent 共用的本地 hook 规则引擎。
//!
//! `agent-hook` 通过 harness adapter 处理协议差异，共享规则判定与输出基础类型。
//! 按 hook 事件分模块：事件专属逻辑住事件目录，跨事件共用的住 [`common`]：
//!
//! - [`common::outcome`]：子命令统一返回值（业务函数不做 IO）
//! - [`common::wire`]：stdout、stderr 与 exit code 的统一落地
//! - [`pretool`]：共享规则、命令词法、匹配引擎与各 harness JSON 信封
//!
//! 解析失败表现为「无意见」，避免启发式守卫阻断正常工具调用。

pub mod common;
pub mod pretool;
