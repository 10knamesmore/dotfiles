//! `dots sync`：按步骤展示 lifecycle hook、资源收敛和状态保存。

use color_eyre::eyre::eyre;
use dots_core::PlanAction;

use super::Result;
use crate::exec::{ExecReport, execute};
use crate::hooks::run_before_sync;
use crate::realfs::RealFs;
use crate::reconciliation::{load_configuration, prepare_configuration};
use crate::render;

/// 执行或预览同步；冲突、保留目录、漂移和执行失败都返回非成功状态。
pub fn run(dry_run: bool) -> Result<bool> {
    let mut steps = render::Steps::new(if dry_run {
        "dots sync · dry-run"
    } else {
        "dots sync"
    });
    match synchronize(dry_run, &mut steps) {
        Ok(report) => {
            let healthy = report.is_healthy();
            let result = match (dry_run, healthy) {
                (true, true) => "预览完成 · 未执行 hook 或资源变更",
                (true, false) => "预览发现阻塞项 · 未执行 hook 或资源变更",
                (false, true) => "完成",
                (false, false) => "未全部完成 · 成功项已保存",
            };
            steps.finish(healthy, &format!("dots sync {result}"));
            render::detail(&format!(
                "{} 变更 · {} 接管 · {} 删除 · {} 遗忘 · {} 保留 · {} collision · {} drift · {} 失败",
                report.changed,
                report.adopted,
                report.deleted,
                report.forgotten,
                report.retained,
                report.collisions,
                report.drift,
                report.failures.len()
            ));
            Ok(healthy)
        }
        Err(error) => {
            steps.finish(false, &format!("dots sync 中止于步骤 {}", steps.position()));
            render::detail(&format!("{error:#}"));
            render::detail("后续步骤未执行。");
            Ok(false)
        }
    }
}

/// hook 在规划前执行；应用有失败时仍保存成功项，保持原有收敛顺序。
fn synchronize(dry_run: bool, steps: &mut render::Steps) -> Result<ExecReport> {
    let configuration = steps.begin("dots", "读取配置").run(|| {
        let configuration = load_configuration()?;
        render::detail(
            &configuration
                .repo_root
                .join("dots.lua")
                .display()
                .to_string(),
        );
        Ok(configuration)
    })?;
    steps.set_total(configuration.manifest.hooks.before_sync.len() + 4);
    run_before_sync(&configuration.manifest.hooks.before_sync, dry_run, steps)?;

    let mut prepared = steps.begin("dots", "生成变更计划").run(|| {
        let prepared = prepare_configuration(configuration)?;
        if !prepared.script_conflicts.is_empty() {
            return Err(eyre!(
                "scripts ownership 冲突：\n{}",
                prepared.script_conflicts.join("\n")
            ));
        }
        let unchanged = prepared
            .plan
            .items
            .iter()
            .filter(|item| matches!(item.action, PlanAction::Noop))
            .count();
        let blocked = prepared
            .plan
            .items
            .iter()
            .filter(|item| {
                matches!(
                    item.action,
                    PlanAction::Collision { .. }
                        | PlanAction::Drift { .. }
                        | PlanAction::Retain { .. }
                )
            })
            .count();
        render::detail(&format!(
            "{} 项待处理 · {unchanged} 项无需变更 · {blocked} 项阻塞",
            prepared.plan.items.len() - unchanged - blocked + prepared.container_conversions.len()
        ));
        Ok(prepared)
    })?;

    let step = steps.begin(
        "dots",
        if dry_run {
            "预览资源变更"
        } else {
            "应用资源变更"
        },
    );
    let filesystem = RealFs::new();
    let report = execute(
        &prepared.plan,
        &prepared.container_conversions,
        &filesystem,
        &mut prepared.state,
        dry_run,
    );
    for failure in &report.failures {
        render::detail(&format!("✗ {failure}"));
    }
    step.finish(
        report.is_healthy(),
        match (dry_run, report.is_healthy()) {
            (true, true) => "预览完成",
            (true, false) => "存在阻塞项",
            (false, true) => "完成",
            (false, false) => "未全部完成",
        },
    );

    let step = steps.begin("dots", "保存同步状态");
    render::detail(
        &prepared
            .repo_root
            .join(".dots/state.json")
            .display()
            .to_string(),
    );
    if dry_run {
        step.skip();
    } else {
        step.run(|| {
            prepared
                .state
                .save(&prepared.repo_root)
                .map_err(|error| eyre!("资源应用已结束，但同步状态保存失败：{error:#}"))
        })?;
    }
    Ok(report)
}
