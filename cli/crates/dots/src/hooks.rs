//! 生命周期钩子调度（effect 阶段，§6.4）。
//!
//! 链接落盘后按阶段执行已登记闭包；执行前 [`crate::lua::primitives::install`] 注册写原语，
//! 副作用经 [`EffectState`] 受控落盘/收集。

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use dots_core::manifest::{HookPhase, Manifest};
use mlua::Function;
use rustc_hash::FxHashMap;

use crate::lua::primitives;
use crate::lua::{LuaHandles, to_eyre};
use crate::state::State;

/// 通用 Result 别名。
pub type Result<T> = color_eyre::Result<T>;

/// effect 阶段共享可变上下文：原语经它落盘/收集。
pub struct EffectState {
    /// 仓库根。
    pub repo_root: PathBuf,
    /// `$HOME`。
    pub home: PathBuf,
    /// 是否 dry-run（读-改-写原语在 dry-run 下只声明意图）。
    pub dry_run: bool,
    /// 备份时间戳。
    pub stamp: String,
    /// `vars()` 收集的 per-host 变量（供 .inject 渲染）。
    pub host_vars: FxHashMap<String, String>,
    /// `link()` 收集的额外链接 `(source_abs, target_abs)`。
    pub extra_links: Vec<(PathBuf, PathBuf)>,
    /// 台账（run_once / ownership）。
    pub state: State,
}

impl EffectState {
    /// 新建。
    pub fn new(
        repo_root: PathBuf,
        home: PathBuf,
        stamp: String,
        dry_run: bool,
        state: State,
    ) -> Self {
        Self {
            repo_root,
            home,
            dry_run,
            stamp,
            host_vars: FxHashMap::default(),
            extra_links: Vec::new(),
            state,
        }
    }
}

/// 执行某阶段的全部钩子。
pub fn run_phase(
    phase: HookPhase,
    manifest: &Manifest,
    handles: &LuaHandles,
    effect: &Rc<RefCell<EffectState>>,
) -> Result<()> {
    primitives::install(&handles.lua, effect).map_err(to_eyre)?;
    for reg in manifest.hooks.iter().filter(|hook| hook.phase == phase) {
        let key = handles
            .closures
            .get(reg.closure.0 as usize)
            .ok_or_else(|| color_eyre::eyre::eyre!("钩子闭包 id 越界"))?;
        let func: Function = handles.lua.registry_value(key).map_err(to_eyre)?;
        func.call::<()>(()).map_err(to_eyre)?;
    }
    Ok(())
}

/// 执行一个条目级钩子闭包（granularity/distribute 的 pre/post）。
///
/// # Return:
///   仅当闭包显式返回 `false` 时为 `false`（pre 的阻止语义）；
///   其他返回值（含 nil/true）均为 `true`。post 调用方忽略返回值。
pub fn call_entry_hook(
    id: dots_core::manifest::ClosureId,
    handles: &LuaHandles,
    effect: &Rc<RefCell<EffectState>>,
) -> Result<bool> {
    primitives::install(&handles.lua, effect).map_err(to_eyre)?;
    let key = handles
        .closures
        .get(id.0 as usize)
        .ok_or_else(|| color_eyre::eyre::eyre!("条目钩子闭包 id 越界"))?;
    let func: Function = handles.lua.registry_value(key).map_err(to_eyre)?;
    let ret: mlua::Value = func.call(()).map_err(to_eyre)?;
    Ok(!matches!(ret, mlua::Value::Boolean(false)))
}

/// 激活命中的 host 块（执行其闭包，收集 vars/link）。
///
/// # Return:
///   `true` 表示命中并执行；`false` 表示当前 hostname 无对应块。
pub fn activate_host(
    hostname: &str,
    manifest: &Manifest,
    handles: &LuaHandles,
    effect: &Rc<RefCell<EffectState>>,
) -> Result<bool> {
    let Some(id) = manifest.host_blocks.get(hostname) else {
        return Ok(false);
    };
    primitives::install(&handles.lua, effect).map_err(to_eyre)?;
    let key = handles
        .closures
        .get(id.0 as usize)
        .ok_or_else(|| color_eyre::eyre::eyre!("host 块闭包 id 越界"))?;
    let func: Function = handles.lua.registry_value(key).map_err(to_eyre)?;
    func.call::<()>(()).map_err(to_eyre)?;
    Ok(true)
}
