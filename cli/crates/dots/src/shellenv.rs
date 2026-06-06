//! shell 环境注入：`~/.zshrc` stub（兼容旧 marker）+ `~/.config/dots/{env.zsh,root}`。

use std::path::Path;

use crate::realfs::RealFs;

/// 通用 Result 别名。
pub type Result<T> = color_eyre::Result<T>;

/// 新 marker 行（首行标识）。
const MARKER_NEW: &str = "# DOTS_MANAGED:";
/// 旧 marker（install.py 时代），迁移期兼容识别。
const MARKER_OLD: &str = "# DOTFILES_MANAGED:";
/// stub 必须保证存在的 source 行。
const SOURCE_LINE: &str = "source \"$HOME/.zshrc_dotfiles\"";

/// 确保 `~/.zshrc` 是受管 stub，且不覆盖软件追加内容。
///
/// 规则（§8）：
/// - 首行命中新/旧 marker → 仅确保 source 行在，其余行不动（旧 marker 升级为新）。
/// - 无 marker 的普通文件 → 备份后写 stub。
/// - 软链或不存在 → 写 stub。
pub fn ensure_zshrc_stub(home: &Path, fs: &RealFs) -> Result<()> {
    let zshrc = home.join(".zshrc");
    let stub = format!(
        "{MARKER_NEW} 由 dots 维护。下面这行勿删；其余内容（conda/nvm 等追加）安全保留。\n{SOURCE_LINE}\n"
    );

    match std::fs::read_to_string(&zshrc) {
        Ok(content) => {
            let first = content.lines().next().unwrap_or("");
            if first.starts_with(MARKER_NEW) || first.starts_with(MARKER_OLD) {
                // 已受管：保留其余行，确保 source 行存在，旧 marker 升级。
                let mut lines: Vec<String> = content.lines().map(|line| line.to_owned()).collect();
                if let Some(first_line) = lines.first_mut() {
                    if first_line.starts_with(MARKER_OLD) {
                        *first_line = format!("{MARKER_NEW} 由 dots 维护（自旧 dotfiles 升级）。");
                    }
                }
                if !lines.iter().any(|line| line.trim() == SOURCE_LINE) {
                    // source 行缺失则补在 marker 后。
                    lines.insert(1, SOURCE_LINE.to_owned());
                }
                let rebuilt = format!("{}\n", lines.join("\n"));
                fs.write_atomic(&zshrc, rebuilt.as_bytes())?;
            } else {
                // 陌生普通文件：备份后接管。
                fs.backup(&zshrc)?;
                fs.write_atomic(&zshrc, stub.as_bytes())?;
            }
        }
        Err(_) => {
            // 不存在或是软链（read_to_string 对软链会跟随；这里按内容缺失处理）。
            if fs_is_symlink(&zshrc) {
                fs.remove_symlink(&zshrc)?;
            }
            fs.write_atomic(&zshrc, stub.as_bytes())?;
        }
    }
    Ok(())
}

/// 写 `~/.config/dots/env.zsh`（路径注入 A）与 `~/.config/dots/root`（Hyprland 兜底，§6-C）。
pub fn write_shell_env(home: &Path, repo_root: &Path, fs: &RealFs) -> Result<()> {
    let dots_dir = home.join(".config").join("dots");
    let repo = repo_root.display();
    let env = format!(
        "# 由 dots 生成。消灭模板变量：配置引用 $DOTFILES_DIR / $DOTS_SCRIPTS。\n\
         export DOTFILES_DIR=\"{repo}\"\n\
         export DOTS_SCRIPTS=\"$DOTFILES_DIR/.gen/scripts\"\n\
         path=($DOTS_SCRIPTS $path)\n"
    );
    fs.write_atomic(&dots_dir.join("env.zsh"), env.as_bytes())?;
    fs.write_atomic(&dots_dir.join("root"), format!("{repo}\n").as_bytes())?;
    Ok(())
}

/// 判断给定路径本身是否为软链（不跟随）。
fn fs_is_symlink(path: &Path) -> bool {
    std::fs::symlink_metadata(path)
        .map(|meta| meta.file_type().is_symlink())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::min_ident_chars, clippy::missing_docs_in_private_items)]
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn creates_stub_when_absent() -> Result<()> {
        let dir = tempdir()?;
        let fs = RealFs::new(dir.path(), "ts");
        ensure_zshrc_stub(dir.path(), &fs)?;
        let content = std::fs::read_to_string(dir.path().join(".zshrc"))?;
        assert!(content.starts_with(MARKER_NEW));
        assert!(content.contains(SOURCE_LINE));
        Ok(())
    }

    #[test]
    fn preserves_appended_content_and_upgrades_old_marker() -> Result<()> {
        let dir = tempdir()?;
        let fs = RealFs::new(dir.path(), "ts");
        let zshrc = dir.path().join(".zshrc");
        // 旧 marker + 用户追加（conda）。
        std::fs::write(
            &zshrc,
            format!("{MARKER_OLD} old\n{SOURCE_LINE}\n# >>> conda init >>>\nexport X=1\n"),
        )?;
        ensure_zshrc_stub(dir.path(), &fs)?;
        let content = std::fs::read_to_string(&zshrc)?;
        assert!(content.starts_with(MARKER_NEW), "旧 marker 应升级为新");
        assert!(content.contains("conda init"), "追加内容应保留");
        assert!(content.contains("export X=1"));
        Ok(())
    }

    #[test]
    fn backs_up_foreign_file() -> Result<()> {
        let dir = tempdir()?;
        let fs = RealFs::new(dir.path(), "ts");
        let zshrc = dir.path().join(".zshrc");
        std::fs::write(&zshrc, "# my own zshrc\nalias x=y\n")?;
        ensure_zshrc_stub(dir.path(), &fs)?;
        let content = std::fs::read_to_string(&zshrc)?;
        assert!(content.starts_with(MARKER_NEW));
        // 原文件被备份
        assert!(dir.path().join("backup/ts/.zshrc").exists());
        Ok(())
    }

    #[test]
    fn writes_env_and_root() -> Result<()> {
        let dir = tempdir()?;
        let fs = RealFs::new(dir.path(), "ts");
        write_shell_env(dir.path(), Path::new("/home/u/dotfiles"), &fs)?;
        let env = std::fs::read_to_string(dir.path().join(".config/dots/env.zsh"))?;
        assert!(env.contains("DOTFILES_DIR=\"/home/u/dotfiles\""));
        let root = std::fs::read_to_string(dir.path().join(".config/dots/root"))?;
        assert_eq!(root.trim(), "/home/u/dotfiles");
        Ok(())
    }
}
