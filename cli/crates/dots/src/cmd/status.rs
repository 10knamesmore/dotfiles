//! `dots status`：只读展示与 sync 完全相同的 Resource Plan。

use super::Result;
use crate::exec::execute;
use crate::realfs::RealFs;
use crate::reconciliation::prepare;
use crate::render;

/// 运行 status，并返回当前机器是否已经完全收敛。
pub fn run() -> Result<bool> {
    let mut prepared = prepare()?;
    render::header("dots status");
    for conflict in &prepared.script_conflicts {
        render::err(&format!("scripts ownership 冲突：{conflict}"));
    }
    let filesystem = RealFs::new();
    let report = execute(
        &prepared.plan,
        &prepared.container_conversions,
        &filesystem,
        &mut prepared.state,
        true,
    );
    for failure in &report.failures {
        render::err(failure);
    }
    let converged = prepared.plan.is_clean()
        && prepared.container_conversions.is_empty()
        && prepared.script_conflicts.is_empty()
        && report.is_healthy();
    render::ok(if converged {
        "Resource 全部已收敛"
    } else {
        "存在待执行变更、Collision、Drift 或 planning failure"
    });
    Ok(converged)
}
