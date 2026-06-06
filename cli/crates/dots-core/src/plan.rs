//! Plan / PlanItem / PlanAction —— plan/execute 两段式的核心产物。
//!
//! `Plan` 是纯数据：dry-run 打印它、单测断言它、executor 落盘它。

use crate::types::AbsPath;

/// 一次 sync 计算出的完整计划。
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Plan {
    /// 逐目标位置的动作。
    pub items: Vec<PlanItem>,
}

impl Plan {
    /// 空计划。
    pub fn new() -> Self {
        Self::default()
    }

    /// 追加一项。
    pub fn push(&mut self, item: PlanItem) {
        self.items.push(item);
    }

    /// 是否没有任何需要落盘的动作（全 Noop）。
    pub fn is_clean(&self) -> bool {
        self.items
            .iter()
            .all(|item| matches!(item.action, PlanAction::Noop))
    }
}

/// 单个目标位置的计划项。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanItem {
    /// 目标位置（`$HOME` 侧）。
    pub target: AbsPath,
    /// 对该目标采取的动作。
    pub action: PlanAction,
}

/// 对一个目标位置的动作（§3.1 判定结果）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlanAction {
    /// 目标不存在 → 建新链。
    Link {
        /// 链接指向的仓库内源。
        source: AbsPath,
    },
    /// 目标是落在仓库内的旧链（含断链）→ 无条件重建。
    Relink {
        /// 新源。
        source: AbsPath,
        /// 旧链原指向（诊断用）。
        old_target: std::path::PathBuf,
    },
    /// 目标是真实文件/目录 → 备份后建链。
    BackupThenLink {
        /// 链接指向的仓库内源。
        source: AbsPath,
    },
    /// 期望是容器（children），但现状是整目录软链 → 转换为真实目录 + 逐子项链。
    ContainerConvert {
        /// 原整目录软链指向的源目录。
        source_dir: AbsPath,
    },
    /// 目标是符号链接但指向仓库外 → 报漂移，不擅自动。
    DriftForeign {
        /// 指向的外部路径。
        points_to: std::path::PathBuf,
    },
    /// 目标已正确指向期望源 → 无操作。
    Noop,
}

#[cfg(test)]
mod tests {
    #![allow(clippy::min_ident_chars, clippy::missing_docs_in_private_items)]
    use super::*;

    #[test]
    fn clean_plan() {
        let mut p = Plan::new();
        assert!(p.is_clean());
        p.push(PlanItem {
            target: AbsPath::new("/a"),
            action: PlanAction::Noop,
        });
        assert!(p.is_clean());
        p.push(PlanItem {
            target: AbsPath::new("/b"),
            action: PlanAction::Link {
                source: AbsPath::new("/repo/b"),
            },
        });
        assert!(!p.is_clean());
    }
}
