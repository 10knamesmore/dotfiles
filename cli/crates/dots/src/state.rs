//! `.dots/state.json` 台账。
//!
//! 记录 dots 建过的链接、钩子认领的 keypath（漂移检测）、run_once 已执行键、
//! 以及 adopt/unlink 操作日志（undo 用）。机器本地状态，不入库。

use std::fs;
use std::path::{Path, PathBuf};

use color_eyre::eyre::Context;
use serde::{Deserialize, Serialize};

/// 通用 Result 别名。
pub type Result<T> = color_eyre::Result<T>;

/// 台账根。
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct State {
    /// dots 建立的链接记录。
    #[serde(default)]
    pub links: Vec<LinkRecord>,
    /// 钩子原语认领的「文件 → keypath」所有权（doctor 漂移检测）。
    #[serde(default)]
    pub owned: Vec<OwnRecord>,
    /// 已执行的 run_once 键。
    #[serde(default)]
    pub run_once: Vec<String>,
    /// 可撤销操作日志（最后一条供 `undo`）。
    #[serde(default)]
    pub ops: Vec<OpLog>,
}

/// 一条链接记录。
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct LinkRecord {
    /// 链接位置（$HOME 侧）。
    pub target: PathBuf,
    /// 指向的仓库内源。
    pub source: PathBuf,
}

/// 钩子认领的所有权记录。
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct OwnRecord {
    /// 被认领的目标文件。
    pub file: PathBuf,
    /// 认领的 keypath（如 `hooks.Stop`）或 managed-block marker。
    pub keypath: String,
}

/// 可撤销操作（adopt / unlink）。
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct OpLog {
    /// 操作类型。
    pub kind: OpKind,
    /// $HOME 侧位置。
    pub home_path: PathBuf,
    /// 仓库内位置。
    pub repo_path: PathBuf,
}

/// 操作类型。
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum OpKind {
    /// adopt：文件搬入仓库 + 原地反链。
    Adopt,
    /// unlink：删链 + 文件搬回 $HOME。
    Unlink,
}

impl State {
    /// 从 `repo_root/.dots/state.json` 加载；不存在则返回默认。
    pub fn load(repo_root: &Path) -> Result<Self> {
        let path = Self::path(repo_root);
        match fs::read_to_string(&path) {
            Err(_) => Ok(Self::default()),
            Ok(text) => {
                let st = serde_json::from_str(&text)
                    .wrap_err_with(|| format!("解析 state.json 失败：{}", path.display()))?;
                Ok(st)
            }
        }
    }

    /// 写回 `repo_root/.dots/state.json`。
    pub fn save(&self, repo_root: &Path) -> Result<()> {
        let path = Self::path(repo_root);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .wrap_err_with(|| format!("建 .dots 目录失败：{}", parent.display()))?;
        }
        let json = serde_json::to_string_pretty(self).wrap_err("序列化 state 失败")?;
        fs::write(&path, json)
            .wrap_err_with(|| format!("写 state.json 失败：{}", path.display()))?;
        Ok(())
    }

    /// 记一条链接（去重）。
    pub fn record_link(&mut self, target: PathBuf, source: PathBuf) {
        let rec = LinkRecord { target, source };
        if !self.links.contains(&rec) {
            self.links.push(rec);
        }
    }

    /// 记一条所有权（去重）。
    pub fn record_own(&mut self, file: PathBuf, keypath: String) {
        let rec = OwnRecord { file, keypath };
        if !self.owned.contains(&rec) {
            self.owned.push(rec);
        }
    }

    /// 某 run_once 键是否已执行。
    pub fn has_run(&self, key: &str) -> bool {
        self.run_once.iter().any(|existing| existing == key)
    }

    /// 标记 run_once 键已执行。
    pub fn mark_run(&mut self, key: impl Into<String>) {
        let key = key.into();
        if !self.has_run(&key) {
            self.run_once.push(key);
        }
    }

    /// 拼出 `repo_root/.dots/state.json` 的绝对路径。
    fn path(repo_root: &Path) -> PathBuf {
        repo_root.join(".dots").join("state.json")
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::min_ident_chars, clippy::missing_docs_in_private_items)]
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn round_trip() -> Result<()> {
        let dir = tempdir()?;
        let mut st = State::default();
        st.record_link("/home/u/.vimrc".into(), "/r/tree/home/.vimrc".into());
        st.record_own("/home/u/.claude/settings.json".into(), "hooks.Stop".into());
        st.mark_run("zoxide-import");
        st.save(dir.path())?;

        let loaded = State::load(dir.path())?;
        assert_eq!(loaded.links.len(), 1);
        assert_eq!(loaded.owned.len(), 1);
        assert!(loaded.has_run("zoxide-import"));
        Ok(())
    }

    #[test]
    fn dedup() {
        let mut st = State::default();
        st.record_link("/a".into(), "/b".into());
        st.record_link("/a".into(), "/b".into());
        assert_eq!(st.links.len(), 1);
        st.mark_run("k");
        st.mark_run("k");
        assert_eq!(st.run_once.len(), 1);
    }
}
