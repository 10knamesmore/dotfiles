//! Desired、Applied 与 Observed 三方 reconciliation。

use std::collections::{BTreeMap, BTreeSet};

use color_eyre::eyre::eyre;

use crate::plan::{Plan, PlanAction, PlanItem};
use crate::{AppliedResource, OwnershipSurface, ResourceSpec, ResourceState};

/// 一个 Ownership Surface 当前真实状态。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ObservedState {
    /// 该 surface 当前不存在。
    Missing,

    /// 该 surface 存在且结构可以精确表示。
    Present(ResourceState),

    /// 实际对象存在，但类型或 marker 结构无法安全处理。
    Invalid {
        /// 面向用户的具体原因。
        reason: String,
    },
}

/// 依据 Desired Set、Applied Inventory 与 Observed State 生成完整 Plan。
///
/// # Error:
///   Desired Set 存在重复或重叠 ownership 时，在产生任何可执行 Plan 前返回错误。
pub fn reconcile(
    desired: Vec<ResourceSpec>,
    applied: &[AppliedResource],
    observed: &BTreeMap<OwnershipSurface, ObservedState>,
) -> crate::Result<Plan> {
    validate_ownership(&desired)?;

    let desired_by_surface: BTreeMap<OwnershipSurface, ResourceSpec> = desired
        .into_iter()
        .map(|resource| (resource.surface(), resource))
        .collect();
    let applied_by_surface: BTreeMap<OwnershipSurface, AppliedResource> = applied
        .iter()
        .cloned()
        .map(|resource| (resource.surface.clone(), resource))
        .collect();
    let surfaces: BTreeSet<OwnershipSurface> = desired_by_surface
        .keys()
        .chain(applied_by_surface.keys())
        .cloned()
        .collect();

    let mut items = Vec::with_capacity(surfaces.len());
    for surface in surfaces {
        let desired_resource = desired_by_surface.get(&surface).cloned();
        let applied_resource = applied_by_surface.get(&surface).cloned();
        let observed_state = observed
            .get(&surface)
            .cloned()
            .unwrap_or(ObservedState::Missing);
        let action = classify(
            &surface,
            desired_resource.as_ref(),
            applied_resource.as_ref(),
            &observed_state,
        );
        items.push(PlanItem {
            surface,
            desired: desired_resource,
            applied: applied_resource,
            observed: observed_state,
            action,
        });
    }
    items.sort_by_key(apply_phase);
    Ok(Plan { items })
}

/// 对单个 surface 执行三方状态分类。
fn classify(
    surface: &OwnershipSurface,
    desired: Option<&ResourceSpec>,
    applied: Option<&AppliedResource>,
    observed: &ObservedState,
) -> PlanAction {
    match (desired, applied) {
        (Some(desired_resource), None) => {
            let desired_state = desired_resource.desired_state();
            match observed {
                ObservedState::Missing => PlanAction::Create,
                ObservedState::Present(actual) if actual == &desired_state => PlanAction::Adopt,
                ObservedState::Present(actual) => PlanAction::Collision {
                    reason: format!("实际状态 {actual:?} 与声明不一致"),
                },
                ObservedState::Invalid { reason } => PlanAction::Collision {
                    reason: reason.clone(),
                },
            }
        }
        (Some(desired_resource), Some(applied_resource)) => {
            let desired_state = desired_resource.desired_state();
            match observed {
                ObservedState::Present(actual) if actual == &desired_state => {
                    if applied_resource.state == desired_state {
                        PlanAction::Noop
                    } else {
                        PlanAction::Adopt
                    }
                }
                ObservedState::Present(actual) if actual == &applied_resource.state => {
                    PlanAction::Update
                }
                ObservedState::Missing if matches!(surface, OwnershipSurface::Path { .. }) => {
                    PlanAction::Create
                }
                ObservedState::Missing => PlanAction::Drift {
                    reason: "上次应用的局部或外部状态已消失".to_owned(),
                },
                ObservedState::Present(actual) => PlanAction::Drift {
                    reason: format!("实际状态 {actual:?} 已偏离上次应用状态"),
                },
                ObservedState::Invalid { reason } => PlanAction::Drift {
                    reason: reason.clone(),
                },
            }
        }
        (None, Some(applied_resource)) => match observed {
            ObservedState::Missing => PlanAction::Forget,
            ObservedState::Present(actual) if actual == &applied_resource.state => {
                PlanAction::Delete
            }
            ObservedState::Present(actual) => PlanAction::Drift {
                reason: format!("Retired Resource 的实际状态 {actual:?} 已被修改"),
            },
            ObservedState::Invalid { reason } => PlanAction::Drift {
                reason: reason.clone(),
            },
        },
        (None, None) => PlanAction::Noop,
    }
}

