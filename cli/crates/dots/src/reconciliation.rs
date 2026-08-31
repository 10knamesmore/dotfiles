//! 把仓库配置编译为 Desired Set，并构造 sync/status 共用的三方 Plan。

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::cmd::{
    absolute_source, absolute_target, current_os, expand_home, find_repo_root, home_dir,
    load_manifest, os_str,
};
use crate::inject::{InjectCtx, render as render_template};
use crate::managed_block;
use crate::realfs::RealFs;
use crate::state::State;
use color_eyre::eyre::Context;
use dots_core::manifest::{DistributeSpec, Manifest, ResourceDeclaration};
use dots_core::{
    AbsPath, AppliedResource, ExpectedLink, FileSystem, Layer, LinkMode, ManagedBlockPlacement,
    NodeKind, ObservedState, Os, OwnershipSurface, Plan, ResourceSpec, ResourceState,
    content_sha256, expand_layers, plan_scripts, reconcile,
};

/// 一次 planning 所需的完整机器与仓库上下文。
pub struct PreparedReconciliation {
    /// 仓库根。
    pub repo_root: PathBuf,

    /// 当前 `$HOME`。
    pub home: PathBuf,

    /// 当前声明的 Manifest。
    pub manifest: Manifest,

    /// planning 前加载的 Applied Inventory。
    pub state: State,

    /// 唯一 Plan。
    pub plan: Plan,

    /// 旧整目录仓库 symlink 转为真实容器的前置动作。
    pub container_conversions: Vec<ContainerConversion>,

    /// scripts 聚合中发现的重名冲突。
    pub script_conflicts: Vec<String>,
}

/// 从 `dots.lua` 加载、但尚未读取机器状态的 sync 配置。
pub struct LoadedConfiguration {
    /// 仓库根。
    pub repo_root: PathBuf,

    /// 当前 `$HOME`。
    pub home: PathBuf,

    /// 当前运行平台。
    pub os: Os,

    /// 当前声明的 Manifest。
    pub manifest: Manifest,
}

/// 旧整目录 link 到逐子项 Resource 的结构迁移动作。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContainerConversion {
    /// 应从 symlink 改成真实目录的位置。
    pub target: PathBuf,

    /// 旧 symlink 指向的仓库目录，仅用于诊断。
    pub source: PathBuf,
}

/// 从当前 checkout、Applied Inventory 与真实机器状态构建 Plan。
pub fn prepare() -> crate::Result<PreparedReconciliation> {
    prepare_configuration(load_configuration()?)
}

/// 求值 `dots.lua`，供 sync 在 planning 前执行 lifecycle hook。
pub fn load_configuration() -> crate::Result<LoadedConfiguration> {
    let repo_root = find_repo_root()?;
    let home = home_dir()?;
    let os = current_os();
    let mut manifest = load_manifest(&repo_root, &home, os)?;
    for root in &mut manifest.roots {
        root.path = absolute_target(&root.path, &home)?.display().to_string();
    }

    Ok(LoadedConfiguration {
        repo_root,
        home,
        os,
        manifest,
    })
}

/// 在 lifecycle hook 完成后读取机器状态并生成 Resource Plan。
pub fn prepare_configuration(
    configuration: LoadedConfiguration,
) -> crate::Result<PreparedReconciliation> {
    let LoadedConfiguration {
        repo_root,
        home,
        os,
        manifest,
    } = configuration;

    let state = State::load(&repo_root)?;
    let fs = RealFs::new();
    let (desired, script_conflicts) = collect_desired(&repo_root, &home, os, &manifest, &fs)?;
    let conversions = find_container_conversions(&desired, &repo_root, &fs);
    let mut observed = observe_all(&desired, &state.resources)?;
    for conversion in &conversions {
        for surface in observed.keys().cloned().collect::<Vec<_>>() {
            if surface.filesystem_path().is_some_and(|path| {
                path.starts_with(&conversion.target) && path != conversion.target
            }) {
                observed.insert(surface, ObservedState::Missing);
            }
        }
    }
    let plan = reconcile(desired, &state.resources, &observed)?;
    Ok(PreparedReconciliation {
        repo_root,
        home,
        manifest,
        state,
        plan,
        container_conversions: conversions,
        script_conflicts,
    })
}

