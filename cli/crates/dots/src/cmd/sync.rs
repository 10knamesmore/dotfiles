//! `dots sync`：对唯一 Resource Plan 执行真实收敛或 dry-run。

use super::Result;
use crate::exec::execute;
use crate::realfs::RealFs;
use crate::reconciliation::prepare;
use crate::render;

/// 运行 sync，并返回最终是否没有 Collision、Drift、脚本冲突或 apply failure。
pub fn run(dry_run: bool) -> Result<bool> {
    let mut prepared = prepare()?;
    render::header(if dry_run {
        "dots sync · dry-run"
    } else {
        "dots sync"
    });
    if !prepared.script_conflicts.is_empty() {
        for conflict in &prepared.script_conflicts {
            render::err(&format!("scripts ownership 冲突：{conflict}"));
        }
        return Ok(false);
    }

    let filesystem = RealFs::new();
    let report = execute(
        &prepared.plan,
        &prepared.container_conversions,
        &filesystem,
        &mut prepared.state,
        dry_run,
    );
    for failure in &report.failures {
        render::err(failure);
    }
    if !dry_run {
        prepared.state.save(&prepared.repo_root)?;
    }
    render::ok(&format!(
        "{} 变更 · {} 接管 · {} 删除 · {} 遗忘 · {} collision · {} drift · {} 失败",
        report.changed,
        report.adopted,
        report.deleted,
        report.forgotten,
        report.collisions,
        report.drift,
        report.failures.len()
    ));
    Ok(report.is_healthy())
}
