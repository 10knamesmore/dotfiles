//! `dots sync` —— 幂等收敛总编排（§5/§7/§9/§11）。

use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

use color_eyre::eyre::eyre;
use dots_core::manifest::{DistributeSpec, HookPhase, Manifest};
use dots_core::{
    AbsPath, ExpectedLink, FileSystem, Layer, LinkMode, Os, expand_layers, plan_scripts, resolve,
};

use super::{Result, current_os, expand_home, find_repo_root, home_dir, os_str};
use crate::exec::execute;
use crate::hooks::{EffectState, activate_host, call_entry_hook, run_phase};
use crate::lua::{LuaCtx, eval_manifest};
use crate::realfs::RealFs;
use crate::render;
use crate::shellenv::{ensure_zshrc_stub, write_shell_env};
use crate::state::State;

/// 运行 sync。
pub fn run(dry_run: bool) -> Result<()> {
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
    let (manifest, handles) = eval_manifest(&dots_lua, &ctx)?;

    let state = State::load(&repo_root)?;
    let stamp = crate::timestamp();
    let fs = RealFs::new(repo_root.clone(), stamp.clone());
    let effect = Rc::new(RefCell::new(EffectState::new(
        repo_root.clone(),
        home.clone(),
        stamp,
        dry_run,
        state,
    )));

    render::header(&format!("dots sync · {hostname} ({})", os_str(os)));

    // 1) pre_sync 钩子
    run_phase(HookPhase::PreSync, &manifest, &handles, &effect)?;

    // 2) host 激活（收集 vars / extra_links）。未命中且存在 host 块 → 硬报错。
    let hit = activate_host(&hostname, &manifest, &handles, &effect)?;
    if !hit && !manifest.host_blocks.is_empty() {
        print_host_skeleton(&hostname);
        return Err(eyre!(
            "当前主机 `{hostname}` 未在 dots.lua 的 hosts{{}} 覆盖"
        ));
    }
    if hit {
        run_phase(HookPhase::OnHostActivate, &manifest, &handles, &effect)?;
    }

    // 3) 收集期望链接：层映射 + distribute + scripts + host extra_links
    let repo_abs = AbsPath::new(&repo_root);
    let home_abs = AbsPath::new(&home);
    let mut links = expand_layers(&fs, &repo_abs, &home_abs, os, &manifest);

    // 3.5) 条目级 pre：返回 false 的条目，其链接整体剔除；幸存条目的 post 入队。
    let mut entry_posts: Vec<dots_core::manifest::ClosureId> = Vec::new();
    for (path, spec) in &manifest.granularity {
        if let Some(pre) = spec.pre {
            if !call_entry_hook(pre, &handles, &effect)? {
                let prefix = repo_root.join("tree").join(path.as_path());
                links.retain(|link| !link.source.as_path().starts_with(&prefix));
                render::warn(&format!("⊘ 条目跳过（pre）：{}", path.as_path().display()));
                continue;
            }
        }
        if let Some(post) = spec.post {
            entry_posts.push(post);
        }
    }
    let mut blocked_distribute = rustc_hash::FxHashSet::default();
    for (idx, spec) in manifest.distribute.iter().enumerate() {
        if let Some(pre) = spec.pre {
            if !call_entry_hook(pre, &handles, &effect)? {
                blocked_distribute.insert(idx);
                render::warn(&format!("⊘ 分发跳过（pre）：{}", spec.name));
                continue;
            }
        }
        if let Some(post) = spec.post {
            entry_posts.push(post);
        }
    }

    links.extend(distribute_links(
        &fs,
        &repo_abs,
        &manifest,
        &home,
        &blocked_distribute,
    ));
    let (script_links, conflicts) = plan_scripts(&fs, &repo_abs, os, &manifest.scripts_ignore_tree);
    for conflict in &conflicts {
        render::warn(&format!(
            "脚本重名冲突：{}（{} 个来源）",
            conflict.name,
            conflict.sources.len()
        ));
    }
    links.extend(script_links);
    links.extend(take_extra_links(&effect));

    // 4) 判定 + 执行
    let plan = resolve(&fs, &repo_abs, &links);
    {
        let mut st = effect.borrow_mut();
        let report = execute(&plan, &fs, &mut st.state, dry_run)?;
        render::ok(&format!(
            "{} 链接 · {} 重建 · {} 备份 · {} 转换 · {} 漂移",
            report.linked, report.relinked, report.backed_up, report.converted, report.drift
        ));
    }

    // 4.5) 条目级 post（仅未被 pre 阻止的条目；dry-run 不执行——效果未发生）
    if !dry_run {
        for id in &entry_posts {
            call_entry_hook(*id, &handles, &effect)?;
        }
    }

    // 5) .inject 渲染（生成型，用 host_vars + secrets）
    render_injects(&repo_root, &home, &fs, &effect, os, dry_run)?;

    // 6) post_link 钩子（链接与 .inject 已就绪）
    run_phase(HookPhase::PostLink, &manifest, &handles, &effect)?;

    // 7) shell 环境（stub + env.zsh + root）
    if !dry_run {
        ensure_zshrc_stub(&home, &fs)?;
        write_shell_env(&home, &repo_root, &fs)?;
    }

    // 8) systemd user enable
    enable_systemd(&manifest, dry_run);

    // 9) post_sync 钩子——一切就绪（链接/inject/env.zsh/systemd）之后、台账保存之前。
    run_phase(HookPhase::PostSync, &manifest, &handles, &effect)?;

    // 10) 保存台账。effect 的 Rc 仍被 lua 闭包持有（handles 在作用域内），
    //    故 clone 出 state 保存，而非 try_unwrap 独占。
    //    保存前修剪孤儿记录（target 磁盘已无——如用户手动删了落点目录）。
    if !dry_run {
        let mut final_state = effect.borrow().state.clone();
        let pruned = final_state.prune_missing();
        if pruned > 0 {
            println!("  ○ 台账修剪：{pruned} 条孤儿记录（磁盘已无）");
        }
        final_state.save(&repo_root)?;
    }
    Ok(())
}

