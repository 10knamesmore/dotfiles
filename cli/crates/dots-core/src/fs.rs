//! 文件系统读抽象。
//!
//! core 算 Plan 只需「读」现状，故 trait 只暴露读能力；写副作用全在 `dots` bin
//! 的 executor。这是「Plan 离线可测」的关键边界——测试用 [`MemFs`] 注入任意现状。

use std::path::{Path, PathBuf};

use rustc_hash::FxHashMap;

/// 目标位置的当前状态（§3.1 判定的输入）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NodeKind {
    /// 不存在。
    Missing,
    /// 普通文件。
    File,
    /// 真实目录（非符号链接）。
    Dir,
    /// 符号链接，`target` 为 `readlink` 结果（可能断链，即指向不存在的路径）。
    Symlink {
        /// 链接指向的原始路径（未规范化）。
        target: PathBuf,
    },
}

/// 只读文件系统抽象。
pub trait FileSystem {
    /// 判定路径当前状态。符号链接不跟随（区分 link 与其指向）。
    fn classify(&self, path: &Path) -> NodeKind;

    /// 列出目录下的一级条目（绝对/相对取决于传入 `path`）。目录不存在或非目录时返回空。
    fn read_dir(&self, path: &Path) -> Vec<PathBuf>;
}

/// 内存假文件系统，仅供测试构造任意现状。
#[derive(Default)]
pub struct MemFs {
    /// 路径 → 节点状态。
    nodes: FxHashMap<PathBuf, NodeKind>,
}

impl MemFs {
    /// 新建空内存文件系统。
    pub fn new() -> Self {
        Self::default()
    }

    /// 登记一个普通文件。
    pub fn file(&mut self, path: impl Into<PathBuf>) -> &mut Self {
        self.nodes.insert(path.into(), NodeKind::File);
        self
    }

    /// 登记一个真实目录。
    pub fn dir(&mut self, path: impl Into<PathBuf>) -> &mut Self {
        self.nodes.insert(path.into(), NodeKind::Dir);
        self
    }

    /// 登记一个符号链接（`target` 可指向不存在的路径以模拟断链）。
    pub fn symlink(&mut self, at: impl Into<PathBuf>, target: impl Into<PathBuf>) -> &mut Self {
        self.nodes.insert(
            at.into(),
            NodeKind::Symlink {
                target: target.into(),
            },
        );
        self
    }
}

impl FileSystem for MemFs {
    fn classify(&self, path: &Path) -> NodeKind {
        self.nodes.get(path).cloned().unwrap_or(NodeKind::Missing)
    }

    fn read_dir(&self, path: &Path) -> Vec<PathBuf> {
        // 返回直接子项（仅一级）。
        let mut out: Vec<PathBuf> = self
            .nodes
            .keys()
            .filter(|key| key.parent() == Some(path))
            .cloned()
            .collect();
        out.sort();
        out
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::min_ident_chars, clippy::missing_docs_in_private_items)]
    use super::*;

    #[test]
    fn classify_states() {
        let mut fs = MemFs::new();
        fs.file("/a").dir("/d").symlink("/l", "/target");
        assert_eq!(fs.classify(Path::new("/a")), NodeKind::File);
        assert_eq!(fs.classify(Path::new("/d")), NodeKind::Dir);
        assert_eq!(
            fs.classify(Path::new("/l")),
            NodeKind::Symlink {
                target: PathBuf::from("/target")
            }
        );
        assert_eq!(fs.classify(Path::new("/missing")), NodeKind::Missing);
    }

    #[test]
    fn read_dir_one_level() {
        let mut fs = MemFs::new();
        // 只列已登记节点中 parent == p 的直接子项；深层项需各自登记。
        fs.file("/d/a").file("/d/b").dir("/d/sub").file("/d/sub/c");
        assert_eq!(
            fs.read_dir(Path::new("/d")),
            vec![
                PathBuf::from("/d/a"),
                PathBuf::from("/d/b"),
                PathBuf::from("/d/sub")
            ]
        );
    }
}
