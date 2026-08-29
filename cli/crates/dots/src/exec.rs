//! 依固定 phase 执行 Resource Plan，并按单项成功更新 Applied Inventory。

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use dots_core::{
    AppliedResource, FileSystem, NodeKind, OwnershipSurface, Plan, PlanAction, PlanItem,
    ResourceSpec, ResourceState,
};
use owo_colors::OwoColorize;

use crate::managed_block;
use crate::realfs::RealFs;
use crate::reconciliation::{ContainerConversion, observe_surface};
use crate::state::State;

/// Executor 对 planning 后重观察与 systemd 写入的可替换边界。
trait Runtime {
    /// 重新读取一个 Ownership Surface。
    fn observe(&self, surface: &OwnershipSurface) -> dots_core::ObservedState;

    /// 设置 systemd user unit enabled 状态。
    fn set_systemd_enabled(&self, unit: &str, enabled: bool) -> Result<()>;
}

/// 真实机器 runtime。
struct RealRuntime;

impl Runtime for RealRuntime {
    fn observe(&self, surface: &OwnershipSurface) -> dots_core::ObservedState {
        observe_surface(surface)
    }

    fn set_systemd_enabled(&self, unit: &str, enabled: bool) -> Result<()> {
        let action = if enabled { "enable" } else { "disable" };
        let status = Command::new("systemctl")
            .args(["--user", action, unit])
            .status()?;
        if !status.success() {
            return Err(color_eyre::eyre::eyre!(
                "systemctl --user {action} {unit} 失败：{status}"
            ));
        }
        Ok(())
    }
}

/// 通用 Result 别名。
pub type Result<T> = color_eyre::Result<T>;

/// 一次执行的动作与失败汇总。
#[derive(Debug, Default)]
pub struct ExecReport {
    /// 外部状态或 inventory 真正发生变化的项目数。
    pub changed: usize,

    /// 无需外部写入而接管的项目数。
    pub adopted: usize,

    /// 安全删除的 Retired Resource 数。
    pub deleted: usize,

    /// 从 inventory 遗忘的已不存在 Retired Resource 数。
    pub forgotten: usize,

    /// 未执行的 Collision 数。
    pub collisions: usize,

    /// 未执行的 Drift 数。
    pub drift: usize,

    /// apply 失败，且保留旧 inventory 供重试的项目。
    pub failures: Vec<String>,
}

impl ExecReport {
    /// 返回本次执行是否没有 Collision、Drift 或 apply failure。
    pub fn is_healthy(&self) -> bool {
        self.collisions == 0 && self.drift == 0 && self.failures.is_empty()
    }
}

/// 执行 container conversion 与 Resource Plan；dry-run 只输出，不修改 inventory。
pub fn execute(
    plan: &Plan,
    conversions: &[ContainerConversion],
    filesystem: &RealFs,
    state: &mut State,
    dry_run: bool,
) -> ExecReport {
    execute_with_runtime(plan, conversions, filesystem, state, dry_run, &RealRuntime)
}

/// 使用指定 runtime 执行 Plan，供真实入口与 systemd failure 测试共享。
fn execute_with_runtime(
    plan: &Plan,
    conversions: &[ContainerConversion],
    filesystem: &RealFs,
    state: &mut State,
    dry_run: bool,
    runtime: &dyn Runtime,
) -> ExecReport {
    let mut report = ExecReport::default();
    let blocked_prefixes = execute_conversions(conversions, filesystem, dry_run, &mut report);
    for item in &plan.items {
        if item.surface.filesystem_path().is_some_and(|path| {
            blocked_prefixes
                .iter()
                .any(|prefix| path.starts_with(prefix))
        }) {
            report.failures.push(format!(
                "{}：父容器转换失败，未执行",
                item.surface.selector()
            ));
            continue;
        }
        execute_item(item, filesystem, state, dry_run, runtime, &mut report);
    }
    report
}

