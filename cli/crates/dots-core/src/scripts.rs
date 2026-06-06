//! scripts 聚合计划（§9）。
//!
//! `scripts/common` + `scripts/<os>` 聚合到 `.gen/scripts/`：顶层文件链过去；
//! 子目录默认整目录链（保持树形）；`ignore_tree` 列出的子目录递归拍平其文件。
//! 重名 → 冲突（doctor 用）。

use crate::fs::{FileSystem, NodeKind};
use crate::layer::ExpectedLink;
use crate::types::{AbsPath, Layer, Os};

/// 一个脚本名冲突（多个来源产出同一聚合目标）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScriptConflict {
    /// 聚合目标名（`.gen/scripts/` 下）。
    pub name: String,
    /// 冲突的来源们。
    pub sources: Vec<AbsPath>,
}

/// 计算 scripts 聚合链接 + 冲突。
///
/// # Params:
///   - `fs`: 文件系统
///   - `repo_root`: 仓库根
///   - `os`: 当前平台（决定 `scripts/<os>` 是否纳入）
///   - `ignore_tree`: 不保树形、递归拍平其文件的子目录名（子目录默认整目录链）
///
/// # Return:
///   `(聚合链接集合, 冲突列表)`；聚合目标位于 `repo_root/.gen/scripts/`。
pub fn plan_scripts(
    fs: &dyn FileSystem,
    repo_root: &AbsPath,
    os: Os,
    ignore_tree: &[String],
) -> (Vec<ExpectedLink>, Vec<ScriptConflict>) {
    let gen_dir = repo_root.join(".gen").join("scripts");
    let via = Layer {
        name: "scripts".to_owned(),
        os: None,
    };

    let mut links: Vec<ExpectedLink> = Vec::new();
    let mut conflicts: Vec<ScriptConflict> = Vec::new();

    let os_dir = match os {
        Os::Linux => "linux",
        Os::Macos => "macos",
    };
    for scope in ["common", os_dir] {
        let scope_dir = repo_root.join("scripts").join(scope);
        let mut entries = fs.read_dir(scope_dir.as_path());
        entries.sort();
        for entry in entries {
            let Some(name) = entry.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            let src = AbsPath::new(&entry);
            match fs.classify(&entry) {
                NodeKind::Dir if ignore_tree.iter().any(|flat| flat == name) => {
                    flatten_dir(fs, &src, &gen_dir, &via, &mut links, &mut conflicts);
                }
                NodeKind::Missing => {}
                // 文件、软链、或默认保树的子目录：直接链一条。
                _ => add_link(
                    &gen_dir.join(name),
                    &src,
                    name,
                    &via,
                    &mut links,
                    &mut conflicts,
                ),
            }
        }
    }
    (links, conflicts)
}

/// 递归把目录下所有文件拍平链到 `gen_dir/<filename>`。
fn flatten_dir(
    fs: &dyn FileSystem,
    dir: &AbsPath,
    gen_dir: &AbsPath,
    via: &Layer,
    links: &mut Vec<ExpectedLink>,
    conflicts: &mut Vec<ScriptConflict>,
) {
    let mut entries = fs.read_dir(dir.as_path());
    entries.sort();
    for entry in entries {
        let Some(name) = entry.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let src = AbsPath::new(&entry);
        match fs.classify(&entry) {
            NodeKind::Dir => flatten_dir(fs, &src, gen_dir, via, links, conflicts),
            NodeKind::Missing => {}
            _ => add_link(&gen_dir.join(name), &src, name, via, links, conflicts),
        }
    }
}

/// 追加一条聚合链，若目标已存在则记冲突。
fn add_link(
    target: &AbsPath,
    src: &AbsPath,
    name: &str,
    via: &Layer,
    links: &mut Vec<ExpectedLink>,
    conflicts: &mut Vec<ScriptConflict>,
) {
    if let Some(existing) = links.iter().find(|link| &link.target == target) {
        let existing_src = existing.source.clone();
        if let Some(conflict) = conflicts.iter_mut().find(|conflict| conflict.name == name) {
            if !conflict.sources.contains(src) {
                conflict.sources.push(src.clone());
            }
        } else {
            conflicts.push(ScriptConflict {
                name: name.to_owned(),
                sources: vec![existing_src, src.clone()],
            });
        }
        return;
    }
    links.push(ExpectedLink {
        target: target.clone(),
        source: src.clone(),
        via: via.clone(),
        shadowed: Vec::new(),
    });
}

