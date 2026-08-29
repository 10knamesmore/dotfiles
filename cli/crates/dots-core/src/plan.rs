//! 三方状态 reconciliation 产生的唯一执行计划。

use crate::{AppliedResource, ObservedState, OwnershipSurface, ResourceSpec};

/// 一次 sync 对所有 Resource 的完整、确定性计划。
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Plan {
    /// 按固定 apply phase 排序的计划项。
    pub items: Vec<PlanItem>,
}

impl Plan {
    /// 返回 Plan 是否不存在 Collision 或 Drift。
    pub fn is_healthy(&self) -> bool {
        self.items.iter().all(|item| {
            !matches!(
                item.action,
                PlanAction::Collision { .. } | PlanAction::Drift { .. }
            )
        })
    }

    /// 返回 Plan 是否完全不需要外部写入或 inventory 变化。
    pub fn is_clean(&self) -> bool {
        self.items
            .iter()
            .all(|item| matches!(item.action, PlanAction::Noop))
    }
}

/// 一个 Ownership Surface 的三方状态与判定结果。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanItem {
    /// 本项 Resource identity。
    pub surface: OwnershipSurface,

    /// 当前 Declaration；Retired Resource 为 `None`。
    pub desired: Option<ResourceSpec>,

    /// 上一次成功记录；首次接管为 `None`。
    pub applied: Option<AppliedResource>,

    /// planning 时读取到的真实状态。
    pub observed: ObservedState,

    /// executor 应采取的动作。
    pub action: PlanAction,
}

/// Planner 对一个 Resource 的判定。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlanAction {
    /// Desired、Applied 与 Observed 已一致。
    Noop,

    /// Observed 已与 Desired 一致，只需写 Applied Inventory。
    Adopt,

    /// Ownership Surface 尚不存在，创建 Desired Resource。
    Create,

    /// Observed 仍匹配 Applied，可以安全更新到 Desired。
    Update,

    /// Retired Resource 仍匹配 Applied，安全删除。
    Delete,

    /// Retired Resource 已不存在，只需从 Applied Inventory 移除。
    Forget,

    /// 尚未拥有的 surface 被不同状态占据。
    Collision {
        /// 面向用户的具体冲突原因。
        reason: String,
    },

    /// 已拥有的 surface 偏离上次成功状态。
    Drift {
        /// 面向用户的具体漂移原因。
        reason: String,
    },
}
