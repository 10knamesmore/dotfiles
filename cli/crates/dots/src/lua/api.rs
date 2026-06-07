//! 声明阶段 DSL 函数注册：把 Lua 调用翻译进 [`Manifest`]。
//!
//! 钩子/host 块的闭包只存进 registry（记 [`ClosureId`]），不在此执行。

use std::cell::RefCell;
use std::rc::Rc;

use dots_core::manifest::{
    ClosureId, DistributeSpec, GranularitySpec, HookPhase, HookReg, Manifest, RootSpec,
};
use dots_core::types::{LinkMode, RepoPath};
use mlua::{Function, Lua, RegistryKey, Table};

/// mlua Result 别名（本模块全在 Lua 域内，边界转换由调用方做）。
type Result<T> = mlua::Result<T>;

/// 共享的 Manifest builder。
type Builder = Rc<RefCell<Manifest>>;
/// 共享的闭包句柄表。
type Closures = Rc<RefCell<Vec<RegistryKey>>>;

/// 注册全部声明期 DSL 函数到 Lua 全局。
pub fn register(lua: &Lua, builder: &Builder, closures: &Closures) -> Result<()> {
    register_granularity(lua, builder, closures)?;
    register_distribute(lua, builder, closures)?;
    register_root(lua, builder)?;
    register_systemd_user(lua, builder)?;
    register_scripts(lua, builder)?;
    register_on(lua, builder, closures)?;
    register_hosts(lua, builder, closures)?;
    Ok(())
}

/// 从 spec table 提取可选的条目级钩子闭包（`pre`/`post` 字段）。
fn entry_hook(
    lua: &Lua,
    closures: &Closures,
    spec: &Table,
    key: &str,
) -> mlua::Result<Option<ClosureId>> {
    match spec.get::<Option<Function>>(key)? {
        Some(func) => Ok(Some(register_closure(lua, closures, func)?)),
        None => Ok(None),
    }
}

/// 解析 `mode` 字段字符串为 [`LinkMode`]，缺省 `Dir`。
fn parse_mode(mode_str: Option<String>) -> LinkMode {
    match mode_str.as_deref() {
        Some("children") => LinkMode::Children,
        Some("file") => LinkMode::File,
        _ => LinkMode::Dir,
    }
}

/// 读 table 的字符串数组字段。
fn string_seq(table: &Table, key: &str) -> mlua::Result<Vec<String>> {
    match table.get::<Option<Table>>(key)? {
        None => Ok(Vec::new()),
        Some(seq) => seq.sequence_values::<String>().collect(),
    }
}

/// 注册 `granularity` DSL：按路径登记粒度规格（含可选条目级 pre/post 钩子）。
fn register_granularity(lua: &Lua, builder: &Builder, closures: &Closures) -> Result<()> {
    let builder_ref = builder.clone();
    let closures_ref = closures.clone();
    let func = lua.create_function(move |lua, (path, spec): (String, Table)| {
        let mode = parse_mode(spec.get::<Option<String>>("mode")?);
        let ignore = string_seq(&spec, "ignore")?;
        let pre = entry_hook(lua, &closures_ref, &spec, "pre")?;
        let post = entry_hook(lua, &closures_ref, &spec, "post")?;
        builder_ref.borrow_mut().granularity.insert(
            RepoPath::new(path),
            GranularitySpec {
                mode,
                ignore,
                pre,
                post,
            },
        );
        Ok(())
    })?;
    lua.globals().set("granularity", func)?;
    Ok(())
}

/// 注册 `distribute` DSL：登记分发规格（含可选条目级 pre/post 钩子）。
fn register_distribute(lua: &Lua, builder: &Builder, closures: &Closures) -> Result<()> {
    let builder_ref = builder.clone();
    let closures_ref = closures.clone();
    let func = lua.create_function(move |lua, (name, spec): (String, Table)| {
        let src: String = spec.get("src")?;
        let to = string_seq(&spec, "to")?;
        let mode = parse_mode(spec.get::<Option<String>>("mode")?);
        let pre = entry_hook(lua, &closures_ref, &spec, "pre")?;
        let post = entry_hook(lua, &closures_ref, &spec, "post")?;
        builder_ref.borrow_mut().distribute.push(DistributeSpec {
            name,
            src: RepoPath::new(src),
            to,
            mode,
            pre,
            post,
        });
        Ok(())
    })?;
    lua.globals().set("distribute", func)?;
    Ok(())
}

