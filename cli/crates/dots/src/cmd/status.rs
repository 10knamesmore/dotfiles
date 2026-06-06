//! `dots status` —— 三态巡检（§5）。
//!
//! ✔ 已链 / ~ 漂移（重建/外部）/ ✘ 缺失 + 孤儿链接。有问题则返回 `false`（非零退出）。

use dots_core::{AbsPath, PlanAction, expand_layers, plan_scripts, resolve};
use owo_colors::OwoColorize;

use super::{Result, current_os, find_repo_root, home_dir, os_str};
use crate::lua::{LuaCtx, eval_manifest};
use crate::realfs::RealFs;
use crate::render;
use crate::state::State;

/// 运行 status。返回是否全绿。
pub fn run() -> Result<bool> {
    let repo_root = find_repo_root()?;
    let home = home_dir()?;
    let os = current_os();
    let hostname = crate::hosts::current();

    let dots_lua = std::fs::read_to_string(repo_root.join("dots.lua")).unwrap_or_default();
    let ctx = LuaCtx {
        host: hostname.clone(),
        os: os_str(os).to_owned(),
        home: home.display().to_string(),
        repo: repo_root.display().to_string(),
    };
    let (manifest, _handles) = eval_manifest(&dots_lua, &ctx)?;
    let fs = RealFs::new(repo_root.clone(), "status".to_owned());

    let repo_abs = AbsPath::new(&repo_root);
    let home_abs = AbsPath::new(&home);
    let mut links = expand_layers(&fs, &repo_abs, &home_abs, os, &manifest);
    // 只读巡检不执行 pre 闭包：传空 blocked（被 pre 阻止的条目会显示为缺失，已知限制）。
    links.extend(super::sync::distribute_links(
        &fs,
        &repo_abs,
        &manifest,
        &home,
        &rustc_hash::FxHashSet::default(),
    ));
    let (script_links, conflicts) = plan_scripts(&fs, &repo_abs, os, &manifest.scripts_ignore_tree);
    links.extend(script_links);

    let plan = resolve(&fs, &repo_abs, &links);
    render::header(&format!("dots status · {hostname} ({})", os_str(os)));

    let (mut linked, mut drift, mut missing) = (0usize, 0usize, 0usize);
    for item in &plan.items {
        let target = item.target.as_path().display();
        match &item.action {
            PlanAction::Noop => linked += 1,
            PlanAction::Link { .. } => {
                missing += 1;
                println!("  {} {target}", "✘".red());
            }
            PlanAction::Relink { .. }
            | PlanAction::BackupThenLink { .. }
            | PlanAction::ContainerConvert { .. } => {
                drift += 1;
                println!("  {} {target}", "~".yellow());
            }
            PlanAction::DriftForeign { points_to } => {
                drift += 1;
                println!(
                    "  {} {target} → {} (外部)",
                    "~".yellow(),
                    points_to.display()
                );
            }
        }
    }

    // 孤儿：state 记录过但磁盘已无的链接。
    let state = State::load(&repo_root)?;
    let mut orphans = 0usize;
    for rec in &state.links {
        if !rec.target.exists() && std::fs::symlink_metadata(&rec.target).is_err() {
            orphans += 1;
            println!(
                "  {} {} (孤儿：台账有、磁盘无)",
                "○".dimmed(),
                rec.target.display()
            );
        }
    }
    for conflict in &conflicts {
        println!("  {} 脚本重名：{}", "✘".red(), conflict.name);
    }

    render::ok(&format!(
        "{linked} 已链 · {drift} 漂移 · {missing} 缺失 · {orphans} 孤儿"
    ));
    Ok(drift == 0 && missing == 0 && conflicts.is_empty())
}
