//! `dots doctor` —— 深检（§5）。只读：不改任何文件。

use dots_core::{AbsPath, PlanAction, expand_layers, plan_scripts, resolve};

use super::{Result, current_os, find_repo_root, home_dir, os_str};
use crate::lua::{LuaCtx, eval_manifest};
use crate::realfs::RealFs;
use crate::render;
use crate::state::State;

/// 运行 doctor。返回是否无 error 级问题。
pub fn run() -> Result<bool> {
    let repo_root = find_repo_root()?;
    let home = home_dir()?;
    let os = current_os();
    let hostname = crate::hosts::current();
    let mut healthy = true;

    render::header("dots doctor");

    let dots_lua = std::fs::read_to_string(repo_root.join("dots.lua")).unwrap_or_default();
    let ctx = LuaCtx {
        host: hostname.clone(),
        os: os_str(os).to_owned(),
        home: home.display().to_string(),
    };
    let (manifest, _h) = eval_manifest(&dots_lua, &ctx)?;
    let fs = RealFs::new(repo_root.clone(), "doctor".to_owned());

    // 1) hostname 是否被 hosts{} 覆盖
    if !manifest.host_blocks.is_empty() && !manifest.host_blocks.contains_key(&hostname) {
        healthy = false;
        render::err(&format!(
            "当前主机 `{hostname}` 未在 hosts{{}} 覆盖（per-host 配置不会生效）"
        ));
    }

    // 2) scripts 重名冲突
    let repo_abs = AbsPath::new(&repo_root);
    let home_abs = AbsPath::new(&home);
    let (_sl, conflicts) = plan_scripts(&fs, &repo_abs, os, &manifest.scripts_ignore_tree);
    for conflict in &conflicts {
        healthy = false;
        render::err(&format!(
            "脚本重名：{}（{} 个来源）",
            conflict.name,
            conflict.sources.len()
        ));
    }

    // 3) 链接漂移（外部链接）
    let mut links = expand_layers(&fs, &repo_abs, &home_abs, os, &manifest);
    // 只读巡检不执行 pre 闭包：传空 blocked。
    links.extend(super::sync::distribute_links(
        &fs,
        &repo_abs,
        &manifest,
        &home,
        &rustc_hash::FxHashSet::default(),
    ));
    let plan = resolve(&fs, &repo_abs, &links);
    for item in &plan.items {
        if let PlanAction::DriftForeign { points_to } = &item.action {
            healthy = false;
            render::err(&format!(
                "{} 指向仓库外 {}（dots 不擅动）",
                item.target.as_path().display(),
                points_to.display()
            ));
        }
    }

    // 4) 钩子认领的 keypath 漂移（owned 文件里该 key 是否还在）
    let state = State::load(&repo_root)?;
    for own in &state.owned {
        if let Ok(content) = std::fs::read_to_string(&own.file) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                let top = own.keypath.split('.').next().unwrap_or(&own.keypath);
                if json.get(top).is_none() {
                    healthy = false;
                    render::err(&format!(
                        "{} 的认领键 `{}` 已消失（工具改写覆盖了？重跑 dots sync）",
                        own.file.display(),
                        own.keypath
                    ));
                }
            }
        }
    }

    // 5) 只读提示：Claude permissions 落点
    render::warn(
        "提示：Claude `permissions` 运行时实际落点是 ~/.claude.json 的 projects[].allowedTools，\
         入库 settings.json 的 permissions 只读、不跨机回填",
    );

    if healthy {
        render::ok("无 error 级问题");
    }
    Ok(healthy)
}
