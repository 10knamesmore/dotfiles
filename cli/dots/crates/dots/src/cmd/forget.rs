//! `dots forget`：只从 Applied Inventory 放弃 ownership，不读取或修改真实状态。

use super::{Result, expand_home, find_repo_root, home_dir};
use crate::render;
use crate::state::State;

/// 从 Applied Inventory 移除指定 Resource，不构建 Plan 或修改真实状态。
pub fn run(selector: &str) -> Result<()> {
    let repo_root = find_repo_root()?;
    let mut state = State::load(&repo_root)?;
    let selector = if selector.starts_with("path:")
        || selector.starts_with("block:")
        || selector.starts_with("systemd-user:")
    {
        selector.to_owned()
    } else {
        let home = home_dir()?;
        let expanded = expand_home(selector, &home);
        expanded.display().to_string()
    };
    let surface = state.find_resource(&selector)?.surface.clone();
    state.remove_resource(&surface);
    state.save(&repo_root)?;
    render::ok(&format!(
        "已放弃 ownership，真实状态保持不变：{}",
        surface.selector()
    ));
    Ok(())
}
