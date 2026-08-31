//! 将 Pi 工具调用与共享 PreToolUse 规则协议互转。

use serde::Serialize;

use super::rules::Decision;

/// Pi extension 读取的中立守卫决策。
#[derive(Debug, Serialize)]
pub struct PiPreToolOutput {
    /// `allow`、`deny` 或 `ask`。
    pub decision: &'static str,

    /// 命中规则的原始理由；`allow` 时省略。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl PiPreToolOutput {
    /// 表示守卫不阻断当前调用。
    #[must_use]
    pub const fn allow() -> Self {
        Self {
            decision: "allow",
            reason: None,
        }
    }

    /// 保留共享规则的 `deny`/`ask` 语义和理由。
    #[must_use]
    pub fn from_decision(decision: Decision, reason: &str) -> Self {
        Self {
            decision: decision.as_str(),
            reason: Some(reason.to_owned()),
        }
    }
}

/// 把 Pi 小写 tool 名和 `path` 字段改写为共享规则词汇。
///
/// 坏 JSON、缺少 tool name/input 或非 object input 返回 `None`，由
/// caller 以可见诊断 fail-open。未知 custom tool 保留原名称和输入。
#[must_use]
pub fn rewrite_stdin(stdin_text: &str) -> Option<String> {
    let mut envelope = serde_json::from_str::<serde_json::Value>(stdin_text).ok()?;
    let tool_name = envelope.get("tool_name")?.as_str()?;
    let canonical_name = match tool_name {
        "bash" => "Bash",
        "read" => "Read",
        "edit" => "Edit",
        "write" => "Write",
        "webfetch" => "WebFetch",
        _ => tool_name,
    }
    .to_owned();

    let tool_input = envelope.get_mut("tool_input")?.as_object_mut()?;
    if matches!(canonical_name.as_str(), "Read" | "Edit" | "Write")
        && let Some(path) = tool_input.get("path").cloned()
    {
        tool_input.insert("file_path".to_owned(), path);
    }
    envelope["tool_name"] = serde_json::Value::String(canonical_name);
    Some(envelope.to_string())
}
