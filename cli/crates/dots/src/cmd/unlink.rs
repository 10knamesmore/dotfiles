//! `dots unlink` —— 解除纳管：删链 + 文件搬回 $HOME（§5）。

use std::fs;
use std::path::PathBuf;

use color_eyre::eyre::eyre;

use super::{Result, find_repo_root, home_dir};
use crate::realfs::RealFs;
use crate::render;
use crate::state::{OpKind, OpLog, State};

/// 解除一个路径的纳管。
///
/// # Params:
///   - `path`: $HOME 侧的链接位置
///   - `keep_in_repo`: 为真则只删链、保留仓库副本；否则把文件搬回 $HOME
pub fn run(path: &PathBuf, keep_in_repo: bool) -> Result<()> {
    let repo_root = find_repo_root()?;
    let _home = home_dir()?;
    let fs = RealFs::new(repo_root.clone(), crate::timestamp());
    let mut state = State::load(&repo_root)?;

    let target = path.canonicalize().unwrap_or_else(|_| path.clone());
    // 找到对应仓库源。
    let source = state
        .links
        .iter()
        .find(|link| link.target == *path || link.target == target)
        .map(|link| link.source.clone())
        .ok_or_else(|| eyre!("台账无此链接：{}", path.display()))?;

    if fs_is_symlink(path) {
        fs.remove_symlink(path)?;
    }
    if !keep_in_repo {
        fs::rename(&source, path).map_err(|err| eyre!("搬回失败：{err}"))?;
        state.ops.push(OpLog {
            kind: OpKind::Unlink,
            home_path: path.clone(),
            repo_path: source.clone(),
        });
    }
    state.links.retain(|link| link.source != source);
    state.save(&repo_root)?;
    render::ok(&format!("已解除纳管：{}", path.display()));
    Ok(())
}

/// 判断路径是否为符号链接。
fn fs_is_symlink(path: &std::path::Path) -> bool {
    fs::symlink_metadata(path)
        .map(|meta| meta.file_type().is_symlink())
        .unwrap_or(false)
}