#[cfg(test)]
mod tests {
    #![allow(clippy::min_ident_chars, clippy::missing_docs_in_private_items)]
    use super::*;
    use crate::fs::MemFs;
    use std::path::Path;

    #[test]
    fn aggregates_common_and_platform() {
        let mut fs = MemFs::new();
        fs.dir("/r/scripts")
            .dir("/r/scripts/common")
            .file("/r/scripts/common/a.py");
        fs.dir("/r/scripts/linux").file("/r/scripts/linux/b.sh");
        fs.dir("/r/scripts/linux/hypr")
            .file("/r/scripts/linux/hypr/c.sh");

        let root = AbsPath::new("/r");
        let (links, conflicts) = plan_scripts(&fs, &root, Os::Linux, &[]);
        let targets: Vec<_> = links
            .iter()
            .map(|l| l.target.as_path().to_owned())
            .collect();
        assert!(targets.contains(&Path::new("/r/.gen/scripts/a.py").to_owned()));
        assert!(targets.contains(&Path::new("/r/.gen/scripts/b.sh").to_owned()));
        // 子目录默认整链（保持树形），无需任何声明
        assert!(targets.contains(&Path::new("/r/.gen/scripts/hypr").to_owned()));
        assert!(!targets.contains(&Path::new("/r/.gen/scripts/c.sh").to_owned()));
        assert!(conflicts.is_empty());
    }

    #[test]
    fn keeps_tree_by_default() {
        let mut fs = MemFs::new();
        fs.dir("/r/scripts").dir("/r/scripts/common");
        fs.dir("/r/scripts/common/sub")
            .file("/r/scripts/common/sub/deep.sh");
        let root = AbsPath::new("/r");
        let (links, _) = plan_scripts(&fs, &root, Os::Linux, &[]);
        let targets: Vec<_> = links
            .iter()
            .map(|l| l.target.as_path().to_owned())
            .collect();
        // 默认保持目录形态：sub 整链、deep.sh 不提到顶层
        assert!(targets.contains(&Path::new("/r/.gen/scripts/sub").to_owned()));
        assert!(!targets.contains(&Path::new("/r/.gen/scripts/deep.sh").to_owned()));
    }

    #[test]
    fn flattens_ignore_tree_dir() {
        let mut fs = MemFs::new();
        fs.dir("/r/scripts").dir("/r/scripts/common");
        fs.dir("/r/scripts/common/sub")
            .file("/r/scripts/common/sub/deep.sh");
        let root = AbsPath::new("/r");
        let (links, _) = plan_scripts(&fs, &root, Os::Linux, &["sub".to_owned()]);
        let targets: Vec<_> = links
            .iter()
            .map(|l| l.target.as_path().to_owned())
            .collect();
        // ignore_tree 列出的子目录递归拍平：脚本提到顶层、目录本身不整链
        assert!(targets.contains(&Path::new("/r/.gen/scripts/deep.sh").to_owned()));
        assert!(!targets.contains(&Path::new("/r/.gen/scripts/sub").to_owned()));
    }

    #[test]
    fn detects_name_conflict() {
        let mut fs = MemFs::new();
        fs.dir("/r/scripts")
            .dir("/r/scripts/common")
            .file("/r/scripts/common/dup.sh");
        fs.dir("/r/scripts/linux").file("/r/scripts/linux/dup.sh");
        let root = AbsPath::new("/r");
        let (_links, conflicts) = plan_scripts(&fs, &root, Os::Linux, &[]);
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].name, "dup.sh");
        assert_eq!(conflicts[0].sources.len(), 2);
    }
}
