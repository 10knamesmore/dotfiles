//! 将共享决策转换为 Claude Code PreToolUse 输出。

use super::envelope::PreToolUseOutput;
use super::rules::Decision;

/// 保留 Claude Code 原生支持的 `deny` 与 `ask` 决策。
#[must_use]
pub fn output(decision: Decision, reason: &str) -> PreToolUseOutput {
    PreToolUseOutput::from_decision(decision, reason)
}
