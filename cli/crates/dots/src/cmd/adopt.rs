//! `dots adopt` —— 收编：搬入 tree + 原地反链 + 记台账（§5）。
//!
//! CLI 永不写 dots.lua；需要清单配合时打印建议行。

use std::fs;
use std::path::{Path, PathBuf};

use color_eyre::eyre::{Context, eyre};
use dots_core::LinkMode;

use super::{Result, find_repo_root, home_dir};
use crate::realfs::RealFs;
use crate::render;
use crate::state::{OpKind, OpLog, State};

/// adopt 选项。
pub struct AdoptOpts {
    /// 指定落层（覆盖推断），如 `home` / `home.linux`。
    pub layer: Option<String>,
    /// 指定粒度（影响后续建议行）。
    pub mode: Option<LinkMode>,
}

/// 收编一批路径。
pub fn run(paths: &[PathBuf], opts: &AdoptOpts) -> Result<()> {
    let repo_root = find_repo_root()?;
    let home = home_dir()?;
    let fs = RealFs::new(repo_root.clone(), crate::timestamp());
    let mut state = State::load(&repo_root)?;

    for path in paths {
        adopt_one(path, &repo_root, &home, &fs, &mut state, opts)?;
    }
    state.save(&repo_root)?;
    Ok(())
}

/// 收编单条路径：搬入 tree、原地反链、记台账并打印建议。
fn adopt_one(
    path: &Path,
    repo_root: &Path,
    home: &Path,
    fs: &RealFs,
    state: &mut State,
    opts: &AdoptOpts,
) -> Result<()> {
    let abs = path
        .canonicalize()
        .wrap_err_with(|| format!("无法解析路径：{}", path.display()))?;
    if !abs.starts_with(home) {
        return Err(eyre!("暂只支持收编 $HOME 下的路径：{}", abs.display()));
    }
    let rel = abs.strip_prefix(home).unwrap_or(&abs);
    // 默认落通用层 home；平台专属由 --layer home.linux 指定。
    let layer = opts.layer.clone().unwrap_or_else(|| "home".to_owned());
    let dest = repo_root.join("tree").join(&layer).join(rel);

    if dest.exists() {
        return Err(eyre!("目标已存在，疑似已纳管：{}", dest.display()));
    }
    if let Some(parent) = dest.parent() {
        fs.make_dir_all(parent)?;
    }
    // 1) 搬入仓库
    fs::rename(&abs, &dest).wrap_err_with(|| format!("搬移失败：{}", abs.display()))?;
    // 2) 原地反链
    fs.make_symlink(&dest, &abs)?;
    // 3) 记台账
    state.ops.push(OpLog {
        kind: OpKind::Adopt,
        home_path: abs.clone(),
        repo_path: dest.clone(),
    });
    state.record_link(abs.clone(), dest.clone());

    render::ok(&format!("收编 {} → {}", abs.display(), dest.display()));

    // 智能提醒（不改清单，仅建议）
    warn_if_absolute_repo_path(&dest, repo_root);
    warn_if_runtime_junk(&dest);
    render::suggest("撤销：dots undo");
    Ok(())
}

/// 文件含仓库绝对路径 → 提醒转 .inject。
fn warn_if_absolute_repo_path(dest: &Path, repo_root: &Path) {
    if let Ok(content) = fs::read_to_string(dest) {
        if content.contains(&repo_root.display().to_string()) {
            render::warn(
                "文件含仓库绝对路径，建议改 .inject 后缀走 minijinja 渲染（换机才不失效）",
            );
        }
    }
}

/// 目录含 node_modules/lock → 建议 granularity file + ignore。
fn warn_if_runtime_junk(dest: &Path) {
    if dest.is_dir() {
        for junk in ["node_modules", "package.json", "bun.lock", ".gitignore"] {
            if dest.join(junk).exists() {
                let rel = dest
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("PATH");
                render::suggest(&format!(
                    "granularity(\"home/.config/{rel}\", {{ mode = \"file\", ignore = {{ \"node_modules\" }} }})"
                ));
                break;
            }
        }
    }
}
