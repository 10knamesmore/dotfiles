//! Kimi Code PreToolUse 的输出信封、能力降级与工具名映射。

use serde::Serialize;

use super::rules::Decision;

/// Kimi Code PreToolUse 的整条 stdout JSON。
#[derive(Debug, Serialize)]
pub struct PreToolUseOutput {
    /// Kimi Code 读取的 hook 专属载荷。
    #[serde(rename = "hookSpecificOutput")]
    pub hook_specific_output: PreToolUseDecision,
}

/// Kimi Code PreToolUse 的 deny 决策。
#[derive(Debug, Serialize)]
pub struct PreToolUseDecision {
    /// 恒为 `PreToolUse`。
    #[serde(rename = "hookEventName")]
    pub hook_event_name: &'static str,
    /// 当前只输出 Kimi Code 已支持的 `deny`。
    #[serde(rename = "permissionDecision")]
    pub permission_decision: &'static str,
    /// 提供给模型和用户的阻止原因。
    #[serde(rename = "permissionDecisionReason")]
    pub permission_decision_reason: String,
}

impl PreToolUseOutput {
    /// 把共享决策转换为 Kimi Code 当前支持的 PreToolUse 输出。
    ///
    /// Kimi Code 暂不支持 `permissionDecision: "ask"`；为避免 hook failure 后继续执行，
    /// `Ask` 会降级为带解释的 `deny`。
    #[must_use]
    pub fn new(decision: Decision, reason: &str) -> Self {
        let permission_decision_reason = match decision {
            Decision::Deny => reason.to_owned(),
            Decision::Ask => format!(
                "Kimi Code PreToolUse 暂不支持 ask，已按 hard deny 处理；需要执行时请由用户手动完成。原规则：{reason}"
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

/// 把 Kimi Code 的工具名改写为共享规则表使用的 Claude 命名。
///
/// 两者工具集几乎同名，已知差异是网页抓取工具：Kimi 叫 `FetchURL`，
/// 共享规则表按 Claude 的 `WebFetch` 书写。解析失败时原样返回，保持 fail-open。
#[must_use]
pub fn rewrite_stdin(stdin_text: &str) -> String {
    let Ok(mut value) = serde_json::from_str::<serde_json::Value>(stdin_text) else {
        return stdin_text.to_owned();
    };
    if value.get("tool_name").and_then(serde_json::Value::as_str) == Some("FetchURL") {
        value["tool_name"] = serde_json::Value::String("WebFetch".to_owned());
    }
    value.to_string()
}
