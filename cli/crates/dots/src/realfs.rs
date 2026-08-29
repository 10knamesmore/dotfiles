//! 真实文件系统：实现 core 的 [`FileSystem`] 观测接口和命令执行所需的写能力。
//!
//! 普通文件写入使用 atomic temp + rename；内容无差异时不写盘。

use std::fs;
use std::os::unix::fs as unixfs;
use std::path::{Path, PathBuf};

use color_eyre::eyre::{Context, eyre};
use dots_core::{FileSystem, NodeKind};

/// 真实文件系统读写入口。
#[derive(Default)]
pub struct RealFs;

/// 通用 Result 别名。
pub type Result<T> = color_eyre::Result<T>;

impl RealFs {
    /// 新建真实文件系统入口。
    pub fn new() -> Self {
        Self
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

    /// 原子写文件并确保 permission bits 与声明一致。
    pub fn write_atomic_with_mode(&self, path: &Path, bytes: &[u8], mode: u32) -> Result<bool> {
        let content_changed = self.write_atomic(path, bytes)?;
        let mut permissions = fs::metadata(path)
            .wrap_err_with(|| format!("读取文件权限失败：{}", path.display()))?
            .permissions();
        use std::os::unix::fs::PermissionsExt;
        if permissions.mode() & 0o777 != mode {
            permissions.set_mode(mode);
            fs::set_permissions(path, permissions)
                .wrap_err_with(|| format!("设置文件权限失败：{}", path.display()))?;
            return Ok(true);
        }
        Ok(content_changed)
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

    /// 删除普通文件或 symlink，不跟随 symlink，也不递归删除真实目录。
    pub fn remove_file(&self, path: &Path) -> Result<()> {
        let metadata = fs::symlink_metadata(path)
            .wrap_err_with(|| format!("读取待删路径失败：{}", path.display()))?;
        if metadata.file_type().is_dir() && !metadata.file_type().is_symlink() {
            return Err(eyre!("拒绝删除未声明的真实目录：{}", path.display()));
        }
        fs::remove_file(path).wrap_err_with(|| format!("删除文件失败：{}", path.display()))?;
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
        let fs = RealFs::new();
        let f = dir.path().join("a.txt");
        assert!(fs.write_atomic(&f, b"hello")?); // 首次写
        assert!(!fs.write_atomic(&f, b"hello")?); // 内容相同 → 跳过
        assert!(fs.write_atomic(&f, b"world")?); // 内容变 → 写
        Ok(())
    }

    #[test]
    fn symlink_and_classify() -> Result<()> {
        let dir = tempdir()?;
        let fs = RealFs::new();
        let target = dir.path().join("src");
        let link = dir.path().join("lnk");
        fs.write_atomic(&target, b"x")?;
        fs.make_symlink(&target, &link)?;
        assert!(matches!(fs.classify(&link), NodeKind::Symlink { .. }));
        assert_eq!(fs.classify(&target), NodeKind::File);
        Ok(())
    }
}
