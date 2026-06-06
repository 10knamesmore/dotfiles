//! tree 映射引擎（§3 规则 1/2/3 + 层覆盖）。
//!
//! 把 `tree/<layer>/...` 的目录结构展开成「期望链接集合」。粒度启发式：
//! 层根一级目录是容器（下钻），二级及更深目录整目录链，文件直接链；`granularity()`
//! 可覆盖任意路径的模式与 ignore。最后做层覆盖（平台层条目级盖通用层）。

use std::path::Path;

use crate::fs::{FileSystem, NodeKind};
use crate::manifest::Manifest;
use crate::types::{AbsPath, Layer, LinkMode, Os, RepoPath};

/// 一条期望链接：`target`（$HOME 侧）应是指向 `source`（仓库内）的符号链接。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExpectedLink {
    /// 目标位置。
    pub target: AbsPath,
    /// 仓库内源。
    pub source: AbsPath,
    /// 来自哪个层。
    pub via: Layer,
    /// 被本条遮蔽的、同目标的其他层源（供 status 展示）。
    pub shadowed: Vec<AbsPath>,
}

/// 展开所有生效层为期望链接集合。
///
/// # Params:
///   - `fs`: 文件系统（读 tree 结构）
///   - `repo_root`: 仓库根绝对路径
///   - `home`: `$HOME` 绝对路径
///   - `os`: 当前平台（决定 `home.<os>` 层是否生效）
///   - `manifest`: 提供 granularity / roots 覆盖
///
/// # Return:
///   合并层覆盖后的期望链接集合。
pub fn expand_layers(
    fs: &dyn FileSystem,
    repo_root: &AbsPath,
    home: &AbsPath,
    os: Os,
    manifest: &Manifest,
) -> Vec<ExpectedLink> {
    let tree = repo_root.join("tree");
    let mut raw: Vec<ExpectedLink> = Vec::new();

    let mut layer_dirs = fs.read_dir(tree.as_path());
    layer_dirs.sort();
    for layer_dir in layer_dirs {
        if fs.classify(&layer_dir) != NodeKind::Dir {
            continue;
        }
        let Some(name) = file_name_str(&layer_dir) else {
            continue;
        };
        let layer = Layer::parse(name);
        if !layer.active_on(os) {
            continue;
        }
        let target_root = target_root_for(&layer, home, manifest);
        walk_container(
            fs,
            repo_root,
            &layer,
            Path::new(""),
            &target_root,
            manifest,
            0,
            &mut raw,
        );
    }

    merge_layers(raw)
}

/// 决定某层的目标根：默认 `$HOME`；`root()` 声明的额外层用其 path。
fn target_root_for(layer: &Layer, home: &AbsPath, manifest: &Manifest) -> AbsPath {
    if let Some(root_spec) = manifest
        .roots
        .iter()
        .find(|root_spec| root_spec.name == layer.name)
    {
        // root.path 可能含 `~`；core 不展开 `~`，bin 在喂入前应已展开为绝对路径。
        return AbsPath::new(&root_spec.path);
    }
    home.clone()
}

/// 遍历一个容器目录，对子项按粒度启发式产出链接或继续下钻。
#[allow(clippy::too_many_arguments)]
fn walk_container(
    fs: &dyn FileSystem,
    repo_root: &AbsPath,
    layer: &Layer,
    rel: &Path,
    target_root: &AbsPath,
    manifest: &Manifest,
    depth: usize,
    out: &mut Vec<ExpectedLink>,
) {
    let container_abs = repo_root.join("tree").join(&layer.name).join(rel);
    let ignore = granularity_for(layer, rel, manifest)
        .map(|granularity| granularity.ignore.clone())
        .unwrap_or_default();

    let mut children = fs.read_dir(container_abs.as_path());
    children.sort();
    for child in children {
        let Some(cname) = file_name_str(&child) else {
            continue;
        };
        if ignore.iter().any(|item| item == cname) {
            continue;
        }
        let child_rel = rel.join(cname);
        let child_abs = AbsPath::new(&child);
        let target = target_root.join(&child_rel);

        match fs.classify(&child) {
            // 仓库内源是软链（如 kitty basic.conf）也当文件链过去。
            NodeKind::File | NodeKind::Symlink { .. } => {
                out.push(link(target, child_abs, layer));
            }
            NodeKind::Dir => {
                let mode = granularity_for(layer, &child_rel, manifest)
                    .map(|granularity| granularity.mode)
                    .unwrap_or(if depth == 0 {
                        LinkMode::Children
                    } else {
                        LinkMode::Dir
                    });
                match mode {
                    LinkMode::Dir => out.push(link(target, child_abs, layer)),
                    LinkMode::Children => walk_container(
                        fs,
                        repo_root,
                        layer,
                        &child_rel,
                        target_root,
                        manifest,
                        depth + 1,
                        out,
                    ),
                    LinkMode::File => {
                        walk_files(fs, &child_abs, &target, manifest, layer, &child_rel, out)
                    }
                }
            }
            NodeKind::Missing => {}
        }
    }
}

