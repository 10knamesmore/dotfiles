//! 声明阶段：mlua 沙箱求值 dots.lua → Manifest + 登记闭包。

use std::cell::RefCell;
use std::rc::Rc;

use dots_core::manifest::Manifest;
use mlua::{Lua, RegistryKey};

use super::{api, to_eyre};

/// 通用 Result 别名。
pub type Result<T> = color_eyre::Result<T>;

/// 注入到 Lua 的只读上下文。
#[derive(Clone)]
pub struct LuaCtx {
    /// 当前主机名（`dots.host`）。
    pub host: String,
    /// 当前平台字符串（`dots.os`，`"linux"`/`"macos"`）。
    pub os: String,
    /// `$HOME`（`dots.home`）。
    pub home: String,
}

/// 持有 mlua 运行时与登记的闭包（effect 阶段查回执行）。
pub struct LuaHandles {
    /// mlua 运行时（含已 create 的闭包）。
    pub lua: Lua,
    /// `ClosureId(i)` → registry key（闭包句柄）。
    pub closures: Vec<RegistryKey>,
}

/// 求值 dots.lua。
///
/// # Params:
///   - `src`: dots.lua 源码
///   - `ctx`: 注入的只读上下文
///
/// # Return:
///   `(Manifest, LuaHandles)`：声明数据 + 闭包句柄。
pub fn eval_manifest(src: &str, ctx: &LuaCtx) -> Result<(Manifest, LuaHandles)> {
    let lua = Lua::new();
    sandbox(&lua).map_err(to_eyre)?;
    inject_context(&lua, ctx).map_err(to_eyre)?;

    let builder = Rc::new(RefCell::new(Manifest::default()));
    let closures = Rc::new(RefCell::new(Vec::<RegistryKey>::new()));
    api::register(&lua, &builder, &closures).map_err(to_eyre)?;

    lua.load(src).set_name("dots.lua").exec().map_err(to_eyre)?;

    let manifest = builder.borrow().clone();
    // 注：DSL 闭包仍捕获着 closures 的 Rc clone（存活于 lua 内），故不能 try_unwrap；
    // 直接把已登记的 RegistryKey 取走（留空 Vec），effect 期经 lua.registry_value 查回。
    let closures = std::mem::take(&mut *closures.borrow_mut());
    Ok((manifest, LuaHandles { lua, closures }))
}

/// 沙箱：移除有副作用/可逃逸的 stdlib，保证声明阶段纯净、确定。
fn sandbox(lua: &Lua) -> mlua::Result<()> {
    let globals = lua.globals();
    for name in [
        "io",
        "os",
        "require",
        "dofile",
        "loadfile",
        "load",
        "loadstring",
        "package",
    ] {
        globals.set(name, mlua::Value::Nil)?;
    }
    Ok(())
}

/// 注入只读 `dots.{host,os,home}`。
fn inject_context(lua: &Lua, ctx: &LuaCtx) -> mlua::Result<()> {
    let dots = lua.create_table()?;
    dots.set("host", ctx.host.clone())?;
    dots.set("os", ctx.os.clone())?;
    dots.set("home", ctx.home.clone())?;
    lua.globals().set("dots", dots)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::min_ident_chars, clippy::missing_docs_in_private_items)]
    use super::*;

    fn ctx() -> LuaCtx {
        LuaCtx {
            host: "xz07".into(),
            os: "linux".into(),
            home: "/home/u".into(),
        }
    }

    #[test]
    fn collects_granularity_and_distribute() -> Result<()> {
        let src = r#"
            granularity("home/.config/opencode", { mode = "file", ignore = { "node_modules" } })
            distribute("skills", { src = "tree/home/.claude/skills",
                                   to = { "~/.codex/skills" }, mode = "children" })
            systemd_user { "mihomo.service", "napcat.service" }
            scripts { keep_tree = { "hypr" } }
        "#;
        let (m, _h) = eval_manifest(src, &ctx())?;
        assert_eq!(m.granularity.len(), 1);
        assert_eq!(m.distribute.len(), 1);
        assert_eq!(m.distribute[0].to, vec!["~/.codex/skills".to_string()]);
        assert_eq!(m.systemd_user.len(), 2);
        assert_eq!(m.scripts_keep_tree, vec!["hypr".to_string()]);
        Ok(())
    }

    #[test]
    fn registers_hook_without_running() -> Result<()> {
        // 钩子体内若执行会调用未定义的 dots.json（声明期未注册），故能跑通即证明未执行。
        let src = r#"
            on("post_link", function() dots.json.merge("/x", {}) end)
        "#;
        let (m, h) = eval_manifest(src, &ctx())?;
        assert_eq!(m.hooks.len(), 1);
        assert_eq!(h.closures.len(), 1);
        Ok(())
    }

    #[test]
    fn sandbox_blocks_io_and_os() {
        let bad = r#" io.open("/etc/passwd") "#;
        assert!(eval_manifest(bad, &ctx()).is_err());
        let bad2 = r#" os.execute("rm -rf /") "#;
        assert!(eval_manifest(bad2, &ctx()).is_err());
    }

    #[test]
    fn deterministic() -> Result<()> {
        let src = r#" systemd_user { "a.service" } "#;
        let (m1, _) = eval_manifest(src, &ctx())?;
        let (m2, _) = eval_manifest(src, &ctx())?;
        assert_eq!(m1, m2);
        Ok(())
    }

    #[test]
    fn hosts_block_registers_closure() -> Result<()> {
        let src = r#"
            hosts { xz07 = function() vars { backlight = "amdgpu_bl1" } end }
        "#;
        let (m, _h) = eval_manifest(src, &ctx())?;
        assert!(m.host_blocks.contains_key("xz07"));
        Ok(())
    }
}
