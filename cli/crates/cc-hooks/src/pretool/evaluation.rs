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
    /// 用于审计和定位配置的规则名。
    pub rule_name: String,
    /// hook 输入中的 canonical tool name。
    pub tool_name: String,
    /// Bash 命令摘要；非 Bash 工具为 `None`。
    pub command: Option<String>,
}

impl Match {
    /// 按 harness 最终落地的决策生成审计行。
    #[must_use]
    pub fn audit(&self, effective_decision: &str) -> String {
        let source = if effective_decision == self.decision.as_str() {
            String::new()
        } else {
            format!(" source_decision={}", self.decision.as_str())
        };
        let command = self
            .command
            .as_deref()
            .map(|value| format!(" cmd={}", snippet(value)))
            .unwrap_or_default();
        format!(
            "decision={effective_decision}{source} tool={} rule={}{}",
            self.tool_name, self.rule_name, command
        )
    }
}

/// 用共享规则判定一个 Claude/Codex 兼容的 PreToolUse JSON 信封。
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
            rule_name: rule.name.clone(),
            tool_name,
            command: Some(command.to_owned()),
        });
    }

    engine::check_tool(config, &tool_name, &tool_input).map(|rule| Match {
        decision: rule.decision,
        reason: rule.reason.clone(),
        rule_name: rule.name.clone(),
        tool_name,
        command: None,
    })
}

/// 截断并单行化 Bash 命令，防止审计日志被长命令撑爆。
fn snippet(command: &str) -> String {
    command
        .chars()
        .take(200)
        .collect::<String>()
        .replace('\n', " ")
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
name = "git-push"
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
        assert_eq!(
            matched.audit("deny"),
            "decision=deny source_decision=ask tool=Bash rule=git-push cmd=git push"
        );
        Ok(())
    }
}