/// 将 mapping、内建结果和参与 sync lifecycle 的显式 API 编译为 Resource。
fn collect_desired(
    repo_root: &Path,
    home: &Path,
    os: Os,
    manifest: &Manifest,
    filesystem: &RealFs,
) -> crate::Result<(Vec<ResourceSpec>, Vec<String>)> {
    let repo = AbsPath::new(repo_root);
    let home_abs = AbsPath::new(home);
    let mut resources = Vec::new();

    let layer_links = expand_layers(filesystem, &repo, &home_abs, os, manifest)
        .into_iter()
        .filter(|link| {
            link.source
                .as_path()
                .extension()
                .is_none_or(|extension| extension != "inject")
        });
    resources.extend(layer_links.map(symlink_resource));

    let distribute_links = collect_distribute_links(filesystem, &repo, manifest, home);
    resources.extend(distribute_links.map(symlink_resource));

    let (script_links, conflicts) =
        plan_scripts(filesystem, &repo, os, &manifest.scripts_ignore_tree);
    resources.extend(script_links.into_iter().map(symlink_resource));

    resources.extend(collect_injected_resources(repo_root, home, os, manifest)?);
    resources.extend(shell_resources(repo_root, home));
    for declaration in &manifest.resources {
        resources.extend(compile_declaration(declaration, repo_root, home)?);
    }

    Ok((
        resources,
        conflicts
            .into_iter()
            .map(|conflict| format!("{}（{} 个来源）", conflict.name, conflict.sources.len()))
            .collect(),
    ))
}

/// 把 mapping engine 的 ExpectedLink 归一为 Symlink Resource。
fn symlink_resource(link: ExpectedLink) -> ResourceSpec {
    let source_is_dir = link.source.as_path().is_dir();
    ResourceSpec::Symlink {
        target: link.target,
        source: link.source,
        source_is_dir,
        origin: format!("{} mapping", link.via.name),
    }
}

/// 展开 distribute mapping，并在目标工具根不存在时保持该分发不进入 Desired Set。
fn collect_distribute_links(
    filesystem: &dyn FileSystem,
    repo: &AbsPath,
    manifest: &Manifest,
    home: &Path,
) -> impl Iterator<Item = ExpectedLink> {
    let mut links = Vec::new();
    let via = Layer {
        name: "distribute".to_owned(),
        os: None,
    };
    for spec in &manifest.distribute {
        let source = repo.join(spec.src.as_path());
        for target in &spec.to {
            let target = AbsPath::new(expand_home(target, home));
            if target
                .as_path()
                .parent()
                .is_some_and(|parent| !parent.exists())
            {
                continue;
            }
            push_distribute(filesystem, spec, &source, &target, &via, &mut links);
        }
    }
    links.into_iter()
}

/// 按 distribute 粒度展开一条 mapping。
fn push_distribute(
    filesystem: &dyn FileSystem,
    spec: &DistributeSpec,
    source: &AbsPath,
    target: &AbsPath,
    via: &Layer,
    links: &mut Vec<ExpectedLink>,
) {
    match spec.mode {
        LinkMode::Children => {
            for child in filesystem.read_dir(source.as_path()) {
                if let Some(name) = child.file_name() {
                    links.push(ExpectedLink {
                        target: AbsPath::new(target.as_path().join(name)),
                        source: AbsPath::new(child),
                        via: via.clone(),
                        shadowed: Vec::new(),
                    });
                }
            }
        }
        LinkMode::Dir | LinkMode::File => links.push(ExpectedLink {
            target: target.clone(),
            source: source.clone(),
            via: via.clone(),
            shadowed: Vec::new(),
        }),
    }
}

