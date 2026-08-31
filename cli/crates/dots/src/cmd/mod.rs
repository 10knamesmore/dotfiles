//! 命令实现 + 共享辅助（仓库根/HOME/平台/路径展开）。

pub mod forget;
pub mod install;
pub mod status;
pub mod sync;

use std::fs;
use std::path::{Path, PathBuf};

use color_eyre::eyre::{Context, eyre};
use dots_core::{Manifest, Os};

use crate::lua::{LuaCtx, eval_manifest};

/// 通用 Result 别名。
pub type Result<T> = color_eyre::Result<T>;

/// 定位仓库根：优先 `$DOTFILES_DIR`，否则从 cwd 向上找含 `dots.lua` 的目录。
pub fn find_repo_root() -> Result<PathBuf> {
    if let Ok(dir) = std::env::var("DOTFILES_DIR") {
        let path = PathBuf::from(dir);
        if path.join("dots.lua").exists() || path.join("cli").exists() {
            return Ok(path);
        }
    }
    // 从 cwd 向上查找 dots.lua 或 cli/Cargo.toml 仓库标志。
    let mut cur = std::env::current_dir()?;
    loop {
        if cur.join("dots.lua").exists() || cur.join("cli").join("Cargo.toml").exists() {
            return Ok(cur);
        }
        if !cur.pop() {
            break;
        }
    }
    Err(eyre!(
        "找不到仓库根（无 $DOTFILES_DIR，且 cwd 向上既无 dots.lua 也无 cli/Cargo.toml）"
    ))
}

/// 取 `$HOME`。
pub fn home_dir() -> Result<PathBuf> {
    std::env::var("HOME")
        .map(PathBuf::from)
        .map_err(|_| eyre!("$HOME 未设置"))
}

/// 当前平台。
pub fn current_os() -> Os {
    if cfg!(target_os = "macos") {
        Os::Macos
    } else {
        Os::Linux
    }
}

/// 平台字符串（注入 Lua）。
pub fn os_str(os: Os) -> &'static str {
    match os {
        Os::Linux => "linux",
        Os::Macos => "macos",
    }
}

/// 求值仓库的 `dots.lua`，返回当前机器上的完整声明。
pub fn load_manifest(repo_root: &Path, home: &Path, os: Os) -> Result<Manifest> {
    let source = fs::read_to_string(repo_root.join("dots.lua")).unwrap_or_default();
    let context = LuaCtx {
        os: os_str(os).to_owned(),
        home: home.display().to_string(),
        repo: repo_root.display().to_string(),
    };
    eval_manifest(&source, &context)
}

/// 把 `~` 前缀展开为 `$HOME`。
pub fn expand_home(path: &str, home: &Path) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        home.join(rest)
    } else if path == "~" {
        home.to_owned()
    } else {
        PathBuf::from(path)
    }
}

/// 解析声明 source；相对路径以仓库根为基准。
pub fn absolute_source(raw: &str, repo_root: &Path, home: &Path) -> Result<PathBuf> {
    let expanded = expand_home(raw, home);
    let joined = if expanded.is_absolute() {
        expanded
    } else {
        repo_root.join(expanded)
    };
    std::path::absolute(&joined)
        .wrap_err_with(|| format!("无法规范化 Resource source：{}", joined.display()))
}

/// 解析声明 target；只接受绝对路径或 `~`。
pub fn absolute_target(raw: &str, home: &Path) -> Result<PathBuf> {
    let expanded = expand_home(raw, home);
    if !expanded.is_absolute() {
        return Err(eyre!("Resource target 必须是绝对路径或 `~` 路径：{raw}"));
    }
    std::path::absolute(&expanded)
        .wrap_err_with(|| format!("无法规范化 Resource target：{}", expanded.display()))
}
