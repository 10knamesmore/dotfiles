//! Manifest —— dots.lua 求值结果的纯数据表示。
//!
//! core 拥有这份数据结构但**不依赖 mlua**：bin 的 lua 求值器把 Lua table 翻译成
//! `Manifest`，再喂给 core 计算 Plan。闭包（钩子/host 块）在此只存不透明 id，
//! 真正的 mlua 闭包句柄留在 bin 侧的 `LuaHandles`。

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
    /// systemd user 单元（sync 时 `systemctl --user enable`）。
    pub systemd_user: Vec<String>,
    /// scripts 聚合时不保树形、递归拍平的子目录名（子目录默认整目录链）。
    pub scripts_ignore_tree: Vec<String>,
    /// per-host 块：hostname → 闭包 id（effect 阶段由 bin 执行）。
    pub host_blocks: FxHashMap<String, ClosureId>,
    /// 已注册的生命周期钩子。
    pub hooks: Vec<HookReg>,
}

/// 链接粒度规格（§3 规则 3）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GranularitySpec {
    /// 粒度模式。
    pub mode: LinkMode,
    /// 链接时跳过的子项名（如 `node_modules`）。
    pub ignore: Vec<String>,
    /// 条目级 pre 钩子（链接该条目前执行；返回 false 则整条目跳过）。
    pub pre: Option<ClosureId>,
    /// 条目级 post 钩子（该条目链接完成后执行；被 pre 阻止则不执行）。
    pub post: Option<ClosureId>,
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
    /// 条目级 pre 钩子（返回 false 则整个分发跳过）。
    pub pre: Option<ClosureId>,
    /// 条目级 post 钩子（被 pre 阻止则不执行）。
    pub post: Option<ClosureId>,
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

/// 生命周期钩子注册项。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HookReg {
    /// 触发阶段。
    pub phase: HookPhase,
    /// 闭包 id（effect 阶段由 bin 在 `LuaHandles` 中查回真正闭包执行）。
    pub closure: ClosureId,
}

/// 生命周期点。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HookPhase {
    /// 链接落盘前。
    PreSync,
    /// 链接全部建好后（含 `.inject` 渲染）。
    PostLink,
    /// sync 末尾：链接 / inject / env.zsh / systemd enable 全部就绪后、台账保存前。
    PostSync,
    /// 命中 host 块激活时。
    OnHostActivate,
}

impl HookPhase {
    /// 从 Lua 端传入的字符串解析。
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "pre_sync" => Some(Self::PreSync),
            "post_link" => Some(Self::PostLink),
            "post_sync" => Some(Self::PostSync),
            "on_host_activate" => Some(Self::OnHostActivate),
            _ => None,
        }
    }
}

/// 不透明闭包 id：core 不持有 mlua 闭包，只记一个序号，bin 据此查回。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ClosureId(pub u32);

#[cfg(test)]
mod tests {
    #![allow(clippy::min_ident_chars, clippy::missing_docs_in_private_items)]
    use super::*;

    #[test]
    fn hook_phase_parse() {
        assert_eq!(HookPhase::parse("post_link"), Some(HookPhase::PostLink));
        assert_eq!(HookPhase::parse("pre_sync"), Some(HookPhase::PreSync));
        assert_eq!(HookPhase::parse("nope"), None);
    }

    #[test]
    fn manifest_default_empty() {
        let m = Manifest::default();
        assert!(m.distribute.is_empty());
        assert!(m.hooks.is_empty());
        assert!(m.granularity.is_empty());
    }
}