/// 编译一个参与 sync lifecycle 的显式 Lua Resource declaration。
fn compile_declaration(
    declaration: &ResourceDeclaration,
    repo_root: &Path,
    home: &Path,
) -> crate::Result<Vec<ResourceSpec>> {
    match declaration {
        ResourceDeclaration::Symlink { source, target } => {
            let source = absolute_source(source, repo_root, home)?;
            let target = absolute_target(target, home)?;
            Ok(vec![ResourceSpec::Symlink {
                source_is_dir: source.is_dir(),
                source: AbsPath::new(source),
                target: AbsPath::new(target),
                origin: "dots.resource.symlink".to_owned(),
            }])
        }
        ResourceDeclaration::CopiedFile { source, target } => {
            let source = absolute_source(source, repo_root, home)?;
            file_resource(
                &source,
                absolute_target(target, home)?,
                "dots.resource.copied_file",
            )
            .map(|resource| vec![resource])
        }
        ResourceDeclaration::ManagedBlock {
            target,
            marker,
            content,
        } => Ok(vec![ResourceSpec::ManagedBlock {
            target: AbsPath::new(absolute_target(target, home)?),
            marker: marker.clone(),
            content: content.clone(),
            placement: ManagedBlockPlacement::End,
            origin: "dots.resource.managed_block".to_owned(),
        }]),
        ResourceDeclaration::SystemdUserUnit { unit } => Ok(vec![ResourceSpec::SystemdUserUnit {
            unit: unit.clone(),
            origin: "dots.resource.systemd_user_unit".to_owned(),
        }]),
    }
}

/// 读取普通文件 payload 与 mode 并构造 File Resource。
fn file_resource(source: &Path, target: PathBuf, origin: &str) -> crate::Result<ResourceSpec> {
    let content = fs::read(source)
        .wrap_err_with(|| format!("读取 Resource source 失败：{}", source.display()))?;
    let mode = fs::metadata(source)
        .wrap_err_with(|| format!("读取 Resource mode 失败：{}", source.display()))?
        .permissions()
        .mode()
        & 0o777;
    Ok(ResourceSpec::File {
        target: AbsPath::new(target),
        content,
        mode,
        origin: origin.to_owned(),
    })
}

/// 把 `.inject` 模板编译为 generated file 与其落点 symlink。
fn collect_injected_resources(
    repo_root: &Path,
    home: &Path,
    os: Os,
    manifest: &Manifest,
) -> crate::Result<Vec<ResourceSpec>> {
    let context = InjectCtx {
        dotfiles: repo_root.display().to_string(),
        scripts: repo_root.join(".gen/scripts").display().to_string(),
    };
    let mut layers = vec![
        ("home".to_owned(), home.to_path_buf()),
        (format!("home.{}", os_str(os)), home.to_path_buf()),
    ];
    for root in &manifest.roots {
        if root
            .os
            .as_deref()
            .is_none_or(|required| required == os_str(os))
        {
            layers.push((root.name.clone(), PathBuf::from(&root.path)));
        }
    }
    let mut by_target: BTreeMap<PathBuf, (String, PathBuf, String)> = BTreeMap::new();
    for (layer, target_root) in layers {
        let layer_dir = repo_root.join("tree").join(&layer);
        for entry in walk_files(&layer_dir) {
            if entry
                .extension()
                .is_none_or(|extension| extension != "inject")
            {
                continue;
            }
            let template = fs::read_to_string(&entry)
                .wrap_err_with(|| format!("读取 inject 模板失败：{}", entry.display()))?;
            let rendered = render_template(&template, &context)?;
            let relative = entry
                .strip_prefix(&layer_dir)
                .wrap_err("计算 inject 相对路径失败")?;
            let stripped = relative.with_extension("");
            by_target.insert(
                target_root.join(&stripped),
                (layer.clone(), stripped, rendered),
            );
        }
    }

    let mut resources = Vec::new();
    for (target, (layer, relative, rendered)) in by_target {
        let generated = repo_root.join(".gen/injected").join(layer).join(relative);
        resources.push(ResourceSpec::File {
            target: AbsPath::new(&generated),
            content: rendered.into_bytes(),
            mode: 0o644,
            origin: ".inject generated file".to_owned(),
        });
        resources.push(ResourceSpec::Symlink {
            target: AbsPath::new(target),
            source: AbsPath::new(generated),
            source_is_dir: false,
            origin: ".inject target".to_owned(),
        });
    }
    Ok(resources)
}

