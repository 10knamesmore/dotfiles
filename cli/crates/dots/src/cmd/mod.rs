//! 命令实现 + 共享辅助（仓库根/HOME/平台/路径展开）。

pub mod adopt;
pub mod bootstrap;
pub mod completions;
pub mod doctor;
pub mod secret;
pub mod status;
pub mod sync;
pub mod undo;
pub mod unlink;

use std::path::{Path, PathBuf};

use color_eyre::eyre::eyre;
use dots_core::Os;

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
    // 从 cwd 向上找：认 dots.lua（已迁移）或 cli/Cargo.toml（迁移前的 dots 仓库标志）。
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