/// 取走 host 块收集的 extra_links（转成 ExpectedLink）。
fn take_extra_links(effect: &Rc<RefCell<EffectState>>) -> Vec<ExpectedLink> {
    let mut st = effect.borrow_mut();
    let via = Layer {
        name: "host".to_owned(),
        os: None,
    };
    std::mem::take(&mut st.extra_links)
        .into_iter()
        .map(|(source, target)| ExpectedLink {
            target: AbsPath::new(target),
            source: AbsPath::new(source),
            via: via.clone(),
            shadowed: Vec::new(),
        })
        .collect()
}

/// distribute：一源多落点。父工具目录不存在则 warn 跳过（§12 容错）。
///
/// `blocked` 是被条目级 pre 阻止的 spec 下标集合（status/doctor 等只读路径传空集）。
pub(crate) fn distribute_links(
    fs: &dyn FileSystem,
    repo_abs: &AbsPath,
    manifest: &Manifest,
    home: &Path,
    blocked: &rustc_hash::FxHashSet<usize>,
) -> Vec<ExpectedLink> {
    let via = Layer {
        name: "distribute".to_owned(),
        os: None,
    };
    let mut out = Vec::new();
    for (idx, spec) in manifest.distribute.iter().enumerate() {
        if blocked.contains(&idx) {
            continue;
        }
        let src_abs = repo_abs.join(spec.src.as_path());
        for to in &spec.to {
            let to_abs = AbsPath::new(expand_home(to, home));
            // 工具根目录（落点的父目录）不存在 → 跳过（工具未装）。
            if let Some(parent) = to_abs.as_path().parent() {
                if !parent.exists() {
                    render::warn(&format!("分发跳过（{} 不存在）：{}", parent.display(), to));
                    continue;
                }
            }
            push_distribute(fs, spec, &src_abs, &to_abs, &via, &mut out);
        }
    }
    out
}

