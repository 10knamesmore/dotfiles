//! 在 harness 协议之外执行共享的 PreToolUse 规则判定。

use super::engine;
use super::envelope;
use super::rules::{Config, Decision};

/// 一条已命中的工具调用规则。
#[derive(Debug, PartialEq, Eq)]
pub struct Match {
    /// 规则声明的抽象决策。
    pub decision: Decision,
    /// 提供给 harness 或模型的解释。
    pub reason: String,
}

/// 用共享规则判定一个 adapter 兼容的 PreToolUse JSON 信封。
///
/// 坏 JSON、字段缺失和未命中都返回 `None`，由各 harness adapter 保持 fail-open。
#[must_use]
pub fn evaluate(config: &Config, stdin_text: &str) -> Option<Match> {
    let (tool_name, tool_input) = envelope::parse_pretool(stdin_text)?;

    if tool_name == "Bash"
        && let Some(command) = tool_input
            .get("command")
            .and_then(serde_json::Value::as_str)
        && let Some(rule) = engine::check_bash(config, command)
    {
        return Some(Match {
            decision: rule.decision,
            reason: rule.reason.clone(),
        });
    }

    engine::check_tool(config, &tool_name, &tool_input).map(|rule| Match {
        decision: rule.decision,
        reason: rule.reason.clone(),
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::missing_docs_in_private_items)]

    use super::*;

    /// 共享判定层不应带入任何 harness 特定降级。
    #[test]
    fn preserves_abstract_ask_decision() -> Result<(), Box<dyn std::error::Error>> {
        let config = Config::from_toml(
            r#"
[[bash]]
cmd = "git"
subcmd = "push"
decision = "ask"
reason = "需要确认"
"#,
        )?;
        let input =
            serde_json::json!({"tool_name":"Bash","tool_input":{"command":"git push"}}).to_string();
        let Some(matched) = evaluate(&config, &input) else {
            return Err("规则应命中".into());
        };
        assert_eq!(matched.decision, Decision::Ask);
        assert_eq!(matched.reason, "需要确认");
        Ok(())
    }
}