/// `file` 模式：递归把目录下所有文件逐个链到对应位置（建中间目录由 executor 负责）。
#[allow(clippy::too_many_arguments)]
fn walk_files(
    fs: &dyn FileSystem,
    src_dir: &AbsPath,
    target_dir: &AbsPath,
    manifest: &Manifest,
    layer: &Layer,
    rel: &Path,
    out: &mut Vec<ExpectedLink>,
) {
    let ignore = granularity_for(layer, rel, manifest)
        .map(|granularity| granularity.ignore.clone())
        .unwrap_or_default();
    let mut children = fs.read_dir(src_dir.as_path());
    children.sort();
    for child in children {
        let Some(cname) = file_name_str(&child) else {
            continue;
        };
        if ignore.iter().any(|item| item == cname) {
            continue;
        }
        let child_abs = AbsPath::new(&child);
        let target = target_dir.join(cname);
        match fs.classify(&child) {
            NodeKind::File | NodeKind::Symlink { .. } => out.push(link(target, child_abs, layer)),
            NodeKind::Dir => {
                let child_rel = rel.join(cname);
                walk_files(fs, &child_abs, &target, manifest, layer, &child_rel, out);
            }
            NodeKind::Missing => {}
        }
    }
}

/// 查某层内相对路径 `rel` 的 granularity 规格。
fn granularity_for<'mf>(
    layer: &Layer,
    rel: &Path,
    manifest: &'mf Manifest,
) -> Option<&'mf crate::manifest::GranularitySpec> {
    let key = if rel.as_os_str().is_empty() {
        layer.name.clone()
    } else {
        format!("{}/{}", layer.name, rel.to_string_lossy())
    };
    manifest.granularity.get(&RepoPath::new(key))
}

/// 构造一条无遮蔽的 `ExpectedLink`。
fn link(target: AbsPath, source: AbsPath, via: &Layer) -> ExpectedLink {
    ExpectedLink {
        target,
        source,
        via: via.clone(),
        shadowed: Vec::new(),
    }
}

/// 层覆盖：同 target 时平台层（`os=Some`）盖通用层（`os=None`），被盖者记入 shadowed。
fn merge_layers(raw: Vec<ExpectedLink>) -> Vec<ExpectedLink> {
    use rustc_hash::FxHashMap;
    let mut by_target: FxHashMap<AbsPath, ExpectedLink> = FxHashMap::default();
    for link in raw {
        match by_target.get_mut(&link.target) {
            None => {
                by_target.insert(link.target.clone(), link);
            }
            Some(existing) => {
                // 平台层优先；同优先级保留先到者，另一方记 shadowed。
                let incoming_platform = link.via.os.is_some();
                let existing_platform = existing.via.os.is_some();
                if incoming_platform && !existing_platform {
                    let mut winner = link;
                    winner.shadowed.push(existing.source.clone());
                    winner.shadowed.append(&mut existing.shadowed);
                    *existing = winner;
                } else {
                    existing.shadowed.push(link.source);
                }
            }
        }
    }
    let mut out: Vec<ExpectedLink> = by_target.into_values().collect();
    out.sort_by(|left, right| left.target.cmp(&right.target));
    out
}

