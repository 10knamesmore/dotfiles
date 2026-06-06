//! Executor —— 把 [`Plan`] 落盘。
//!
//! dry-run 在此统一守卫（不像 install.py 散落各处）。所有破坏性动作前备份。

use dots_core::{Plan, PlanAction};
use owo_colors::OwoColorize;

use crate::realfs::RealFs;
use crate::state::State;

/// 通用 Result 别名。
pub type Result<T> = color_eyre::Result<T>;

/// 执行报告（计数）。
#[derive(Debug, Default)]
pub struct ExecReport {
    /// 新建链接数。
    pub linked: usize,
    /// 重建链接数。
    pub relinked: usize,
    /// 备份数。
    pub backed_up: usize,
    /// 容器转换数。
    pub converted: usize,
    /// 报告的外部漂移数（未自动处理）。
    pub drift: usize,
}

/// 执行 Plan。`dry_run` 时只打印不落盘。
///
/// # Params:
///   - `plan`: 待执行计划
///   - `fs`: 真实文件系统
///   - `state`: 台账（记录新建链接）
///   - `dry_run`: 是否仅预览
///
/// # Return:
///   执行报告（计数）。
pub fn execute(plan: &Plan, fs: &RealFs, state: &mut State, dry_run: bool) -> Result<ExecReport> {
    let mut report = ExecReport::default();
    for item in &plan.items {
        let target = item.target.as_path();
        match &item.action {
            PlanAction::Noop => {}
            PlanAction::Link { source } => {
                println!("  {} {}", "+".green(), target.display());
                if !dry_run {
                    fs.make_symlink(source.as_path(), target)?;
                    state.record_link(target.to_owned(), source.as_path().to_owned());
                }
                report.linked += 1;
            }
            PlanAction::Relink { source, old_target } => {
                println!(
                    "  {} {} {}",
                    "~".yellow(),
                    target.display(),
                    format!("(was {})", old_target.display()).dimmed()
                );
                if !dry_run {
                    fs.remove_symlink(target)?;
                    fs.make_symlink(source.as_path(), target)?;
                    state.record_link(target.to_owned(), source.as_path().to_owned());
                }
                report.relinked += 1;
            }
            PlanAction::BackupThenLink { source } => {
                println!(
                    "  {} {} {}",
                    "+".green(),
                    target.display(),
                    "(backup existing)".dimmed()
                );
                if !dry_run {
                    fs.backup(target)?;
                    fs.make_symlink(source.as_path(), target)?;
                    state.record_link(target.to_owned(), source.as_path().to_owned());
                }
                report.backed_up += 1;
                report.linked += 1;
            }
            PlanAction::ContainerConvert { source_dir } => {
                println!(
                    "  {} {} {}",
                    "⇄".cyan(),
                    target.display(),
                    format!("(was dir-link → {})", source_dir.as_path().display()).dimmed()
                );
                if !dry_run {
                    fs.remove_symlink(target)?;
                    fs.make_dir_all(target)?;
                }
                report.converted += 1;
            }
            PlanAction::DriftForeign { points_to } => {
                println!(
                    "  {} {} {}",
                    "✗".red(),
                    target.display(),
                    format!("→ {} (外部链接，跳过)", points_to.display()).dimmed()
                );
                report.drift += 1;
            }
        }
    }
    Ok(report)
}
