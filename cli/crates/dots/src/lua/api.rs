//! 纯声明 Lua API：mapping、sync Resource、lifecycle hook 与 Cargo installation。

use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

use dots_core::manifest::{
    BeforeSyncHook, CargoBinaryDeclaration, DistributeSpec, GranularitySpec, Manifest,
    ResourceDeclaration, RootSpec,
};
use dots_core::{LinkMode, RepoPath};
use mlua::{Lua, Table, Value};

/// 共享 Manifest builder。
pub type Builder = Rc<RefCell<Manifest>>;

/// 注册 dots.lua 的全部声明 API。
pub fn register(lua: &Lua, builder: &Builder) -> mlua::Result<()> {
    register_granularity(lua, builder)?;
    register_distribute(lua, builder)?;
    register_root(lua, builder)?;
    register_scripts(lua, builder)?;
    register_resources(lua, builder)?;
    register_hooks(lua, builder)?;
    register_path_queries(lua)?;
    Ok(())
}

/// 注册只在 `dots sync` 中执行的 lifecycle hook。
fn register_hooks(lua: &Lua, builder: &Builder) -> mlua::Result<()> {
    let dots: Table = lua.globals().get("dots")?;
    let hook = lua.create_table()?;
    let before_sync_builder = builder.clone();
    hook.set(
        "before_sync",
        lua.create_function(move |_, spec: Table| {
            if enabled(&spec)? {
                before_sync_builder
                    .borrow_mut()
                    .hooks
                    .before_sync
                    .push(BeforeSyncHook {
                        name: spec.get("name")?,
                        cwd: spec.get("cwd")?,
                        program: spec.get("program")?,
                        args: string_sequence(&spec, "args")?,
                    });
            }
            Ok(())
        })?,
    )?;
    dots.set("hook", hook)?;
    Ok(())
}

/// 注册链接粒度覆盖；`pre` 或 `post` 字段会报错。
fn register_granularity(lua: &Lua, builder: &Builder) -> mlua::Result<()> {
    let builder_ref = builder.clone();
    let function = lua.create_function(move |_, (path, spec): (String, Table)| {
        reject_action_fields(&spec, "granularity")?;
        let mode = parse_mode(spec.get::<Option<String>>("mode")?)?;
        let ignore = string_sequence(&spec, "ignore")?;
        builder_ref
            .borrow_mut()
            .granularity
            .insert(RepoPath::new(path), GranularitySpec { mode, ignore });
        Ok(())
    })?;
    lua.globals().set("granularity", function)?;
    Ok(())
}

/// 注册一源多落点 mapping declaration。
fn register_distribute(lua: &Lua, builder: &Builder) -> mlua::Result<()> {
    let builder_ref = builder.clone();
    let function = lua.create_function(move |_, (name, spec): (String, Table)| {
        reject_action_fields(&spec, "distribute")?;
        let source: String = spec.get("src")?;
        let targets = string_sequence(&spec, "to")?;
        let mode = parse_mode(spec.get::<Option<String>>("mode")?)?;
        builder_ref.borrow_mut().distribute.push(DistributeSpec {
            name,
            src: RepoPath::new(source),
            to: targets,
            mode,
        });
        Ok(())
    })?;
    lua.globals().set("distribute", function)?;
    Ok(())
}

/// 注册非 `$HOME` tree layer target root。
fn register_root(lua: &Lua, builder: &Builder) -> mlua::Result<()> {
    let builder_ref = builder.clone();
    let function = lua.create_function(move |_, (name, spec): (String, Table)| {
        let path: String = spec.get("path")?;
        let os: Option<String> = spec.get("os")?;
        if os
            .as_deref()
            .is_some_and(|value| !matches!(value, "linux" | "macos"))
        {
            return Err(mlua::Error::external("root.os 只接受 `linux` 或 `macos`"));
        }
        builder_ref
            .borrow_mut()
            .roots
            .push(RootSpec { name, path, os });
        Ok(())
    })?;
    lua.globals().set("root", function)?;
    Ok(())
}

/// 注册 scripts 聚合设置。
fn register_scripts(lua: &Lua, builder: &Builder) -> mlua::Result<()> {
    let builder_ref = builder.clone();
    let function = lua.create_function(move |_, spec: Table| {
        if spec.contains_key("keep_tree")? {
            return Err(mlua::Error::external(
                "scripts.keep_tree 不受支持：子目录默认保持树形，需要拍平的子目录使用 ignore_tree",
            ));
        }
        builder_ref
            .borrow_mut()
            .scripts_ignore_tree
            .extend(string_sequence(&spec, "ignore_tree")?);
        Ok(())
    })?;
    lua.globals().set("scripts", function)?;
    Ok(())
}

