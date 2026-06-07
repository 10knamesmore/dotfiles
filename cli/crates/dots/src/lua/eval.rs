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
    /// 仓库根（`dots.repo`）。
    pub repo: String,
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

/// 注入只读 `dots.{host,os,home,repo}`。
fn inject_context(lua: &Lua, ctx: &LuaCtx) -> mlua::Result<()> {
    let dots = lua.create_table()?;
    dots.set("host", ctx.host.clone())?;
    dots.set("os", ctx.os.clone())?;
    dots.set("home", ctx.home.clone())?;
    dots.set("repo", ctx.repo.clone())?;
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
            repo: "/home/u/dotfiles".into(),
        }
    }

    #[test]
    fn collects_granularity_and_distribute() -> Result<()> {
        let src = r#"
            granularity("home/.config/opencode", { mode = "file", ignore = { "node_modules" } })
            distribute("skills", { src = "tree/home/.claude/skills",
                                   to = { "~/.codex/skills" }, mode = "children" })
            systemd_user { "mihomo.service", "napcat.service" }
            scripts { ignore_tree = { "snippets" } }
        "#;
        let (m, _h) = eval_manifest(src, &ctx())?;
        assert_eq!(m.granularity.len(), 1);
        assert_eq!(m.distribute.len(), 1);
        assert_eq!(m.distribute[0].to, vec!["~/.codex/skills".to_string()]);
        assert_eq!(m.systemd_user.len(), 2);
        assert_eq!(m.scripts_ignore_tree, vec!["snippets".to_string()]);
        Ok(())
    }

    #[test]
    fn scripts_old_keep_tree_field_errors() {
        // 语义已反转（默认保树）：旧字段必须报错，不能静默无视。
        let src = r#" scripts { keep_tree = { "hypr" } } "#;
        assert!(
            eval_manifest(src, &ctx()).is_err(),
            "keep_tree 应报迁移错误"
        );
    }

    #[test]
    fn registers_hook_without_running() -> Result<()> {
        // 钩子体内若执行会调用未定义的 dots.json（声明期未注册），故能跑通即证明未执行。
        let src = r#"
            on { post_link = function() dots.json.merge("/x", {}) end }
        "#;
        let (m, h) = eval_manifest(src, &ctx())?;
        assert_eq!(m.hooks.len(), 1);
        assert_eq!(h.closures.len(), 1);
        Ok(())
    }

    #[test]
    fn on_table_registers_multiple_phases() -> Result<()> {
        use dots_core::manifest::HookPhase;
        let src = r#"
            on {
                pre_sync = function() end,
                post_sync = function() end,
            }
        "#;
        let (m, h) = eval_manifest(src, &ctx())?;
        assert_eq!(m.hooks.len(), 2);
        assert_eq!(h.closures.len(), 2);
        let phases: Vec<_> = m.hooks.iter().map(|reg| reg.phase).collect();
        assert!(phases.contains(&HookPhase::PreSync));
        assert!(phases.contains(&HookPhase::PostSync));
        Ok(())
    }

    #[test]
    fn on_table_array_value_registers_in_order() -> Result<()> {
        use dots_core::manifest::HookPhase;
        let src = r#"
            on { post_sync = { function() end, function() end } }
        "#;
        let (m, h) = eval_manifest(src, &ctx())?;
        assert_eq!(m.hooks.len(), 2);
        assert_eq!(h.closures.len(), 2);
        assert!(m.hooks.iter().all(|reg| reg.phase == HookPhase::PostSync));
        // 同 phase 多钩子按数组下标序登记（ClosureId 递增）
        let ids: Vec<u32> = m.hooks.iter().map(|reg| reg.closure.0).collect();
        assert_eq!(ids, vec![0, 1]);
        Ok(())
    }

    #[test]
    fn on_unknown_phase_errors() {
        let src = r#" on { nope = function() end } "#;
        assert!(eval_manifest(src, &ctx()).is_err(), "未知 phase 应报错");
    }

    #[test]
    fn on_non_function_value_errors() {
        let src = r#" on { post_sync = 42 } "#;
        assert!(eval_manifest(src, &ctx()).is_err(), "非函数 value 应报错");
    }

    #[test]
    fn on_array_with_non_function_errors() {
        let src = r#" on { post_sync = { function() end, "oops" } } "#;
        assert!(eval_manifest(src, &ctx()).is_err(), "数组内混非函数应报错");
    }

    #[test]
    fn on_old_two_arg_form_errors() {
        // 旧式 on(phase, fn) 已删除，应报错而非静默接受。
        let src = r#" on("post_sync", function() end) "#;
        assert!(eval_manifest(src, &ctx()).is_err(), "旧式双参应报错");
    }

    #[test]
    fn granularity_pre_post_registered_without_running() -> Result<()> {
        // pre/post 体内调用声明期未注册的 dots.json —— 能跑通即证明只登记不执行。
        let src = r#"
            granularity("home/.claude", {
                mode = "children",
                pre = function() dots.json.merge("/x", {}) end,
                post = function() dots.json.merge("/y", {}) end,
            })
        "#;
        let (m, h) = eval_manifest(src, &ctx())?;
        let spec = m
            .granularity
            .get(&dots_core::types::RepoPath::new("home/.claude"))
            .ok_or_else(|| color_eyre::eyre::eyre!("granularity 条目缺失"))?;
        assert!(spec.pre.is_some(), "pre 闭包应已登记");
        assert!(spec.post.is_some(), "post 闭包应已登记");
        assert_eq!(h.closures.len(), 2);
        Ok(())
    }

    #[test]
    fn granularity_without_hooks_has_none() -> Result<()> {
        let src = r#" granularity("home/.config/opencode", { mode = "file" }) "#;
        let (m, _h) = eval_manifest(src, &ctx())?;
        let spec = m
            .granularity
            .get(&dots_core::types::RepoPath::new("home/.config/opencode"))
            .ok_or_else(|| color_eyre::eyre::eyre!("granularity 条目缺失"))?;
        assert!(spec.pre.is_none());
        assert!(spec.post.is_none());
        Ok(())
    }

    #[test]
    fn distribute_pre_post_registered() -> Result<()> {
        let src = r#"
            distribute("skills", {
                src = "tree/home/.claude/skills",
                to = { "~/.codex/skills" },
                mode = "children",
                pre = function() end,
                post = function() end,
            })
        "#;
        let (m, h) = eval_manifest(src, &ctx())?;
        let spec = m
            .distribute
            .first()
            .ok_or_else(|| color_eyre::eyre::eyre!("distribute 条目缺失"))?;
        assert!(spec.pre.is_some());
        assert!(spec.post.is_some());
        assert_eq!(h.closures.len(), 2);
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