/// 执行旧整目录 symlink 到真实目录的转换，并返回失败 prefix。
fn execute_conversions(
    conversions: &[ContainerConversion],
    filesystem: &RealFs,
    dry_run: bool,
    report: &mut ExecReport,
) -> Vec<PathBuf> {
    let mut blocked = Vec::new();
    for conversion in conversions {
        println!(
            "  {} container:{} {}",
            "update".cyan(),
            conversion.target.display(),
            format!("was symlink → {}", conversion.source.display()).dimmed()
        );
        if dry_run {
            report.changed += 1;
            continue;
        }
        let result = match filesystem.classify(&conversion.target) {
            NodeKind::Symlink { target } if target == conversion.source => filesystem
                .remove_symlink(&conversion.target)
                .and_then(|()| filesystem.make_dir_all(&conversion.target)),
            actual => Err(color_eyre::eyre::eyre!(
                "container 在 apply 前变化：{actual:?}"
            )),
        };
        match result {
            Ok(()) => report.changed += 1,
            Err(error) => {
                blocked.push(conversion.target.clone());
                report.failures.push(format!(
                    "container:{}：{error}",
                    conversion.target.display()
                ));
            }
        }
    }
    blocked
}

/// 执行单个 PlanItem；失败只记录，不中止无依赖 Resource。
fn execute_item(
    item: &PlanItem,
    filesystem: &RealFs,
    state: &mut State,
    dry_run: bool,
    runtime: &dyn Runtime,
    report: &mut ExecReport,
) {
    let selector = item.surface.selector();
    match &item.action {
        PlanAction::Noop => {}
        PlanAction::Adopt => {
            println!("  {} {selector}", "adopt".cyan());
            if !dry_run {
                update_inventory_from_desired(item, state);
            }
            report.adopted += 1;
        }
        PlanAction::Create => {
            println!("  {} {selector}", "create".green());
            execute_desired(item, filesystem, dry_run, state, runtime, report);
        }
        PlanAction::Update => {
            println!("  {} {selector}", "update".yellow());
            execute_desired(item, filesystem, dry_run, state, runtime, report);
        }
        PlanAction::Delete => {
            println!("  {} {selector}", "delete".red());
            if dry_run {
                report.deleted += 1;
            } else {
                match delete_applied(item, filesystem, runtime) {
                    Ok(()) => {
                        state.remove_resource(&item.surface);
                        report.deleted += 1;
                    }
                    Err(error) => report.failures.push(format!("{selector}：{error}")),
                }
            }
        }
        PlanAction::Forget => {
            println!("  {} {selector}", "forget".cyan());
            if !dry_run {
                state.remove_resource(&item.surface);
            }
            report.forgotten += 1;
        }
        PlanAction::Collision { reason } => {
            println!("  {} {selector} — {reason}", "collision".red());
            report.collisions += 1;
        }
        PlanAction::Drift { reason } => {
            println!("  {} {selector} — {reason}", "drift".red());
            report.drift += 1;
        }
    }
}

/// apply Desired Resource，并在成功后替换对应 inventory record。
fn execute_desired(
    item: &PlanItem,
    filesystem: &RealFs,
    dry_run: bool,
    state: &mut State,
    runtime: &dyn Runtime,
    report: &mut ExecReport,
) {
    if dry_run {
        report.changed += 1;
        return;
    }
    let Some(desired) = item.desired.as_ref() else {
        report.failures.push(format!(
            "{}：Plan 缺 Desired Resource",
            item.surface.selector()
        ));
        return;
    };
    if let Err(error) = ensure_observed_unchanged(item, runtime) {
        report
            .failures
            .push(format!("{}：{error}", item.surface.selector()));
        return;
    }
    match apply_resource(desired, filesystem, runtime) {
        Ok(()) => {
            update_inventory_from_desired(item, state);
            report.changed += 1;
        }
        Err(error) => report
            .failures
            .push(format!("{}：{error}", item.surface.selector())),
    }
}