/// 注册 `dots.resource.*` typed declaration，包括独立执行的 Cargo installation。
fn register_resources(lua: &Lua, builder: &Builder) -> mlua::Result<()> {
    let dots: Table = lua.globals().get("dots")?;
    let resource = lua.create_table()?;

    let symlink_builder = builder.clone();
    resource.set(
        "symlink",
        lua.create_function(move |_, spec: Table| {
            if enabled(&spec)? {
                symlink_builder
                    .borrow_mut()
                    .resources
                    .push(ResourceDeclaration::Symlink {
                        source: spec.get("source")?,
                        target: spec.get("target")?,
                    });
            }
            Ok(())
        })?,
    )?;

    let copied_file_builder = builder.clone();
    resource.set(
        "copied_file",
        lua.create_function(move |_, spec: Table| {
            if enabled(&spec)? {
                copied_file_builder
                    .borrow_mut()
                    .resources
                    .push(ResourceDeclaration::CopiedFile {
                        source: spec.get("source")?,
                        target: spec.get("target")?,
                    });
            }
            Ok(())
        })?,
    )?;

    let cargo_builder = builder.clone();
    resource.set(
        "cargo_binary",
        lua.create_function(move |_, spec: Table| {
            if enabled(&spec)? {
                cargo_builder
                    .borrow_mut()
                    .cargo_binaries
                    .push(parse_cargo_binary_declaration(&spec)?);
            }
            Ok(())
        })?,
    )?;

    let block_builder = builder.clone();
    resource.set(
        "managed_block",
        lua.create_function(move |_, spec: Table| {
            if enabled(&spec)? {
                block_builder
                    .borrow_mut()
                    .resources
                    .push(ResourceDeclaration::ManagedBlock {
                        target: spec.get("target")?,
                        marker: spec.get("marker")?,
                        content: spec.get("content")?,
                    });
            }
            Ok(())
        })?,
    )?;

    let systemd_builder = builder.clone();
    resource.set(
        "systemd_user_unit",
        lua.create_function(move |_, spec: Table| {
            if enabled(&spec)? {
                systemd_builder
                    .borrow_mut()
                    .resources
                    .push(ResourceDeclaration::SystemdUserUnit {
                        unit: spec.get("unit")?,
                    });
            }
            Ok(())
        })?,
    )?;

    dots.set("resource", resource)?;
    Ok(())
}

/// 注册声明阶段允许的只读 path 查询。
fn register_path_queries(lua: &Lua) -> mlua::Result<()> {
    let dots: Table = lua.globals().get("dots")?;
    let path = lua.create_table()?;
    path.set(
        "exists",
        lua.create_function(|_, raw: String| Ok(Path::new(&raw).exists()))?,
    )?;
    dots.set("path", path)?;
    Ok(())
}

/// 解析 Resource、lifecycle hook 与 Cargo binary 共用的 `enabled` 字段。
fn enabled(spec: &Table) -> mlua::Result<bool> {
    Ok(spec.get::<Option<bool>>("enabled")?.unwrap_or(true))
}

/// 解析 cargo binary 的 workspace/crates.io declaration。
fn parse_cargo_binary_declaration(spec: &Table) -> mlua::Result<CargoBinaryDeclaration> {
    match spec.get::<Value>("source")? {
        Value::Table(source) => Ok(CargoBinaryDeclaration::Workspace {
            path: source.get("path")?,
            binary: source.get("binary")?,
            root: spec.get("root")?,
        }),
        Value::String(package) => {
            if spec.contains_key("target")?
                || spec.contains_key("root")?
                || spec.contains_key("version")?
                || spec.contains_key("binary")?
            {
                return Err(mlua::Error::external(
                    "crates.io cargo binary 只接受字符串 source；root、target、version、binary 由 Cargo 决定",
                ));
            }
            Ok(CargoBinaryDeclaration::CratesIo {
                package: package.to_str()?.to_owned(),
                binaries: string_sequence(spec, "binaries")?,
            })
        }
        other => Err(mlua::Error::external(format!(
            "dots.resource.cargo_binary.source 必须是 crates.io package 字符串或 workspace table，得到 {}",
            other.type_name()
        ))),
    }
}

/// 拒绝不受支持的条目级 Action 字段，避免静默失效。
fn reject_action_fields(spec: &Table, api: &str) -> mlua::Result<()> {
    if spec.contains_key("pre")? || spec.contains_key("post")? {
        return Err(mlua::Error::external(format!(
            "{api} 不接受 pre/post：sync 只接受声明式 Resource"
        )));
    }
    Ok(())
}

/// 解析链接粒度；缺省为整目录。
fn parse_mode(mode: Option<String>) -> mlua::Result<LinkMode> {
    match mode.as_deref() {
        Some("children") => Ok(LinkMode::Children),
        Some("file") => Ok(LinkMode::File),
        Some("dir") | None => Ok(LinkMode::Dir),
        Some(other) => Err(mlua::Error::external(format!(
            "未知链接粒度 `{other}`；只接受 dir、children、file"
        ))),
    }
}

/// 读取 table 的字符串数组字段。
fn string_sequence(table: &Table, key: &str) -> mlua::Result<Vec<String>> {
    match table.get::<Option<Table>>(key)? {
        None => Ok(Vec::new()),
        Some(sequence) => sequence.sequence_values::<String>().collect(),
    }
}
