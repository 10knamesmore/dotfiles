//! Codex PreToolUse 的输出信封与能力降级。

use serde::Serialize;

use super::rules::Decision;

/// Codex PreToolUse 的整条 stdout JSON。
#[derive(Debug, Serialize)]
pub struct PreToolUseOutput {
    /// Codex 读取的 hook 专属载荷。
    #[serde(rename = "hookSpecificOutput")]
    pub hook_specific_output: PreToolUseDecision,
}

/// Codex PreToolUse 的 deny 决策。
#[derive(Debug, Serialize)]
pub struct PreToolUseDecision {
    /// 恒为 `PreToolUse`。
    #[serde(rename = "hookEventName")]
    pub hook_event_name: &'static str,
    /// 当前只输出 Codex 已支持的 `deny`。
    #[serde(rename = "permissionDecision")]
    pub permission_decision: &'static str,
    /// 提供给模型和用户的阻止原因。
    #[serde(rename = "permissionDecisionReason")]
    pub permission_decision_reason: String,
}

impl PreToolUseOutput {
    /// 把共享决策转换为 Codex 当前支持的 PreToolUse 输出。
    ///
    /// Codex 暂不支持 `permissionDecision: "ask"`；为避免 hook failure 后继续执行，
    /// `Ask` 会降级为带解释的 `deny`。
    #[must_use]
    pub fn new(decision: Decision, reason: &str) -> Self {
        let permission_decision_reason = match decision {
            Decision::Deny => reason.to_owned(),
            Decision::Ask => format!(
                "Codex PreToolUse 暂不支持 ask，已按 hard deny 处理；需要执行时请由用户手动完成。原规则：{reason}"
            ),
        };
        Self {
            hook_specific_output: PreToolUseDecision {
                hook_event_name: "PreToolUse",
                permission_decision: "deny",
                permission_decision_reason,
            },
        }
    }
}