/// 把 Desired snapshot 提交到 Applied Inventory。
fn update_inventory_from_desired(item: &PlanItem, state: &mut State) {
    if let Some(desired) = item.desired.as_ref() {
        state.upsert_resource(AppliedResource {
            surface: item.surface.clone(),
            state: desired.desired_state(),
        });
    }
}

/// 创建或更新一项 Desired Resource。
fn apply_resource(
    resource: &ResourceSpec,
    filesystem: &RealFs,
    runtime: &dyn Runtime,
) -> Result<()> {
    match resource {
        ResourceSpec::Symlink { target, source, .. } => {
            prepare_path_for_replace(target.as_path(), filesystem)?;
            filesystem.make_symlink(source.as_path(), target.as_path())
        }
        ResourceSpec::File {
            target,
            content,
            mode,
            ..
        } => {
            if let Ok(metadata) = fs::symlink_metadata(target.as_path()) {
                if metadata.file_type().is_symlink() {
                    filesystem.remove_file(target.as_path())?;
                } else if metadata.file_type().is_dir() {
                    return Err(color_eyre::eyre::eyre!(
                        "普通文件 Resource 不能覆盖真实目录：{}",
                        target.as_path().display()
                    ));
                }
            }
            filesystem
                .write_atomic_with_mode(target.as_path(), content, *mode)
                .map(|_| ())
        }
        ResourceSpec::ManagedBlock {
            target,
            marker,
            content,
            placement,
            ..
        } => {
            let existing = fs::read_to_string(target.as_path()).unwrap_or_default();
            let existing = if marker == "dots-env" {
                migrate_legacy_zshrc(&existing)
            } else {
                existing
            };
            let rebuilt = managed_block::upsert(&existing, marker, content, *placement)
                .map_err(color_eyre::eyre::Report::msg)?;
            filesystem
                .write_atomic_with_mode(target.as_path(), rebuilt.as_bytes(), 0o644)
                .map(|_| ())
        }
        ResourceSpec::SystemdUserUnit { unit, .. } => runtime.set_systemd_enabled(unit, true),
    }
}

/// 把旧单行 stub marker/source 去掉，再由统一 `dots-env` block 接管。
fn migrate_legacy_zshrc(existing: &str) -> String {
    existing
        .lines()
        .filter(|line| {
            !line.starts_with("# DOTS_MANAGED:")
                && !line.starts_with("# DOTFILES_MANAGED:")
                && line.trim() != "source \"$HOME/.zshrc_dotfiles\""
        })
        .collect::<Vec<_>>()
        .join("\n")
        .trim_start_matches('\n')
        .to_owned()
}

/// 在 symlink apply 前删除 planner 已确认仍由 dots 拥有的旧节点。
fn prepare_path_for_replace(path: &Path, filesystem: &RealFs) -> Result<()> {
    if fs::symlink_metadata(path).is_err() {
        return Ok(());
    }
    filesystem.remove_file(path)
}

/// 删除仍与 Applied snapshot 一致的 Retired Resource。
fn delete_applied(item: &PlanItem, filesystem: &RealFs, runtime: &dyn Runtime) -> Result<()> {
    ensure_observed_unchanged(item, runtime)?;
    let applied = item
        .applied
        .as_ref()
        .ok_or_else(|| color_eyre::eyre::eyre!("Delete action 缺 Applied Resource"))?;
    match (&applied.surface, &applied.state) {
        (OwnershipSurface::Path { path }, ResourceState::Symlink { .. })
        | (OwnershipSurface::Path { path }, ResourceState::File { .. }) => {
            filesystem.remove_file(path)
        }
        (OwnershipSurface::ManagedBlock { file, marker }, ResourceState::ManagedBlock { .. }) => {
            let existing = fs::read_to_string(file)?;
            let rebuilt =
                managed_block::remove(&existing, marker).map_err(color_eyre::eyre::Report::msg)?;
            filesystem
                .write_atomic_with_mode(file, rebuilt.as_bytes(), 0o644)
                .map(|_| ())
        }
        (OwnershipSurface::SystemdUserUnit { unit }, ResourceState::SystemdUserUnit { .. }) => {
            runtime.set_systemd_enabled(unit, false)
        }
        _ => Err(color_eyre::eyre::eyre!(
            "Applied Resource surface/state 类型不一致"
        )),
    }
}

