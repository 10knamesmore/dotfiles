//! dots.lua 求值子系统。
//!
//! 两阶段（§4 / §6.4）：
//! - **声明阶段**（[`eval`]）：mlua 沙箱求值 dots.lua，收集 [`dots_core::Manifest`]；
//!   钩子/host 块只登记闭包（存 RegistryKey），不执行。
//! - **effect 阶段**（[`crate::hooks`]）：链接落盘后执行闭包，期间注册写原语
//!   [`primitives`]，副作用经受控上下文落盘。

pub mod api;
pub mod eval;
pub mod primitives;

pub use eval::{LuaCtx, LuaHandles, eval_manifest};

/// 把 `mlua::Error`（无 Send+Sync）转成 color-eyre Report，用于 mlua↔color-eyre 边界。
pub fn to_eyre(err: mlua::Error) -> color_eyre::Report {
    color_eyre::eyre::eyre!("lua: {err}")
}
