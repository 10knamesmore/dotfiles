//! `.dots/state.json` Applied Inventory。

use std::fs;
use std::path::{Path, PathBuf};

use color_eyre::eyre::{Context, eyre};
use dots_core::{AppliedResource, OwnershipSurface};
use serde::{Deserialize, Serialize};

/// 当前 state schema；旧 schema 直接拒绝，避免旧 Cargo file ownership 被解释为 retirement。
const STATE_VERSION: u32 = 4;

/// 通用 Result 别名。
pub type Result<T> = color_eyre::Result<T>;

/// 本机 Applied Inventory。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct State {
    /// 必须精确匹配当前 schema 的版本号。
    version: u32,

    /// 上一次成功由 dots 拥有的 Resource。
    pub resources: Vec<AppliedResource>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            version: STATE_VERSION,
            resources: Vec::new(),
        }
    }
}

/// 只读取 schema version 的最小结构。
#[derive(Deserialize)]
struct VersionProbe {
    /// 缺失表示旧 schema。
    version: Option<u32>,
}

impl State {
    /// 从 `repo_root/.dots/state.json` 加载；不存在时返回空 inventory。
    ///
    /// # Error:
    ///   文件存在但 schema 不是当前版本时，要求用户明确重置，且不读取旧字段。
    pub fn load(repo_root: &Path) -> Result<Self> {
        let path = Self::path(repo_root);
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::default());
            }
            Err(error) => {
                return Err(error)
                    .wrap_err_with(|| format!("读取 state.json 失败：{}", path.display()));
            }
        };
        let probe: VersionProbe = serde_json::from_str(&text)
            .wrap_err_with(|| format!("解析 state.json 版本失败：{}", path.display()))?;
        if probe.version != Some(STATE_VERSION) {
            return Err(eyre!(
                "state.json schema 不兼容：需要 version {STATE_VERSION}；请确认后删除 {} 并重新 sync",
                path.display()
            ));
        }
        serde_json::from_str(&text)
            .wrap_err_with(|| format!("解析 state.json 失败：{}", path.display()))
    }

    /// 原子写回 Applied Inventory。
    pub fn save(&self, repo_root: &Path) -> Result<()> {
        let path = Self::path(repo_root);
        let parent = path
            .parent()
            .ok_or_else(|| eyre!("state.json 无父目录：{}", path.display()))?;
        fs::create_dir_all(parent)
            .wrap_err_with(|| format!("建 .dots 目录失败：{}", parent.display()))?;
        let json = serde_json::to_vec_pretty(self).wrap_err("序列化 state 失败")?;
        let temporary = path.with_extension("json.dots-tmp");
        fs::write(&temporary, json)
            .wrap_err_with(|| format!("写临时 state 失败：{}", temporary.display()))?;
        fs::rename(&temporary, &path)
            .wrap_err_with(|| format!("提交 state.json 失败：{}", path.display()))?;
        Ok(())
    }

    /// 插入或替换一个成功 Applied Resource。
    pub fn upsert_resource(&mut self, resource: AppliedResource) {
        self.resources
            .retain(|existing| existing.surface != resource.surface);
        self.resources.push(resource);
        self.resources
            .sort_by(|left, right| left.surface.cmp(&right.surface));
    }

    /// 删除一个 surface 的 Applied Inventory 记录。
    pub fn remove_resource(&mut self, surface: &OwnershipSurface) -> bool {
        let before = self.resources.len();
        self.resources
            .retain(|resource| &resource.surface != surface);
        self.resources.len() != before
    }

    /// 返回 selector 对应的唯一 Applied Resource。
    pub fn find_resource(&self, selector: &str) -> Result<&AppliedResource> {
        let mut matches = self
            .resources
            .iter()
            .filter(|resource| resource_matches_selector(resource, selector));
        let first = matches
            .next()
            .ok_or_else(|| eyre!("Applied Inventory 中没有 Resource：{selector}"))?;
        if matches.next().is_some() {
            return Err(eyre!("Resource selector 不唯一：{selector}"));
        }
        Ok(first)
    }

    /// 返回 state 文件路径。
    fn path(repo_root: &Path) -> PathBuf {
        repo_root.join(".dots").join("state.json")
    }
}

/// 支持完整 selector 与 filesystem path 两种人类输入。
fn resource_matches_selector(resource: &AppliedResource, selector: &str) -> bool {
    if resource.surface.selector() == selector {
        return true;
    }
    resource
        .surface
        .filesystem_path()
        .is_some_and(|path| path == Path::new(selector))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::min_ident_chars, clippy::missing_docs_in_private_items)]

    use dots_core::ResourceState;
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn current_schema_round_trips() -> Result<()> {
        let directory = tempdir()?;
        let mut state = State::default();
        state.upsert_resource(AppliedResource {
            surface: OwnershipSurface::Path {
                path: "/home/u/.vimrc".into(),
            },
            state: ResourceState::Symlink {
                source: "/repo/tree/home/.vimrc".into(),
            },
        });
        state.save(directory.path())?;
        let loaded = State::load(directory.path())?;
        assert_eq!(loaded.resources, state.resources);
        Ok(())
    }

    #[test]
    fn legacy_schema_is_rejected() -> Result<()> {
        let directory = tempdir()?;
        let state_dir = directory.path().join(".dots");
        fs::create_dir_all(&state_dir)?;
        fs::write(state_dir.join("state.json"), br#"{"links":[]}"#)?;
        let error = State::load(directory.path()).err();
        assert!(error.is_some());
        Ok(())
    }
}
