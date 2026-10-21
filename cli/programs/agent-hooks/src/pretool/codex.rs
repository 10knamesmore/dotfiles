//! 将共享决策转换为 Codex PreToolUse 输出。

use super::envelope::PreToolUseOutput;
use super::rules::Decision;

/// Codex 不支持 `ask`；转换为带原始理由的 `deny`，避免 hook failure 后继续调用。
#[must_use]
pub fn output(decision: Decision, reason: &str) -> PreToolUseOutput {
    let permission_decision_reason = match decision {
        Decision::Deny => reason.to_owned(),
        Decision::Ask => format!(
            "Codex PreToolUse 暂不支持 ask，已按 hard deny 处理；需要执行时请由用户手动完成。原规则：{reason}"
        ),
    };
    PreToolUseOutput::deny(permission_decision_reason)
}
