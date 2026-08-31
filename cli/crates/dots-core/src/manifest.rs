//! Manifest —— dots.lua 求值结果的纯数据表示。
//!
//! core 拥有这份数据结构但**不依赖 mlua**：bin 的 Lua 求值器把 Lua table 翻译成
//! `Manifest`。sync 消费 mapping 与 Resource，install 消费 Cargo binary declaration。

use rustc_hash::FxHashMap;

use crate::types::{LinkMode, RepoPath};

/// dots.lua 的全部声明。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Manifest {
    /// 链接粒度覆盖：相对 tree 的路径 → 粒度规格。
    pub granularity: FxHashMap<RepoPath, GranularitySpec>,

    /// 多目标分发。
    pub distribute: Vec<DistributeSpec>,

    /// 非 `$HOME` 镜像的额外层。
    pub roots: Vec<RootSpec>,

    /// scripts 聚合时不保树形、递归拍平的子目录名（子目录默认整目录链）。
    pub scripts_ignore_tree: Vec<String>,

    /// 由 `dots sync` 收敛的显式 Resource declaration。
    pub resources: Vec<ResourceDeclaration>,

    /// 绑定 dots command 生命周期的 hook。
    pub hooks: LifecycleHooks,

    /// 仅由 `dots install` 执行的 Cargo binary declaration。
    pub cargo_binaries: Vec<CargoBinaryDeclaration>,
}

/// dots command 暴露的 lifecycle hook 集合。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LifecycleHooks {
    /// 在真实机器状态读取和 Resource planning 前运行的 hook。
    pub before_sync: Vec<BeforeSyncHook>,
}

/// 在 `dots sync` planning 前运行的一条具名命令。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BeforeSyncHook {
    /// 日志和失败诊断使用的人类可读名称。
    pub name: String,

    /// 启动程序时使用的工作目录。
    pub cwd: String,

    /// 由当前 `PATH` 或明确路径解析的程序。
    pub program: String,

    /// 按原样传给程序的参数。
    pub args: Vec<String>,
}

/// 链接粒度规格。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GranularitySpec {
    /// 粒度模式。
    pub mode: LinkMode,
    /// 链接时跳过的子项名（如 `node_modules`）。
    pub ignore: Vec<String>,
}

/// 多目标分发规格。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DistributeSpec {
    /// 分发组名（如 `skills`）。
    pub name: String,
    /// 唯一真相源（仓库内）。
    pub src: RepoPath,
    /// 落点列表（`$HOME` 侧绝对/`~` 路径，bin 负责展开 `~`）。
    pub to: Vec<String>,
    /// 落点处的链接粒度（`children` 逐项 / `dir` 整目录）。
    pub mode: LinkMode,
}

/// 非 `$HOME` 镜像的额外层（罕见，如 macOS App Support）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RootSpec {
    /// 层名（对应 `tree/<name>`）。
    pub name: String,
    /// 目标根（`$HOME` 外的绝对/`~` 路径）。
    pub path: String,
    /// 仅在该平台生效；`None` 为全平台。
    pub os: Option<String>,
}

/// Lua 显式声明的一项 sync Resource；路径仍保持面向配置的字符串形式。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResourceDeclaration {
    /// 符号链接。
    Symlink {
        /// source；相对路径以仓库根为基准。
        source: String,

        /// 目标绝对路径或 `~` 路径。
        target: String,
    },

    /// 内容复制安装的普通文件。
    CopiedFile {
        /// source；相对路径以仓库根为基准。
        source: String,

        /// 目标绝对路径或 `~` 路径。
        target: String,
    },

    /// 文本文件内的 marker block。
    ManagedBlock {
        /// 包含 block 的文件。
        target: String,

        /// block 的稳定 marker 名称。
        marker: String,

        /// marker 之间的期望内容。
        content: String,
    },

    /// 应保持 enabled 的 systemd user unit。
    SystemdUserUnit {
        /// unit 名称。
        unit: String,
    },
}

/// 一项只由 `dots install` 执行的 Cargo binary declaration。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CargoBinaryDeclaration {
    /// 通过 Cargo 安装的仓库内 workspace binary。
    Workspace {
        /// 传给 `cargo install --path` 的 package directory；相对路径以仓库根为基准。
        path: String,

        /// 传给 `cargo install --bin` 的 binary target 名称。
        binary: String,

        /// 传给 `cargo install --root` 的安装根目录。
        root: String,
    },

    /// crates.io package 的默认版本与全部 bin。
    CratesIo {
        /// crates.io package 名称。
        package: String,

        /// 依次传给 `cargo install --bin` 的可选 binary target 名称。
        binaries: Vec<String>,
    },
}

#[cfg(test)]
mod tests {
    #![allow(clippy::min_ident_chars, clippy::missing_docs_in_private_items)]
    use super::*;

    #[test]
    fn manifest_default_empty() {
        let m = Manifest::default();
        assert!(m.distribute.is_empty());
        assert!(m.resources.is_empty());
        assert!(m.cargo_binaries.is_empty());
        assert!(m.granularity.is_empty());
    }
}
