//! hook JSON 边界：stdin 解析与 `hookSpecificOutput` 渲染。

use crate::rules::Rule;

/// 从 PreToolUse 的 stdin JSON 提取 Bash 命令。
///
/// 取不出合法字符串（坏 JSON / 字段缺失 / 类型不对）一律 `None`——fail-open。
#[must_use]
pub fn extract_command(stdin_text: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(stdin_text).ok()?;
    Some(value.get("tool_input")?.get("command")?.as_str()?.to_owned())
}

/// 渲染单行决策 JSON（`hookSpecificOutput.permissionDecision`）。
#[must_use]
pub fn render(rule: &Rule) -> String {
    serde_json::json!({
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "permissionDecision": rule.decision.as_str(),
            "permissionDecisionReason": rule.reason,
        }
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::min_ident_chars, clippy::missing_docs_in_private_items)]
    use super::*;
    use crate::rules::{Config, Decision};

    #[test]
    fn extracts_command() {
        let payload = r#"{"tool_input":{"command":"rm -rf /tmp"}}"#;
        assert_eq!(extract_command(payload).as_deref(), Some("rm -rf /tmp"));
    }

    #[test]
    fn malformed_inputs_yield_none() {
        let bads = ["", "not json", "{}", r#"{"tool_input":{}}"#, r#"{"tool_input":{"command":42}}"#];
        for bad in bads {
            assert_eq!(extract_command(bad), None, "input: {bad}");
        }
    }

    #[test]
    fn renders_decision_envelope() -> Result<(), Box<dyn std::error::Error>> {
        let config = Config::from_toml(
            "[[rules]]\nname='x'\ncmd='rm'\ndecision='deny'\nreason='理由'",
        )?;
        let rule = config.rules.first().ok_or("fixture 至少一条规则")?;
        let value: serde_json::Value = serde_json::from_str(&render(rule))?;
        let output = &value["hookSpecificOutput"];
        assert_eq!(output["hookEventName"], "PreToolUse");
        assert_eq!(output["permissionDecision"], "deny");
        assert_eq!(output["permissionDecisionReason"], "理由");
        assert_eq!(rule.decision, Decision::Deny);
        Ok(())
    }
}