/// 在每个破坏性动作前重新读取 surface，避免 planning 后的外部改动被覆盖或删除。
fn ensure_observed_unchanged(item: &PlanItem, runtime: &dyn Runtime) -> Result<()> {
    let current = runtime.observe(&item.surface);
    if current != item.observed {
        return Err(color_eyre::eyre::eyre!(
            "Observed State 在 planning 后变化；原为 {:?}，现为 {:?}，请重试",
            item.observed,
            current
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::min_ident_chars, clippy::missing_docs_in_private_items)]

    use std::cell::RefCell;

    use dots_core::ObservedState;

    use super::*;

    /// 记录 enable/disable 选择并可注入一个方向的失败。
    struct FakeRuntime {
        /// 每次重观察返回的固定状态。
        observed: ObservedState,

        /// 收到的 `(unit, enabled)` 调用。
        calls: RefCell<Vec<(String, bool)>>,

        /// 与此值相同的写入方向返回失败。
        fail_enabled: Option<bool>,
    }

    impl Runtime for FakeRuntime {
        fn observe(&self, _surface: &OwnershipSurface) -> ObservedState {
            self.observed.clone()
        }

        fn set_systemd_enabled(&self, unit: &str, enabled: bool) -> Result<()> {
            self.calls.borrow_mut().push((unit.to_owned(), enabled));
            if self.fail_enabled == Some(enabled) {
                return Err(color_eyre::eyre::eyre!("injected systemd failure"));
            }
            Ok(())
        }
    }

    #[test]
    fn systemd_enable_commits_and_failed_disable_keeps_inventory() -> Result<()> {
        let filesystem = RealFs::new();
        let desired = ResourceSpec::SystemdUserUnit {
            unit: "demo.service".to_owned(),
            origin: "test".to_owned(),
        };
        let surface = desired.surface();
        let disabled = ObservedState::Present(ResourceState::SystemdUserUnit { enabled: false });
        let enable_plan = Plan {
            items: vec![PlanItem {
                surface: surface.clone(),
                desired: Some(desired.clone()),
                applied: None,
                observed: disabled.clone(),
                action: PlanAction::Update,
            }],
        };
        let enable_runtime = FakeRuntime {
            observed: disabled,
            calls: RefCell::new(Vec::new()),
            fail_enabled: None,
        };
        let mut state = State::default();
        let enable_report = execute_with_runtime(
            &enable_plan,
            &[],
            &filesystem,
            &mut state,
            false,
            &enable_runtime,
        );
        assert!(enable_report.is_healthy());
        assert_eq!(
            *enable_runtime.calls.borrow(),
            vec![("demo.service".to_owned(), true)]
        );
        let applied = state
            .resources
            .first()
            .cloned()
            .ok_or_else(|| color_eyre::eyre::eyre!("enable 未提交 inventory"))?;

        let enabled = ObservedState::Present(ResourceState::SystemdUserUnit { enabled: true });
        let disable_plan = Plan {
            items: vec![PlanItem {
                surface: surface.clone(),
                desired: None,
                applied: Some(applied),
                observed: enabled.clone(),
                action: PlanAction::Delete,
            }],
        };
        let disable_runtime = FakeRuntime {
            observed: enabled,
            calls: RefCell::new(Vec::new()),
            fail_enabled: Some(false),
        };
        let disable_report = execute_with_runtime(
            &disable_plan,
            &[],
            &filesystem,
            &mut state,
            false,
            &disable_runtime,
        );
        assert_eq!(disable_report.failures.len(), 1);
        assert_eq!(
            *disable_runtime.calls.borrow(),
            vec![("demo.service".to_owned(), false)]
        );
        assert_eq!(state.resources.len(), 1);
        Ok(())
    }
}