/// 注册 `root` DSL：登记根目录规格。
fn register_root(lua: &Lua, builder: &Builder) -> Result<()> {
    let builder_ref = builder.clone();
    let func = lua.create_function(move |_, (name, spec): (String, Table)| {
        let path: String = spec.get("path")?;
        let os: Option<String> = spec.get("os")?;
        builder_ref
            .borrow_mut()
            .roots
            .push(RootSpec { name, path, os });
        Ok(())
    })?;
    lua.globals().set("root", func)?;
    Ok(())
}

/// 注册 `systemd_user` DSL：追加 systemd user 单元。
fn register_systemd_user(lua: &Lua, builder: &Builder) -> Result<()> {
    let builder_ref = builder.clone();
    let func = lua.create_function(move |_, units: Table| {
        let list: Vec<String> = units
            .sequence_values::<String>()
            .collect::<mlua::Result<_>>()?;
        builder_ref.borrow_mut().systemd_user.extend(list);
        Ok(())
    })?;
    lua.globals().set("systemd_user", func)?;
    Ok(())
}

/// 注册 `scripts` DSL：登记需递归拍平的脚本子目录（子目录默认保持树形）。
fn register_scripts(lua: &Lua, builder: &Builder) -> Result<()> {
    let builder_ref = builder.clone();
    let func = lua.create_function(move |_, spec: Table| {
        // 旧字段显式报错，防止语义反转后被静默无视。
        if spec.contains_key("keep_tree")? {
            return Err(mlua::Error::external(
                "scripts.keep_tree 已移除：子目录现在默认保持树形，需要拍平的子目录改用 ignore_tree",
            ));
        }
        let flat = string_seq(&spec, "ignore_tree")?;
        builder_ref.borrow_mut().scripts_ignore_tree.extend(flat);
        Ok(())
    })?;
    lua.globals().set("scripts", func)?;
    Ok(())
}

/// 注册 `on` DSL：表形式登记生命周期钩子闭包。
///
/// `on { phase = fn }` 或 `on { phase = { fn1, fn2 } }`（同 phase 按数组序登记）。
fn register_on(lua: &Lua, builder: &Builder, closures: &Closures) -> Result<()> {
    let builder_ref = builder.clone();
    let closures_ref = closures.clone();
    let func = lua.create_function(move |lua, table: Table| {
        for pair in table.pairs::<String, mlua::Value>() {
            let (phase_str, value) = pair?;
            let Some(phase) = HookPhase::parse(&phase_str) else {
                return Err(mlua::Error::external(format!(
                    "未知生命周期点：{phase_str}"
                )));
            };
            let funcs: Vec<Function> = match value {
                mlua::Value::Function(func) => vec![func],
                mlua::Value::Table(seq) => seq
                    .sequence_values::<Function>()
                    .collect::<mlua::Result<_>>()
                    .map_err(|_| {
                        mlua::Error::external(format!("on.{phase_str} 的数组元素必须都是函数"))
                    })?,
                other => {
                    return Err(mlua::Error::external(format!(
                        "on.{phase_str} 的值必须是函数或函数数组，得到 {}",
                        other.type_name()
                    )));
                }
            };
            for func in funcs {
                let id = register_closure(lua, &closures_ref, func)?;
                builder_ref
                    .borrow_mut()
                    .hooks
                    .push(HookReg { phase, closure: id });
            }
        }
        Ok(())
    })?;
    lua.globals().set("on", func)?;
    Ok(())
}

/// 把一个 Lua 闭包存进 registry，返回其 [`ClosureId`]。
fn register_closure(lua: &Lua, closures: &Closures, func: Function) -> mlua::Result<ClosureId> {
    let key = lua.create_registry_value(func)?;
    let mut cl = closures.borrow_mut();
    cl.push(key);
    Ok(ClosureId((cl.len() - 1) as u32))
}

/// 注册 `hosts` DSL：登记 per-host 闭包块。
fn register_hosts(lua: &Lua, builder: &Builder, closures: &Closures) -> Result<()> {
    let builder_ref = builder.clone();
    let closures_ref = closures.clone();
    let func = lua.create_function(move |lua, table: Table| {
        for pair in table.pairs::<String, Function>() {
            let (name, func) = pair?;
            let id = register_closure(lua, &closures_ref, func)?;
            builder_ref.borrow_mut().host_blocks.insert(name, id);
        }
        Ok(())
    })?;
    lua.globals().set("hosts", func)?;
    Ok(())
}
