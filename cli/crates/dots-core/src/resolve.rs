//! 链接判定（§3.1）—— 无缝迁移的核心。
//!
//! 对每个期望链接，按目标位置当前状态算出 [`PlanAction`]；判定只看「readlink 是否落在
//! 仓库内」，不看新旧路径，故 `git mv` 后的断链旧链也被识别为可重建。

use std::path::{Path, PathBuf};

use rustc_hash::FxHashSet;

use crate::fs::{FileSystem, NodeKind};
use crate::layer::ExpectedLink;
use crate::plan::{Plan, PlanAction, PlanItem};
use crate::types::AbsPath;

/// 把期望链接集合解析为 Plan。
///
/// # Params:
///   - `fs`: 文件系统（读目标现状）
///   - `repo_root`: 仓库根（判定「落在仓库内」的前缀）
///   - `links`: 期望链接集合（来自 [`crate::layer::expand_layers`] 或 distribute）
///
/// # Return:
///   完整 Plan：含每条链接的动作，以及必要的 [`PlanAction::ContainerConvert`]
///   （当某链接的祖先目录当前是落仓库内的软链时，先把它拆成真实目录）。
pub fn resolve(fs: &dyn FileSystem, repo_root: &AbsPath, links: &[ExpectedLink]) -> Plan {
    let mut plan = Plan::new();

    // 先处理「容器祖先是软链」：收集所有 link target 的祖先目录，
    // 若祖先当前是落仓库内的软链，则需先 ContainerConvert。
    let mut converted: FxHashSet<PathBuf> = FxHashSet::default();
    for link in links {
        for ancestor in proper_ancestors(link.target.as_path()) {
            if converted.contains(&ancestor) {
                continue;
            }
            if let NodeKind::Symlink { target } = fs.classify(&ancestor) {
                if inside_repo(&target, repo_root) {
                    plan.push(PlanItem {
                        target: AbsPath::new(ancestor.clone()),
                        action: PlanAction::ContainerConvert {
                            source_dir: AbsPath::new(target),
                        },
                    });
                    converted.insert(ancestor);
                }
            }
        }
    }

    for link in links {
        let action = resolve_one(fs, repo_root, link);
        plan.push(PlanItem {
            target: link.target.clone(),
            action,
        });
    }
    plan
}

/// 单条链接的判定。
fn resolve_one(fs: &dyn FileSystem, repo_root: &AbsPath, link: &ExpectedLink) -> PlanAction {
    match fs.classify(link.target.as_path()) {
        NodeKind::Missing => PlanAction::Link {
            source: link.source.clone(),
        },
        NodeKind::Symlink { target } => {
            if target == link.source.as_path() {
                PlanAction::Noop
            } else if inside_repo(&target, repo_root) {
                PlanAction::Relink {
                    source: link.source.clone(),
                    old_target: target,
                }
            } else {
                PlanAction::DriftForeign { points_to: target }
            }
        }
        NodeKind::File | NodeKind::Dir => PlanAction::BackupThenLink {
            source: link.source.clone(),
        },
    }
}

/// 判定一个 readlink 结果是否落在仓库内（断链也按字符串前缀判）。
fn inside_repo(target: &Path, repo_root: &AbsPath) -> bool {
    target.starts_with(repo_root.as_path())
}

/// 一个路径的所有真祖先（不含自身），从浅到深。
fn proper_ancestors(path: &Path) -> Vec<PathBuf> {
    let mut acc: Vec<PathBuf> = Vec::new();
    let mut cur = path.parent();
    while let Some(parent) = cur {
        if parent.as_os_str().is_empty() {
            break;
        }
        acc.push(parent.to_owned());
        cur = parent.parent();
    }
    acc.reverse();
    acc
}

#[cfg(test)]
mod tests {
    #![allow(clippy::min_ident_chars, clippy::missing_docs_in_private_items)]
    use super::*;
    use crate::fs::MemFs;
    use crate::types::Layer;

    fn link(target: &str, source: &str) -> ExpectedLink {
        ExpectedLink {
            target: AbsPath::new(target),
            source: AbsPath::new(source),
            via: Layer::parse("home"),
            shadowed: vec![],
        }
    }

    fn first_action(plan: &Plan, target: &str) -> PlanAction {
        plan.items
            .iter()
            .find(|i| i.target.as_path() == Path::new(target))
            .map(|i| i.action.clone())
            .unwrap_or(PlanAction::Noop)
    }

    #[test]
    fn missing_target_links() {
        let fs = MemFs::new();
        let root = AbsPath::new("/r");
        let plan = resolve(&fs, &root, &[link("/home/u/.vimrc", "/r/tree/home/.vimrc")]);
        assert_eq!(
            first_action(&plan, "/home/u/.vimrc"),
            PlanAction::Link {
                source: AbsPath::new("/r/tree/home/.vimrc")
            }
        );
    }

    #[test]
    fn old_link_inside_repo_relinks() {
        let mut fs = MemFs::new();
        // 旧链指向 general/（仓库内旧路径），git mv 后断链
        fs.symlink("/home/u/.config/nvim", "/r/general/.config/nvim");
        let root = AbsPath::new("/r");
        let plan = resolve(
            &fs,
            &root,
            &[link("/home/u/.config/nvim", "/r/tree/home/.config/nvim")],
        );
        assert!(matches!(
            first_action(&plan, "/home/u/.config/nvim"),
            PlanAction::Relink { .. }
        ));
    }

    #[test]
    fn foreign_link_reports_drift() {
        let mut fs = MemFs::new();
        fs.symlink("/home/u/.config/x", "/opt/other/x");
        let root = AbsPath::new("/r");
        let plan = resolve(
            &fs,
            &root,
            &[link("/home/u/.config/x", "/r/tree/home/.config/x")],
        );
        assert!(matches!(
            first_action(&plan, "/home/u/.config/x"),
            PlanAction::DriftForeign { .. }
        ));
    }

    #[test]
    fn real_file_backs_up() {
        let mut fs = MemFs::new();
        fs.file("/home/u/.vimrc");
        let root = AbsPath::new("/r");
        let plan = resolve(&fs, &root, &[link("/home/u/.vimrc", "/r/tree/home/.vimrc")]);
        assert!(matches!(
            first_action(&plan, "/home/u/.vimrc"),
            PlanAction::BackupThenLink { .. }
        ));
    }

    #[test]
    fn correct_link_is_noop() {
        let mut fs = MemFs::new();
        fs.symlink("/home/u/.vimrc", "/r/tree/home/.vimrc");
        let root = AbsPath::new("/r");
        let plan = resolve(&fs, &root, &[link("/home/u/.vimrc", "/r/tree/home/.vimrc")]);
        assert_eq!(first_action(&plan, "/home/u/.vimrc"), PlanAction::Noop);
    }

    #[test]
    fn container_ancestor_softlink_converts() {
        let mut fs = MemFs::new();
        // ~/.claude 当前是整目录软链（旧 → general/skills），但现在期望在其下逐项链
        fs.symlink("/home/u/.claude", "/r/general/skills");
        let root = AbsPath::new("/r");
        let plan = resolve(
            &fs,
            &root,
            &[link(
                "/home/u/.claude/skills/foo",
                "/r/tree/home/.claude/skills/foo",
            )],
        );
        // 应先对 ~/.claude 产出 ContainerConvert
        assert!(matches!(
            first_action(&plan, "/home/u/.claude"),
            PlanAction::ContainerConvert { .. }
        ));
    }
}