/// 生成 `.zshrc` block、`env.zsh` 与 `root` 内建 Resource。
fn shell_resources(repo_root: &Path, home: &Path) -> Vec<ResourceSpec> {
    let dots_dir = home.join(".config/dots");
    let repo = repo_root.display();
    let environment = format!(
        "# 由 dots 生成。消灭模板变量：配置引用 $DOTFILES_DIR / $DOTS_SCRIPTS。\n\
         export DOTFILES_DIR=\"{repo}\"\n\
         export DOTS_SCRIPTS=\"$DOTFILES_DIR/.gen/scripts\"\n\
         path=($DOTS_SCRIPTS $path)\n"
    );
    vec![
        ResourceSpec::ManagedBlock {
            target: AbsPath::new(home.join(".zshrc")),
            marker: "dots-env".to_owned(),
            content: "source \"$HOME/.zshrc_dotfiles\"".to_owned(),
            placement: ManagedBlockPlacement::Start,
            origin: "built-in shell entrypoint".to_owned(),
        },
        ResourceSpec::File {
            target: AbsPath::new(dots_dir.join("env.zsh")),
            content: environment.into_bytes(),
            mode: 0o644,
            origin: "built-in shell environment".to_owned(),
        },
        ResourceSpec::File {
            target: AbsPath::new(dots_dir.join("root")),
            content: format!("{repo}\n").into_bytes(),
            mode: 0o644,
            origin: "built-in repository root".to_owned(),
        },
    ]
}

/// 读取 Desired/Applied union 的真实机器状态。
fn observe_all(
    desired: &[ResourceSpec],
    applied: &[AppliedResource],
) -> crate::Result<BTreeMap<OwnershipSurface, ObservedState>> {
    let surfaces: BTreeSet<OwnershipSurface> = desired
        .iter()
        .map(ResourceSpec::surface)
        .chain(applied.iter().map(|resource| resource.surface.clone()))
        .collect();
    let mut observed = BTreeMap::new();
    for surface in &surfaces {
        observed.insert(surface.clone(), observe_surface(surface));
    }
    mark_overlapping_blocks(&surfaces, &mut observed)?;
    Ok(observed)
}

/// 读取单个 Ownership Surface。
pub(crate) fn observe_surface(surface: &OwnershipSurface) -> ObservedState {
    match surface {
        OwnershipSurface::Path { path } => observe_path(path),
        OwnershipSurface::ManagedBlock { file, marker } => observe_block(file, marker),
        OwnershipSurface::SystemdUserUnit { unit } => observe_systemd(unit),
    }
}

/// 不跟随最终 symlink 地读取 path state。
fn observe_path(path: &Path) -> ObservedState {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return ObservedState::Missing;
        }
        Err(error) => {
            return ObservedState::Invalid {
                reason: format!("无法读取 {}：{error}", path.display()),
            };
        }
    };
    if metadata.file_type().is_symlink() {
        return match fs::read_link(path) {
            Ok(source) => ObservedState::Present(ResourceState::Symlink { source }),
            Err(error) => ObservedState::Invalid {
                reason: format!("无法 readlink {}：{error}", path.display()),
            },
        };
    }
    if metadata.file_type().is_file() {
        return match fs::read(path) {
            Ok(content) => ObservedState::Present(ResourceState::File {
                content_sha256: content_sha256(&content),
                mode: metadata.permissions().mode() & 0o777,
            }),
            Err(error) => ObservedState::Invalid {
                reason: format!("无法读取文件 {}：{error}", path.display()),
            },
        };
    }
    ObservedState::Invalid {
        reason: format!("{} 是真实目录或不支持的节点类型", path.display()),
    }
}