/// 在 planner 进入状态分类前拒绝所有重叠 Desired ownership。
fn validate_ownership(resources: &[ResourceSpec]) -> crate::Result<()> {
    for resource in resources {
        validate_resource(resource)?;
    }
    for (index, left) in resources.iter().enumerate() {
        for right in resources.iter().skip(index + 1) {
            if ownership_overlaps(left, right) {
                return Err(eyre!(
                    "Resource ownership 冲突：{}（{}）与 {}（{}）",
                    left.surface().selector(),
                    left.origin(),
                    right.surface().selector(),
                    right.origin()
                ));
            }
        }
    }
    Ok(())
}

/// 拒绝无法形成稳定 ownership 或会产生自引用的单项声明。
fn validate_resource(resource: &ResourceSpec) -> crate::Result<()> {
    match resource {
        ResourceSpec::Symlink { target, source, .. } if target.as_path() == source.as_path() => {
            Err(eyre!(
                "symlink source 与 target 相同：{}",
                target.as_path().display()
            ))
        }
        ResourceSpec::ManagedBlock { marker, .. }
            if marker.is_empty() || marker.contains(['\r', '\n']) =>
        {
            Err(eyre!("managed block marker 必须是非空单行文本"))
        }
        ResourceSpec::SystemdUserUnit { unit, .. } if unit.trim().is_empty() => {
            Err(eyre!("systemd user unit 名称不能为空"))
        }
        _ => Ok(()),
    }
}

/// 判断两项 Desired Resource 是否管理同一实际对象部分。
fn ownership_overlaps(left: &ResourceSpec, right: &ResourceSpec) -> bool {
    let left_surface = left.surface();
    let right_surface = right.surface();
    match (&left_surface, &right_surface) {
        (
            OwnershipSurface::Path { path: left_path },
            OwnershipSurface::Path { path: right_path },
        ) => {
            left_path == right_path
                || (left.owns_descendants() && right_path.starts_with(left_path))
                || (right.owns_descendants() && left_path.starts_with(right_path))
        }
        (OwnershipSurface::Path { path }, OwnershipSurface::ManagedBlock { file, .. }) => {
            path == file || (left.owns_descendants() && file.starts_with(path))
        }
        (OwnershipSurface::ManagedBlock { file, .. }, OwnershipSurface::Path { path }) => {
            path == file || (right.owns_descendants() && file.starts_with(path))
        }
        (
            OwnershipSurface::ManagedBlock {
                file: left_file,
                marker: left_marker,
            },
            OwnershipSurface::ManagedBlock {
                file: right_file,
                marker: right_marker,
            },
        ) => left_file == right_file && left_marker == right_marker,
        (
            OwnershipSurface::SystemdUserUnit { unit: left_unit },
            OwnershipSurface::SystemdUserUnit { unit: right_unit },
        ) => left_unit == right_unit,
        _ => false,
    }
}

/// 返回固定 apply phase 的排序 key。
fn apply_phase(item: &PlanItem) -> u8 {
    match (&item.action, &item.surface) {
        (PlanAction::Delete, OwnershipSurface::SystemdUserUnit { .. }) => 0,
        (PlanAction::Delete, _) if applied_is_symlink(item) => 1,
        (PlanAction::Delete | PlanAction::Forget, _) => 2,
        (PlanAction::Create | PlanAction::Update, OwnershipSurface::SystemdUserUnit { .. }) => 5,
        (PlanAction::Create | PlanAction::Update, _) if desired_is_symlink(item) => 4,
        (PlanAction::Create | PlanAction::Update, _) => 3,
        _ => 3,
    }
}

/// 返回计划项的 Applied Resource 是否为 symlink。
fn applied_is_symlink(item: &PlanItem) -> bool {
    item.applied
        .as_ref()
        .is_some_and(|resource| matches!(&resource.state, ResourceState::Symlink { .. }))
}

