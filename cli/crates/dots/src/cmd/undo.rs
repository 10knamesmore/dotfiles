//! `dots undo` —— 撤销上一次 adopt/unlink 的文件搬运与链接（§5）。

use std::fs;

use color_eyre::eyre::eyre;

use super::{Result, find_repo_root};
use crate::realfs::RealFs;
use crate::render;
use crate::state::{OpKind, State};

/// 撤销最近一次可逆操作。
pub fn run() -> Result<()> {
    let repo_root = find_repo_root()?;
    let fs = RealFs::new(repo_root.clone(), crate::timestamp());
    let mut state = State::load(&repo_root)?;

    let Some(op) = state.ops.pop() else {
        render::warn("无可撤销操作");
        return Ok(());
    };
    match op.kind {
        OpKind::Adopt => {
            // 逆向：删反链、把仓库文件搬回 $HOME。
            if fs_is_symlink(&op.home_path) {
                fs.remove_symlink(&op.home_path)?;
            }
            fs::rename(&op.repo_path, &op.home_path)
                .map_err(|err| eyre!("搬回失败：{} ({err})", op.repo_path.display()))?;
            state.links.retain(|link| link.target != op.home_path);
            render::ok(&format!(
                "已撤销 adopt：{} 搬回原处",
                op.home_path.display()
            ));
        }
        OpKind::Unlink => {
            // 逆向：把文件从 $HOME 搬回仓库 + 重建反链。
            fs::rename(&op.home_path, &op.repo_path).map_err(|err| eyre!("搬回仓库失败：{err}"))?;
            fs.make_symlink(&op.repo_path, &op.home_path)?;
            render::ok(&format!(
                "已撤销 unlink：{} 重新纳管",
                op.home_path.display()
            ));
        }
    }
    state.save(&repo_root)?;
    Ok(())
}

/// 判断路径是否为符号链接。
fn fs_is_symlink(path: &std::path::Path) -> bool {
    fs::symlink_metadata(path)
        .map(|meta| meta.file_type().is_symlink())
        .unwrap_or(false)
}
