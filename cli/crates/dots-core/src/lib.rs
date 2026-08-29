//! dots-core —— dots 的纯逻辑层。
//!
//! 不依赖 mlua、minijinja 或真实文件系统：只接收已构建好的 [`manifest::Manifest`]
//! 与 [`fs::FileSystem`] trait，计算出 [`plan::Plan`]。所有真实 IO 与外部库都在 `dots` bin。

pub mod fs;
pub mod layer;
pub mod manifest;
pub mod plan;
pub mod resolve;
pub mod resource;
pub mod scripts;
pub mod types;

pub use fs::{FileSystem, NodeKind};
pub use layer::{ExpectedLink, expand_layers};
pub use manifest::{
    CargoBinaryDeclaration, DistributeSpec, GranularitySpec, Manifest, ResourceDeclaration,
    RootSpec,
};
pub use plan::{Plan, PlanAction, PlanItem};
pub use resolve::{ObservedState, reconcile};
pub use resource::{
    AppliedResource, ManagedBlockPlacement, OwnershipSurface, ResourceSpec, ResourceState,
    content_sha256, normalize_managed_block_content,
};
pub use scripts::{ScriptConflict, plan_scripts};
pub use types::{AbsPath, Layer, LinkMode, Os, RepoPath};

/// 通用 Result 别名（项目约定：统一 color-eyre）。
pub type Result<T> = color_eyre::Result<T>;