/// 读取 marker block；文件其他内容不属于该 surface。
fn observe_block(file: &Path, marker: &str) -> ObservedState {
    let metadata = match fs::symlink_metadata(file) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return ObservedState::Missing;
        }
        Err(error) => {
            return ObservedState::Invalid {
                reason: format!("无法读取 {}：{error}", file.display()),
            };
        }
    };
    if !metadata.file_type().is_file() {
        return ObservedState::Invalid {
            reason: format!("managed block 文件不是普通文件：{}", file.display()),
        };
    }
    let text = match fs::read_to_string(file) {
        Ok(text) => text,
        Err(error) => {
            return ObservedState::Invalid {
                reason: format!("managed block 文件不是有效文本 {}：{error}", file.display()),
            };
        }
    };
    match managed_block::locate(&text, marker) {
        Ok(None) => ObservedState::Missing,
        Ok(Some(block)) => ObservedState::Present(ResourceState::ManagedBlock {
            content_sha256: content_sha256(text[block.content_range].as_bytes()),
        }),
        Err(reason) => ObservedState::Invalid { reason },
    }
}

/// 查询 systemd user unit enabled 状态；命令无法启动时视为不可安全处理。
fn observe_systemd(unit: &str) -> ObservedState {
    match Command::new("systemctl")
        .args(["--user", "is-enabled", "--quiet", unit])
        .status()
    {
        Ok(status) => ObservedState::Present(ResourceState::SystemdUserUnit {
            enabled: status.success(),
        }),
        Err(error) => ObservedState::Invalid {
            reason: format!("无法查询 systemd user unit `{unit}`：{error}"),
        },
    }
}

/// 同一文件中结构完整但区间互相嵌套的不同 block 都标记为 Invalid。
fn mark_overlapping_blocks(
    surfaces: &BTreeSet<OwnershipSurface>,
    observed: &mut BTreeMap<OwnershipSurface, ObservedState>,
) -> crate::Result<()> {
    let mut by_file: BTreeMap<PathBuf, Vec<(OwnershipSurface, std::ops::Range<usize>)>> =
        BTreeMap::new();
    for surface in surfaces {
        let OwnershipSurface::ManagedBlock { file, marker } = surface else {
            continue;
        };
        let Ok(text) = fs::read_to_string(file) else {
            continue;
        };
        if let Ok(Some(block)) = managed_block::locate(&text, marker) {
            by_file
                .entry(file.clone())
                .or_default()
                .push((surface.clone(), block.block_range));
        }
    }
    for (file, blocks) in by_file {
        for (index, (left_surface, left_range)) in blocks.iter().enumerate() {
            for (right_surface, right_range) in blocks.iter().skip(index + 1) {
                if left_range.start < right_range.end && right_range.start < left_range.end {
                    let reason = format!("{} 中 managed block 区间重叠", file.display());
                    observed.insert(
                        left_surface.clone(),
                        ObservedState::Invalid {
                            reason: reason.clone(),
                        },
                    );
                    observed.insert(
                        right_surface.clone(),
                        ObservedState::Invalid {
                            reason: reason.clone(),
                        },
                    );
                }
            }
        }
    }
    Ok(())
}

/// 找出阻挡 Desired 子路径的旧仓库目录 symlink。
fn find_container_conversions(
    desired: &[ResourceSpec],
    repo_root: &Path,
    filesystem: &dyn FileSystem,
) -> Vec<ContainerConversion> {
    let mut by_target = BTreeMap::new();
    for resource in desired {
        let Some(path) = resource.surface().filesystem_path().map(Path::to_path_buf) else {
            continue;
        };
        let mut ancestors: Vec<PathBuf> = path.ancestors().skip(1).map(Path::to_path_buf).collect();
        ancestors.reverse();
        for ancestor in ancestors {
            if let NodeKind::Symlink { target } = filesystem.classify(&ancestor)
                && target.starts_with(repo_root)
            {
                by_target
                    .entry(ancestor.clone())
                    .or_insert(ContainerConversion {
                        target: ancestor,
                        source: target,
                    });
            }
        }
    }
    by_target.into_values().collect()
}

/// 递归列出目录下全部普通文件。
fn walk_files(directory: &Path) -> Vec<PathBuf> {
    walkdir::WalkDir::new(directory)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_file())
        .map(walkdir::DirEntry::into_path)
        .collect()
}