/// 取路径最后一段作 `&str`。
fn file_name_str(path: &Path) -> Option<&str> {
    path.file_name().and_then(|name| name.to_str())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::min_ident_chars, clippy::missing_docs_in_private_items)]
    use super::*;
    use crate::fs::MemFs;
    use crate::manifest::{GranularitySpec, Manifest};
    use std::path::PathBuf;

    /// 造一个仓库：repo_root=/r，tree 下若干层。
    fn repo() -> (MemFs, AbsPath, AbsPath) {
        (MemFs::new(), AbsPath::new("/r"), AbsPath::new("/home/u"))
    }

    #[test]
    fn rule1_file_and_platform_layer() {
        let (mut fs, root, home) = repo();
        fs.dir("/r/tree")
            .dir("/r/tree/home")
            .file("/r/tree/home/.vimrc");
        fs.dir("/r/tree/home.linux")
            .file("/r/tree/home.linux/.zshrc_linux");
        fs.dir("/r/tree/home.macos")
            .file("/r/tree/home.macos/.zshrc_macos");

        let links = expand_layers(&fs, &root, &home, Os::Linux, &Manifest::default());
        let targets: Vec<_> = links
            .iter()
            .map(|l| l.target.as_path().to_owned())
            .collect();
        assert!(targets.contains(&PathBuf::from("/home/u/.vimrc")));
        assert!(targets.contains(&PathBuf::from("/home/u/.zshrc_linux")));
        // macos 层在 Linux 上不生效
        assert!(!targets.contains(&PathBuf::from("/home/u/.zshrc_macos")));
    }

    #[test]
    fn rule2_container_descends_and_deep_dir_integral() {
        let (mut fs, root, home) = repo();
        fs.dir("/r/tree").dir("/r/tree/home");
        // .config 是层根一级目录=容器；nvim 是二级目录=整链
        fs.dir("/r/tree/home/.config");
        fs.dir("/r/tree/home/.config/nvim")
            .file("/r/tree/home/.config/nvim/init.lua");
        fs.file("/r/tree/home/.config/starship.toml");

        let links = expand_layers(&fs, &root, &home, Os::Linux, &Manifest::default());
        let targets: Vec<_> = links
            .iter()
            .map(|l| l.target.as_path().to_owned())
            .collect();
        // nvim 整目录链（不下钻到 init.lua）
        assert!(targets.contains(&PathBuf::from("/home/u/.config/nvim")));
        assert!(!targets.contains(&PathBuf::from("/home/u/.config/nvim/init.lua")));
        // starship.toml 文件直接链
        assert!(targets.contains(&PathBuf::from("/home/u/.config/starship.toml")));
        // .config 容器自身不成链
        assert!(!targets.contains(&PathBuf::from("/home/u/.config")));
    }

    #[test]
    fn rule3_granularity_file_with_ignore() {
        let (mut fs, root, home) = repo();
        fs.dir("/r/tree")
            .dir("/r/tree/home")
            .dir("/r/tree/home/.config");
        fs.dir("/r/tree/home/.config/opencode");
        fs.file("/r/tree/home/.config/opencode/opencode.json");
        fs.dir("/r/tree/home/.config/opencode/node_modules");
        fs.file("/r/tree/home/.config/opencode/node_modules/junk.js");

        let mut m = Manifest::default();
        m.granularity.insert(
            RepoPath::new("home/.config/opencode"),
            GranularitySpec {
                mode: LinkMode::File,
                ignore: vec!["node_modules".into()],
            },
        );
        let links = expand_layers(&fs, &root, &home, Os::Linux, &m);
        let targets: Vec<_> = links
            .iter()
            .map(|l| l.target.as_path().to_owned())
            .collect();
        // file 模式逐文件链
        assert!(targets.contains(&PathBuf::from("/home/u/.config/opencode/opencode.json")));
        // ignore 的 node_modules 不链
        assert!(
            !targets
                .iter()
                .any(|t| t.starts_with("/home/u/.config/opencode/node_modules"))
        );
    }

    #[test]
    fn rule3_children_container_with_ignore() {
        let (mut fs, root, home) = repo();
        fs.dir("/r/tree")
            .dir("/r/tree/home")
            .dir("/r/tree/home/.claude");
        fs.dir("/r/tree/home/.claude/hooks")
            .file("/r/tree/home/.claude/hooks/x.sh");
        fs.file("/r/tree/home/.claude/settings.json");
        fs.dir("/r/tree/home/.claude/projects"); // 运行时垃圾，应被 ignore

        let mut m = Manifest::default();
        m.granularity.insert(
            RepoPath::new("home/.claude"),
            GranularitySpec {
                mode: LinkMode::Children,
                ignore: vec!["projects".into()],
            },
        );
        let links = expand_layers(&fs, &root, &home, Os::Linux, &m);
        let targets: Vec<_> = links
            .iter()
            .map(|l| l.target.as_path().to_owned())
            .collect();
        // hooks 二级目录整链；settings.json 文件链
        assert!(targets.contains(&PathBuf::from("/home/u/.claude/hooks")));
        assert!(targets.contains(&PathBuf::from("/home/u/.claude/settings.json")));
        // projects 被 ignore
        assert!(!targets.contains(&PathBuf::from("/home/u/.claude/projects")));
    }

    #[test]
    fn layer_override_records_shadowed() {
        let (mut fs, root, home) = repo();
        fs.dir("/r/tree")
            .dir("/r/tree/home")
            .dir("/r/tree/home.linux");
        // 同目标 ~/.config/kitty：通用层与平台层都提供
        fs.dir("/r/tree/home/.config")
            .dir("/r/tree/home/.config/kitty");
        fs.file("/r/tree/home/.config/kitty/x");
        fs.dir("/r/tree/home.linux/.config")
            .dir("/r/tree/home.linux/.config/kitty");
        fs.file("/r/tree/home.linux/.config/kitty/x");

        let links = expand_layers(&fs, &root, &home, Os::Linux, &Manifest::default());
        let kitty = links
            .iter()
            .find(|l| l.target.as_path() == Path::new("/home/u/.config/kitty"));
        assert!(kitty.is_some(), "应有 kitty 链接");
        if let Some(kitty) = kitty {
            // 平台层胜出
            assert_eq!(kitty.via.os, Some(Os::Linux));
            // 通用层被记为 shadowed
            assert!(
                kitty
                    .shadowed
                    .iter()
                    .any(|s| s.as_path() == Path::new("/r/tree/home/.config/kitty"))
            );
        }
    }
}