/// 按 distribute 粒度（children/dir/file）把单条 spec 展开为期望链接。
fn push_distribute(
    fs: &dyn FileSystem,
    spec: &DistributeSpec,
    src_abs: &AbsPath,
    to_abs: &AbsPath,
    via: &Layer,
    out: &mut Vec<ExpectedLink>,
) {
    match spec.mode {
        LinkMode::Children => {
            for child in fs.read_dir(src_abs.as_path()) {
                if let Some(name) = child.file_name() {
                    out.push(ExpectedLink {
                        target: AbsPath::new(to_abs.as_path().join(name)),
                        source: AbsPath::new(child),
                        via: via.clone(),
                        shadowed: Vec::new(),
                    });
                }
            }
        }
        LinkMode::Dir | LinkMode::File => out.push(ExpectedLink {
            target: to_abs.clone(),
            source: src_abs.clone(),
            via: via.clone(),
            shadowed: Vec::new(),
        }),
    }
}

/// 扫 tree 渲染 `*.inject`（生成型，§6-B）。
fn render_injects(
    repo_root: &Path,
    home: &Path,
    fs: &RealFs,
    effect: &Rc<RefCell<EffectState>>,
    os: Os,
    dry_run: bool,
) -> Result<()> {
    use crate::inject::{InjectCtx, render as render_tpl};
    let st = effect.borrow();
    let secret = crate::secret::load_all(repo_root, home, false).unwrap_or_default();
    let ctx = InjectCtx {
        dotfiles: repo_root.display().to_string(),
        scripts: repo_root.join(".gen/scripts").display().to_string(),
        host: st.host_vars.clone(),
        secret,
    };
    drop(st);

    let layers = ["home", &format!("home.{}", os_str(os))];
    for layer in layers {
        let layer_dir = repo_root.join("tree").join(layer);
        for entry in walk_files(&layer_dir) {
            let Some(name) = entry.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if !name.ends_with(".inject") {
                continue;
            }
            let src = std::fs::read_to_string(&entry).unwrap_or_default();
            let rendered = render_tpl(&src, &ctx)?;
            // 目标：去 .inject 后缀，映射回 $HOME 侧。
            let rel = entry.strip_prefix(&layer_dir).unwrap_or(&entry);
            let rel_stripped = rel.to_string_lossy().trim_end_matches(".inject").to_owned();
            let out_gen = repo_root.join(".gen/injected").join(&rel_stripped);
            let target = home.join(&rel_stripped);
            if dry_run {
                render::suggest(&format!("inject → {}", target.display()));
                continue;
            }
            fs.write_atomic(&out_gen, rendered.as_bytes())?;
            // 链接 $HOME 目标到生成产物
            if !matches!(fs.classify(&target), dots_core::NodeKind::Symlink { .. }) {
                let _ = fs.make_symlink(&out_gen, &target);
            }
        }
    }
    Ok(())
}

/// 递归列出目录下所有文件（含子目录）。
fn walk_files(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    for entry in walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(|entry| entry.ok())
    {
        if entry.file_type().is_file() {
            out.push(entry.into_path());
        }
    }
    out
}

/// systemctl --user enable（幂等）；dry-run 仅打印；失败 warn 不致命。
fn enable_systemd(manifest: &Manifest, dry_run: bool) {
    for unit in &manifest.systemd_user {
        if dry_run {
            render::suggest(&format!("systemctl --user enable {unit}"));
            continue;
        }
        let status = std::process::Command::new("systemctl")
            .args(["--user", "enable", unit])
            .status();
        match status {
            Ok(code) if code.success() => {}
            _ => render::warn(&format!("systemctl --user enable {unit} 失败（忽略）")),
        }
    }
}

/// 打印 hosts{} 骨架（未命中当前机时）。
fn print_host_skeleton(hostname: &str) {
    render::warn(&format!(
        "当前主机 `{hostname}` 未覆盖，建议在 dots.lua 加："
    ));
    render::suggest(&format!("hosts {{ {hostname} = function()"));
    render::suggest("  vars { backlight = \"\", ddc_index = \"\" }");
    render::suggest(
        "  -- link(\"hosts/files/THIS/monitors.conf\", \"~/.config/hypr/monitors.conf\")",
    );
    render::suggest("end }");
}
