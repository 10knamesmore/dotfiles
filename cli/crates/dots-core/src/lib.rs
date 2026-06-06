//! dots-core —— dots 的纯逻辑层。
//!
//! 不依赖 mlua/minijinja/age/真实文件系统：只接收已构建好的 [`manifest::Manifest`]
//! 与 [`fs::FileSystem`] trait，计算出 [`plan::Plan`]。所有真实 IO 与外部库都在 `dots` bin。

pub mod fs;
pub mod layer;
pub mod manifest;
pub mod plan;
pub mod resolve;
pub mod scripts;
pub mod types;

pub use fs::{FileSystem, NodeKind};
pub use layer::{ExpectedLink, expand_layers};
pub use manifest::{
    ClosureId, DistributeSpec, GranularitySpec, HookPhase, HookReg, Manifest, RootSpec,
};
pub use plan::{Plan, PlanAction, PlanItem};
pub use resolve::resolve;
pub use scripts::{ScriptConflict, plan_scripts};
pub use types::{AbsPath, Layer, LinkMode, Os, RepoPath};

/// 通用 Result 别名（项目约定：统一 color-eyre）。
pub type Result<T> = color_eyre::Result<T>;
