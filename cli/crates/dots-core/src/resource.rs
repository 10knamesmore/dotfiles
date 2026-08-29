//! 声明式 Resource、ownership 与跨次 sync 的状态快照。

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::AbsPath;

/// 一个 Resource 独占管理的最小机器状态位置。
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OwnershipSurface {
    /// 一个完整 filesystem path。
    Path {
        /// 展开并规范化后的绝对目标路径。
        path: PathBuf,
    },

    /// 文本文件内由 marker 标识的区间。
    ManagedBlock {
        /// 包含 block 的绝对文件路径。
        file: PathBuf,

        /// 在同一文件内稳定标识 block 的名称。
        marker: String,
    },

    /// 当前用户的一个 systemd unit enabled 状态。
    SystemdUserUnit {
        /// unit 名称，如 `mihomo.service`。
        unit: String,
    },
}

impl OwnershipSurface {
    /// 返回适合 CLI 输出和定向操作的稳定 selector。
    pub fn selector(&self) -> String {
        match self {
            Self::Path { path } => format!("path:{}", path.display()),
            Self::ManagedBlock { file, marker } => {
                format!("block:{}#{marker}", file.display())
            }
            Self::SystemdUserUnit { unit } => format!("systemd-user:{unit}"),
        }
    }

    /// 返回该 surface 所属 filesystem path；systemd unit 没有路径。
    pub fn filesystem_path(&self) -> Option<&Path> {
        match self {
            Self::Path { path } => Some(path),
            Self::ManagedBlock { file, .. } => Some(file),
            Self::SystemdUserUnit { .. } => None,
        }
    }
}

/// Resource 在某次成功 apply 后用于 Drift 判断的精确状态。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ResourceState {
    /// 符号链接及其原始 link target。
    Symlink {
        /// `readlink` 应返回的路径。
        source: PathBuf,
    },

    /// 普通文件的内容与权限状态。
    File {
        /// 文件内容的 SHA-256。
        content_sha256: String,

        /// dots 管理的 Unix permission bits。
        mode: u32,
    },

    /// marker 结构完整的 managed block 内容。
    ManagedBlock {
        /// 两条 marker 之间内容的 SHA-256。
        content_sha256: String,
    },

    /// systemd user unit 的 enabled 状态。
    SystemdUserUnit {
        /// unit 当前是否 enabled。
        enabled: bool,
    },
}

/// 上一次成功由 dots 拥有的一项 Resource。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppliedResource {
    /// Resource 的稳定 identity 与 ownership 边界。
    pub surface: OwnershipSurface,

    /// 上一次成功 apply 后预期观测到的状态。
    pub state: ResourceState,
}

/// Planner 本次希望存在的一项 Resource，包含 executor 所需 payload。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResourceSpec {
    /// 符号链接。
    Symlink {
        /// 链接位置。
        target: AbsPath,

        /// 链接应指向的路径。
        source: AbsPath,

        /// source 是目录时，该链接同时拥有 target 的全部后代路径。
        source_is_dir: bool,

        /// 产生这项声明的人类可读来源。
        origin: String,
    },

    /// 原子安装的普通文件；copy、generated file 与 cargo binary 都归一为此类型。
    File {
        /// 安装位置。
        target: AbsPath,

        /// 本次应写入的完整内容。
        content: Vec<u8>,

        /// 应写入的 Unix permission bits。
        mode: u32,

        /// 产生这项声明的人类可读来源。
        origin: String,
    },

    /// 文本文件内的 marker block。
    ManagedBlock {
        /// 包含 block 的文件。
        target: AbsPath,

        /// block 的稳定名称。
        marker: String,

        /// 两条 marker 之间的期望内容，不含 marker 本身。
        content: String,

        /// 新建 block 时放在文件开头还是结尾。
        placement: ManagedBlockPlacement,

        /// 产生这项声明的人类可读来源。
        origin: String,
    },

    /// 应保持 enabled 的 systemd user unit。
    SystemdUserUnit {
        /// unit 名称。
        unit: String,

        /// 产生这项声明的人类可读来源。
        origin: String,
    },
}

/// managed block 首次写入文件时的位置；后续更新保持原区间位置。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ManagedBlockPlacement {
    /// 放在文件开头。
    Start,

    /// 放在文件结尾。
    End,
}

impl ResourceSpec {
    /// 返回 Resource 的稳定 ownership identity。
    pub fn surface(&self) -> OwnershipSurface {
        match self {
            Self::Symlink { target, .. } | Self::File { target, .. } => OwnershipSurface::Path {
                path: target.as_path().to_owned(),
            },
            Self::ManagedBlock { target, marker, .. } => OwnershipSurface::ManagedBlock {
                file: target.as_path().to_owned(),
                marker: marker.clone(),
            },
            Self::SystemdUserUnit { unit, .. } => {
                OwnershipSurface::SystemdUserUnit { unit: unit.clone() }
            }
        }
    }

    /// 返回 apply 成功后应写入 Applied Inventory 的状态。
    pub fn desired_state(&self) -> ResourceState {
        match self {
            Self::Symlink { source, .. } => ResourceState::Symlink {
                source: source.as_path().to_owned(),
            },
            Self::File { content, mode, .. } => ResourceState::File {
                content_sha256: content_sha256(content),
                mode: *mode,
            },
            Self::ManagedBlock { content, .. } => ResourceState::ManagedBlock {
                content_sha256: content_sha256(normalize_managed_block_content(content).as_bytes()),
            },
            Self::SystemdUserUnit { .. } => ResourceState::SystemdUserUnit { enabled: true },
        }
    }

    /// 返回声明来源，用于 ownership conflict 诊断。
    pub fn origin(&self) -> &str {
        match self {
            Self::Symlink { origin, .. }
            | Self::File { origin, .. }
            | Self::ManagedBlock { origin, .. }
            | Self::SystemdUserUnit { origin, .. } => origin,
        }
    }

    /// 返回 path surface 是否覆盖全部后代。
    pub fn owns_descendants(&self) -> bool {
        matches!(
            self,
            Self::Symlink {
                source_is_dir: true,
                ..
            }
        )
    }
}

/// 计算 Applied Inventory 使用的稳定内容摘要。
pub fn content_sha256(content: &[u8]) -> String {
    format!("{:x}", Sha256::digest(content))
}

/// 统一 managed block body：保留原内容，只把结尾规范为一个换行。
pub fn normalize_managed_block_content(content: &str) -> String {
    format!("{}\n", content.trim_end_matches(['\r', '\n']))
}
