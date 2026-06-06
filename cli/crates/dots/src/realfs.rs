//! 真实文件系统：实现 core 的 [`FileSystem`] 读 trait + sync/原语所需的写能力。
//!
//! 写能力统一保证：atomic（temp + rename）、写前备份、无差异不写盘。

use std::fs;
use std::os::unix::fs as unixfs;
use std::path::{Path, PathBuf};

use color_eyre::eyre::{Context, eyre};
use dots_core::{FileSystem, NodeKind};

/// 真实文件系统句柄，持有仓库根（用于 backup 落点）。
pub struct RealFs {
    /// 仓库根绝对路径。
    repo_root: PathBuf,
    /// 本次运行的统一备份时间戳（同一次 sync 的备份归一个目录）。
    stamp: String,
}

/// 通用 Result 别名。
pub type Result<T> = color_eyre::Result<T>;

impl RealFs {
    /// 新建。`stamp` 为 ISO 风格时间戳字符串（由调用方生成，便于测试注入）。
    pub fn new(repo_root: impl Into<PathBuf>, stamp: impl Into<String>) -> Self {
        Self {
            repo_root: repo_root.into(),
            stamp: stamp.into(),
        }
    }

    /// 原子写文件：若目标内容已与 `bytes` 相同则不写（返回 `false`）。
    ///
    /// # Return:
    ///   `true` 表示真正写盘，`false` 表示内容无差异跳过。
    pub fn write_atomic(&self, path: &Path, bytes: &[u8]) -> Result<bool> {
        if let Ok(existing) = fs::read(path) {
            if existing == bytes {
                return Ok(false);
            }
        }
        let parent = path
            .parent()
            .ok_or_else(|| eyre!("无父目录：{}", path.display()))?;
        fs::create_dir_all(parent).wrap_err_with(|| format!("建目录失败：{}", parent.display()))?;
        let tmp = path.with_extension(format!(
            "{}.dots-tmp",
            path.extension().and_then(|ext| ext.to_str()).unwrap_or("")
        ));
        fs::write(&tmp, bytes).wrap_err_with(|| format!("写临时文件失败：{}", tmp.display()))?;
        fs::rename(&tmp, path).wrap_err_with(|| format!("rename 失败：{}", path.display()))?;
        Ok(true)
    }

    /// 把目标移入 `backup/<stamp>/`，保留 `.config` 二级结构。
    pub fn backup(&self, path: &Path) -> Result<()> {
        let backup_root = self.repo_root.join("backup").join(&self.stamp);
        let name = path
            .file_name()
            .ok_or_else(|| eyre!("无文件名：{}", path.display()))?;
        let dest = if path
            .parent()
            .and_then(|parent| parent.file_name())
            .is_some_and(|name| name == ".config")
        {
            backup_root.join(".config").join(name)
        } else {
            backup_root.join(name)
        };
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)
                .wrap_err_with(|| format!("建备份目录失败：{}", parent.display()))?;
        }
        fs::rename(path, &dest)
            .wrap_err_with(|| format!("备份移动失败：{} → {}", path.display(), dest.display()))?;
        Ok(())
    }

    /// 建符号链接 `link_path → target`（先 mkdir 父目录）。
    pub fn make_symlink(&self, target: &Path, link_path: &Path) -> Result<()> {
        if let Some(parent) = link_path.parent() {
            fs::create_dir_all(parent)
                .wrap_err_with(|| format!("建父目录失败：{}", parent.display()))?;
        }
        unixfs::symlink(target, link_path).wrap_err_with(|| {
            format!("建链失败：{} → {}", link_path.display(), target.display())
        })?;
        Ok(())
    }

    /// 删除一个符号链接（不跟随）。
    pub fn remove_symlink(&self, path: &Path) -> Result<()> {
        fs::remove_file(path).wrap_err_with(|| format!("删链失败：{}", path.display()))?;
        Ok(())
    }

    /// 建真实目录（含父）。
    pub fn make_dir_all(&self, path: &Path) -> Result<()> {
        fs::create_dir_all(path).wrap_err_with(|| format!("建目录失败：{}", path.display()))?;
        Ok(())
    }
}

impl FileSystem for RealFs {
    fn classify(&self, path: &Path) -> NodeKind {
        match fs::symlink_metadata(path) {
            Err(_) => NodeKind::Missing,
            Ok(meta) => {
                let ft = meta.file_type();
                if ft.is_symlink() {
                    match fs::read_link(path) {
                        Ok(target) => NodeKind::Symlink { target },
                        Err(_) => NodeKind::Missing,
                    }
                } else if ft.is_dir() {
                    NodeKind::Dir
                } else {
                    NodeKind::File
                }
            }
        }
    }

    fn read_dir(&self, path: &Path) -> Vec<PathBuf> {
        let Ok(rd) = fs::read_dir(path) else {
            return Vec::new();
        };
        let mut out: Vec<PathBuf> = rd
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .collect();
        out.sort();
        out
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::min_ident_chars, clippy::missing_docs_in_private_items)]
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn atomic_write_skips_when_identical() -> Result<()> {
        let dir = tempdir()?;
        let fs = RealFs::new(dir.path(), "stamp");
        let f = dir.path().join("a.txt");
        assert!(fs.write_atomic(&f, b"hello")?); // 首次写
        assert!(!fs.write_atomic(&f, b"hello")?); // 内容相同 → 跳过
        assert!(fs.write_atomic(&f, b"world")?); // 内容变 → 写
        Ok(())
    }

    #[test]
    fn symlink_and_classify() -> Result<()> {
        let dir = tempdir()?;
        let fs = RealFs::new(dir.path(), "stamp");
        let target = dir.path().join("src");
        let link = dir.path().join("lnk");
        fs.write_atomic(&target, b"x")?;
        fs.make_symlink(&target, &link)?;
        assert!(matches!(fs.classify(&link), NodeKind::Symlink { .. }));
        assert_eq!(fs.classify(&target), NodeKind::File);
        Ok(())
    }

    #[test]
    fn backup_moves_file() -> Result<()> {
        let dir = tempdir()?;
        let fs = RealFs::new(dir.path(), "20260606T000000");
        let f = dir.path().join("orig");
        fs.write_atomic(&f, b"data")?;
        fs.backup(&f)?;
        assert_eq!(fs.classify(&f), NodeKind::Missing);
        let backed = dir.path().join("backup/20260606T000000/orig");
        assert_eq!(fs.classify(&backed), NodeKind::File);
        Ok(())
    }
}
