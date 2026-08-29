//! dots.lua 声明求值：Lua 只构造 Manifest，不直接写机器状态。

pub mod api;
pub mod eval;

pub use eval::{LuaCtx, eval_manifest};

/// 把 `mlua::Error`（无 Send+Sync）转成 color-eyre Report，用于 mlua↔color-eyre 边界。
pub fn to_eyre(err: mlua::Error) -> color_eyre::Report {
    color_eyre::eyre::eyre!("lua: {err}")
}
