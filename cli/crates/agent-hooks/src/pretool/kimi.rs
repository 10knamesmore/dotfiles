//! 将共享输入与决策转换为 Kimi Code PreToolUse 协议。

use super::envelope::PreToolUseOutput;
use super::rules::Decision;

/// Kimi Code 不支持 `ask`；转换为带原始理由的 `deny`，避免 hook failure 后继续调用。
#[must_use]
pub fn output(decision: Decision, reason: &str) -> PreToolUseOutput {
    let permission_decision_reason = match decision {
        Decision::Deny => reason.to_owned(),
        Decision::Ask => format!(
            "Kimi Code PreToolUse 暂不支持 ask，已按 hard deny 处理；需要执行时请由用户手动完成。原规则：{reason}"
        ),
    };
    PreToolUseOutput::deny(permission_decision_reason)
}

/// 把 Kimi Code 的工具名改写为共享规则使用的 canonical 名称。
///
/// 两者工具集几乎同名，已知差异是网页抓取工具：Kimi 叫 `FetchURL`，
/// 共享规则使用 `WebFetch`。解析失败时原样返回，保持 fail-open。
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
