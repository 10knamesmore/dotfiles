//! 核心 newtype 与枚举。
//!
//! 项目约定：不用裸 `String`/`PathBuf` 表业务路径，包一层赋予语义。

use std::path::{Path, PathBuf};

/// 相对仓库根的路径，如 `tree/home/.config/nvim`。
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RepoPath(PathBuf);

impl RepoPath {
    /// 从任意可转为 `PathBuf` 的值构造。
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self(path.into())
    }

    /// 借出内部 `Path`。
    pub fn as_path(&self) -> &Path {
        &self.0
    }

    /// 在末尾拼接一段，返回新的 `RepoPath`。
    pub fn join(&self, seg: impl AsRef<Path>) -> Self {
        Self(self.0.join(seg))
    }

    /// 解析为绝对路径（给定仓库根的绝对路径）。
    pub fn to_abs(&self, repo_root: &AbsPath) -> AbsPath {
        AbsPath::new(repo_root.as_path().join(&self.0))
    }
}

/// 绝对路径，如 `/home/wanger/.config/nvim`。
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AbsPath(PathBuf);

impl AbsPath {
    /// 从任意可转为 `PathBuf` 的值构造。
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self(path.into())
    }

    /// 借出内部 `Path`。
    pub fn as_path(&self) -> &Path {
        &self.0
    }

    /// 在末尾拼接一段，返回新的 `AbsPath`。
    pub fn join(&self, seg: impl AsRef<Path>) -> Self {
        Self(self.0.join(seg))
    }

    /// 是否以 `prefix` 为前缀（用于「落在仓库内」判定）。
    pub fn starts_with(&self, prefix: &Self) -> bool {
        self.0.starts_with(&prefix.0)
    }
}

/// 操作系统平台。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Os {
    /// Linux。
    Linux,
    /// macOS。
    Macos,
}

impl Os {
    /// 解析层名后缀，如 `"linux"` → `Some(Os::Linux)`。
    pub fn parse_suffix(suffix: &str) -> Option<Self> {
        match suffix {
            "linux" => Some(Self::Linux),
            "macos" => Some(Self::Macos),
            _ => None,
        }
    }
}

/// 链接粒度（§3 规则 2/3）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum LinkMode {
    /// 整目录链。
    #[default]
    Dir,
    /// 容器：下钻一层、逐子项链。
    Children,
    /// 逐文件链。
    File,
}

/// 一个映射层，由 `tree/` 下的层目录名解析而来。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Layer {
    /// 层目录名，如 `home`、`home.linux`。
    pub name: String,
    /// 若该层仅在某平台生效，则为 `Some`。
    pub os: Option<Os>,
}

impl Layer {
    /// 解析层目录名。
    ///
    /// # Params:
    ///   - `dir_name`: `tree/` 下的一级目录名，如 `"home"` / `"home.linux"`
    ///
    /// # Return:
    ///   解析后的 `Layer`；`home` → `os=None`，`home.linux` → `os=Some(Linux)`。
    pub fn parse(dir_name: &str) -> Self {
        match dir_name.split_once('.') {
            Some((_base, suffix)) => Self {
                name: dir_name.to_owned(),
                os: Os::parse_suffix(suffix),
            },
            None => Self {
                name: dir_name.to_owned(),
                os: None,
            },
        }
    }

    /// 该层在给定平台是否生效（通用层恒生效；平台层仅匹配平台生效）。
    pub fn active_on(&self, os: Os) -> bool {
        match self.os {
            None => true,
            Some(layer_os) => layer_os == os,
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::min_ident_chars, clippy::missing_docs_in_private_items)]
    use super::*;

    #[test]
    fn repo_path_join_and_abs() {
        let rp = RepoPath::new("tree/home").join(".vimrc");
        assert_eq!(rp.as_path(), Path::new("tree/home/.vimrc"));
        let root = AbsPath::new("/home/wanger/dotfiles");
        assert_eq!(
            rp.to_abs(&root).as_path(),
            Path::new("/home/wanger/dotfiles/tree/home/.vimrc")
        );
    }

    #[test]
    fn abs_path_starts_with() {
        let repo = AbsPath::new("/home/wanger/dotfiles");
        let inside = AbsPath::new("/home/wanger/dotfiles/tree/home/.vimrc");
        let outside = AbsPath::new("/etc/passwd");
        assert!(inside.starts_with(&repo));
        assert!(!outside.starts_with(&repo));
    }

    #[test]
    fn link_mode_default_is_dir() {
        assert_eq!(LinkMode::default(), LinkMode::Dir);
    }

    #[test]
    fn layer_parse() {
        assert_eq!(
            Layer::parse("home"),
            Layer {
                name: "home".into(),
                os: None
            }
        );
        assert_eq!(
            Layer::parse("home.linux"),
            Layer {
                name: "home.linux".into(),
                os: Some(Os::Linux)
            }
        );
        assert_eq!(
            Layer::parse("home.macos"),
            Layer {
                name: "home.macos".into(),
                os: Some(Os::Macos)
            }
        );
    }

    #[test]
    fn layer_active_on() {
        assert!(Layer::parse("home").active_on(Os::Linux));
        assert!(Layer::parse("home.linux").active_on(Os::Linux));
        assert!(!Layer::parse("home.linux").active_on(Os::Macos));
    }
}
