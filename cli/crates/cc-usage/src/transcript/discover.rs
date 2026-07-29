//! 找 transcript 文件。

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use walkdir::WalkDir;

/// 覆盖 transcript 根目录（测试与非常规安装用）。
pub const PROJECTS_ENV: &str = "CC_USAGE_PROJECTS_DIR";

/// transcript 根目录：`CC_USAGE_PROJECTS_DIR` 优先，否则 `$HOME/.claude/projects`。
#[must_use]
pub fn projects_root() -> Option<PathBuf> {
    if let Some(custom) = std::env::var_os(PROJECTS_ENV) {
        return Some(PathBuf::from(custom));
    }
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".claude/projects"))
}

/// 递归列出 `*.jsonl`。
///
/// 必须递归：subagent 的 transcript 落在 `<session-id>/subagents/` 子目录，那部分
/// token 常常比主线还多，只扫一级目录会少算一大截。
///
/// `since` 给定时只要 mtime 不早于它的——transcript 是追加写，当天零点后没再动过的
/// 文件不可能含当天的消息，跳过它能把「首次全量解析」从几百 MB 压到几 MB。
#[must_use]
pub fn list(root: &Path, since: Option<SystemTime>) -> Vec<PathBuf> {
    WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "jsonl"))
        .filter(|entry| match since {
            None => true,
            Some(cutoff) => entry
                .metadata()
                .ok()
                .and_then(|meta| meta.modified().ok())
                .is_some_and(|mtime| mtime >= cutoff),
        })
        .map(|entry| entry.into_path())
        .collect()
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::min_ident_chars,
        clippy::missing_docs_in_private_items
    )]
    use std::fs;

    use super::*;

    #[test]
    fn finds_nested_subagent_transcripts() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("proj/session/subagents")).unwrap();
        fs::write(root.join("proj/session.jsonl"), "").unwrap();
        fs::write(root.join("proj/session/subagents/agent-1.jsonl"), "").unwrap();
        fs::write(root.join("proj/notes.md"), "").unwrap();

        let mut found: Vec<String> = list(root, None)
            .iter()
            .filter_map(|path| {
                path.file_name()
                    .map(|name| name.to_string_lossy().into_owned())
            })
            .collect();
        found.sort();
        assert_eq!(found, vec!["agent-1.jsonl", "session.jsonl"]);
    }

    #[test]
    fn since_filter_drops_untouched_files() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::write(root.join("a.jsonl"), "").unwrap();
        let future = SystemTime::now() + std::time::Duration::from_secs(3600);
        assert!(list(root, Some(future)).is_empty());
        assert_eq!(list(root, None).len(), 1);
    }

    #[test]
    fn missing_root_yields_nothing() {
        assert!(list(Path::new("/nonexistent/projects"), None).is_empty());
    }
}