/// 返回计划项的 Desired Resource 是否为 symlink。
fn desired_is_symlink(item: &PlanItem) -> bool {
    item.desired
        .as_ref()
        .is_some_and(|resource| matches!(resource, ResourceSpec::Symlink { .. }))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::min_ident_chars, clippy::missing_docs_in_private_items)]

    use super::*;
    use crate::AbsPath;

    /// 构造一个文件 Resource。
    fn file(path: &str, content: &[u8]) -> ResourceSpec {
        ResourceSpec::File {
            target: AbsPath::new(path),
            content: content.to_vec(),
            mode: 0o644,
            origin: "test".to_owned(),
        }
    }

    #[test]
    fn exact_unowned_resource_is_adopted() -> crate::Result<()> {
        let desired = file("/home/u/a", b"a");
        let surface = desired.surface();
        let observed = BTreeMap::from([(surface, ObservedState::Present(desired.desired_state()))]);
        let plan = reconcile(vec![desired], &[], &observed)?;
        assert!(matches!(
            plan.items.first().map(|item| &item.action),
            Some(PlanAction::Adopt)
        ));
        Ok(())
    }

    #[test]
    fn retired_drift_is_kept_in_inventory() -> crate::Result<()> {
        let desired = file("/home/u/a", b"a");
        let applied = AppliedResource {
            surface: desired.surface(),
            state: desired.desired_state(),
        };
        let observed = BTreeMap::from([(
            applied.surface.clone(),
            ObservedState::Present(file("/home/u/a", b"changed").desired_state()),
        )]);
        let plan = reconcile(Vec::new(), &[applied], &observed)?;
        assert!(matches!(
            plan.items.first().map(|item| &item.action),
            Some(PlanAction::Drift { .. })
        ));
        Ok(())
    }

    #[test]
    fn whole_file_conflicts_with_managed_block() {
        let result = reconcile(
            vec![
                file("/home/u/.zshrc", b"whole"),
                ResourceSpec::ManagedBlock {
                    target: AbsPath::new("/home/u/.zshrc"),
                    marker: "dots".to_owned(),
                    content: "source x".to_owned(),
                    placement: crate::ManagedBlockPlacement::End,
                    origin: "test block".to_owned(),
                },
            ],
            &[],
            &BTreeMap::new(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn unowned_mismatch_is_collision() -> crate::Result<()> {
        let desired = file("/home/u/a", b"desired");
        let surface = desired.surface();
        let observed = BTreeMap::from([(
            surface.clone(),
            ObservedState::Present(file("/home/u/a", b"actual").desired_state()),
        )]);
        let collision = reconcile(vec![desired], &[], &observed)?;
        assert!(matches!(
            collision.items.first().map(|item| &item.action),
            Some(PlanAction::Collision { .. })
        ));
        Ok(())
    }

    #[test]
    fn missing_applied_block_is_drift() -> crate::Result<()> {
        let desired = ResourceSpec::ManagedBlock {
            target: AbsPath::new("/home/u/.profile"),
            marker: "dots".to_owned(),
            content: "source x".to_owned(),
            placement: crate::ManagedBlockPlacement::End,
            origin: "test".to_owned(),
        };
        let applied = AppliedResource {
            surface: desired.surface(),
            state: desired.desired_state(),
        };
        let plan = reconcile(
            vec![desired],
            std::slice::from_ref(&applied),
            &BTreeMap::from([(applied.surface.clone(), ObservedState::Missing)]),
        )?;
        assert!(matches!(
            plan.items.first().map(|item| &item.action),
            Some(PlanAction::Drift { .. })
        ));
        Ok(())
    }

    #[test]
    fn directory_symlink_conflicts_with_descendant_resource() {
        let result = reconcile(
            vec![
                ResourceSpec::Symlink {
                    target: AbsPath::new("/home/u/.config/tool"),
                    source: AbsPath::new("/repo/tool"),
                    source_is_dir: true,
                    origin: "directory".to_owned(),
                },
                file("/home/u/.config/tool/extra", b"extra"),
            ],
            &[],
            &BTreeMap::new(),
        );
        assert!(result.is_err());
    }
}
