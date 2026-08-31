//! 在无副作用 Lua 沙箱中收集 mapping、Resource 与 lifecycle hook declaration。

use std::cell::RefCell;
use std::rc::Rc;

use dots_core::manifest::Manifest;
use mlua::Lua;

use super::{api, to_eyre};

/// 通用 Result 别名。
pub type Result<T> = color_eyre::Result<T>;

/// 注入到 Lua 的只读机器上下文。
#[derive(Clone)]
pub struct LuaCtx {
    /// 当前平台字符串。
    pub os: String,

    /// 当前 `$HOME`。
    pub home: String,

    /// 当前 dotfiles 仓库根。
    pub repo: String,
}

/// 求值 dots.lua 并返回完整 Manifest。
pub fn eval_manifest(source: &str, context: &LuaCtx) -> Result<Manifest> {
    let lua = Lua::new();
    sandbox(&lua).map_err(to_eyre)?;
    inject_context(&lua, context).map_err(to_eyre)?;

    let builder = Rc::new(RefCell::new(Manifest::default()));
    api::register(&lua, &builder).map_err(to_eyre)?;
    lua.load(source)
        .set_name("dots.lua")
        .exec()
        .map_err(to_eyre)?;

    let manifest = builder.borrow().clone();
    Ok(manifest)
}

/// 移除文件、进程和模块加载能力，使 dots.lua 只能声明状态。
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

/// 注入只读 `dots.os/home/repo`。
fn inject_context(lua: &Lua, context: &LuaCtx) -> mlua::Result<()> {
    let dots = lua.create_table()?;
    dots.set("os", context.os.clone())?;
    dots.set("home", context.home.clone())?;
    dots.set("repo", context.repo.clone())?;
    lua.globals().set("dots", dots)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::min_ident_chars, clippy::missing_docs_in_private_items)]

    use dots_core::ResourceDeclaration;

    use super::*;

    /// 构造固定 Lua context。
    fn context() -> LuaCtx {
        LuaCtx {
            os: "linux".to_owned(),
            home: "/home/u".to_owned(),
            repo: "/home/u/dotfiles".to_owned(),
        }
    }

    #[test]
    fn collects_mapping_and_resource_declarations() -> Result<()> {
        let source = r#"
            granularity("home/.config/opencode", { mode = "file", ignore = { "node_modules" } })
            distribute("skills", { src = "tree/home/.agents/skills", to = { "~/.codex/skills" }, mode = "children" })
            dots.resource.systemd_user_unit { unit = "mihomo.service" }
            dots.resource.copied_file { source = "payload", target = "~/bin/payload" }
        "#;
        let manifest = eval_manifest(source, &context())?;
        assert_eq!(manifest.granularity.len(), 1);
        assert_eq!(manifest.distribute.len(), 1);
        assert_eq!(manifest.resources.len(), 2);
        assert!(matches!(
            manifest.resources.first(),
            Some(ResourceDeclaration::SystemdUserUnit { .. })
        ));
        Ok(())
    }

    #[test]
    fn disabled_resource_is_absent_from_desired_set() -> Result<()> {
        let source = r#"
            dots.resource.symlink { source = "a", target = "~/a", enabled = false }
        "#;
        let manifest = eval_manifest(source, &context())?;
        assert!(manifest.resources.is_empty());
        Ok(())
    }

    #[test]
    fn removed_action_api_is_rejected() {
        let result = eval_manifest("on { post_sync = function() end }", &context());
        assert!(result.is_err());
    }
}
