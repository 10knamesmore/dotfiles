//! hook JSON 边界：stdin 解析与结构化输出类型。
//!
//! 输出是 named struct（serde 序列化），不在业务函数里手拼/打印 JSON——
//! 落地（println/eprintln/exit）统一由 bin 的 wire 层完成。

use serde::Serialize;

use crate::pretool::rules::Decision;

/// 从 PreToolUse 的 stdin JSON 提取 `(tool_name, tool_input)`。
///
/// 取不出（坏 JSON / 字段缺失 / 类型不对）一律 `None`——fail-open。
#[must_use]
pub fn parse_pretool(stdin_text: &str) -> Option<(String, serde_json::Value)> {
    let value: serde_json::Value = serde_json::from_str(stdin_text).ok()?;
    let tool_name = value.get("tool_name")?.as_str()?.to_owned();
    let tool_input = value.get("tool_input")?.clone();
    Some((tool_name, tool_input))
}

/// PreToolUse 的结构化输出（整条 stdout JSON）。
#[derive(Debug, Serialize)]
pub struct PreToolUseOutput {
    /// CC hook 协议的决策载荷
    #[serde(rename = "hookSpecificOutput")]
    pub hook_specific_output: PreToolUseDecision,
}

/// `hookSpecificOutput` 载荷。
#[derive(Debug, Serialize)]
pub struct PreToolUseDecision {
    /// 恒为 "PreToolUse"
    #[serde(rename = "hookEventName")]
    pub hook_event_name: &'static str,
    /// "deny" | "ask"
    #[serde(rename = "permissionDecision")]
    pub permission_decision: &'static str,
    /// 喂回模型/展示给用户的理由
    #[serde(rename = "permissionDecisionReason")]
    pub permission_decision_reason: String,
}

impl PreToolUseOutput {
    /// 由命中规则的决策与理由构造。
    #[must_use]
    pub fn new(decision: Decision, reason: &str) -> Self {
        Self {
            hook_specific_output: PreToolUseDecision {
                hook_event_name: "PreToolUse",
                permission_decision: decision.as_str(),
                permission_decision_reason: reason.to_owned(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::min_ident_chars, clippy::missing_docs_in_private_items)]
    use super::*;

    #[test]
    fn parses_tool_name_and_input() {
        let payload = r#"{"tool_name":"Bash","tool_input":{"command":"rm -rf /tmp"}}"#;
        let parsed = parse_pretool(payload);
        assert!(parsed.is_some());
        let (tool_name, tool_input) = parsed.unwrap_or(("?".into(), serde_json::Value::Null));
        assert_eq!(tool_name, "Bash");
        assert_eq!(tool_input["command"], "rm -rf /tmp");
    }

    #[test]
    fn malformed_inputs_yield_none() {
        let bads = [
            "",
            "not json",
            "{}",
            r#"{"tool_input":{"command":"x"}}"#, // 缺 tool_name
            r#"{"tool_name":42,"tool_input":{}}"#,
            r#"{"tool_name":"Bash"}"#, // 缺 tool_input
        ];
        for bad in bads {
            assert!(parse_pretool(bad).is_none(), "input: {bad}");
        }
    }

    #[test]
    fn output_serializes_to_protocol_shape() -> Result<(), serde_json::Error> {
        let output = PreToolUseOutput::new(Decision::Deny, "理由");
        let value: serde_json::Value = serde_json::from_str(&serde_json::to_string(&output)?)?;
        let payload = &value["hookSpecificOutput"];
        assert_eq!(payload["hookEventName"], "PreToolUse");
        assert_eq!(payload["permissionDecision"], "deny");
        assert_eq!(payload["permissionDecisionReason"], "理由");
        Ok(())
    }
}
