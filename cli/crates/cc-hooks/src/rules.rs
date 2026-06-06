//! bash-guard 规则的 TOML schema。
//!
//! 规则文件即配置（`~/.claude/hooks/bash-guard.toml`），改规则无需重编译。
//! 一条规则内给定的条件取 AND；`any`/`all` 内层列表取 OR。

use serde::Deserialize;

/// 决策档位：deny 直接拦（理由喂回模型），ask 弹确认框给用户。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Decision {
    /// 拒绝执行
    Deny,
    /// 升级给用户确认
    Ask,
}

impl Decision {
    /// `hookSpecificOutput.permissionDecision` 的字面值。
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Deny => "deny",
            Self::Ask => "ask",
        }
    }
}

/// 单条拦截规则。
///
/// 词形约定（`any` / `all` 内的元素）：
/// - `-x`（单杠+单字母）→ 短旗标字母，可命中合写簇（`-rf` 含 `r` 与 `f`）
/// - 其余（`--word` 或裸词）→ argv 里的字面词
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rule {
    /// 规则名（报告/调试用）
    pub name: String,
    /// argv[0] 必须等于它（`command` 前缀自动剥除）
    pub cmd: String,
    /// 若给定，argv[1] 必须等于它（如 git 子命令）
    #[serde(default)]
    pub subcmd: Option<String>,
    /// AND-of-OR 词组：每组至少命中一个词形
    #[serde(default)]
    pub all: Vec<Vec<String>>,
    /// 词形列表：任一命中即满足
    #[serde(default)]
    pub any: Vec<String>,
    /// 位置参数正则（作用于 cmd/subcmd 之后的 argv）：任一命中即满足
    #[serde(default)]
    pub args_re: Vec<String>,
    /// 命中后的决策
    pub decision: Decision,
    /// 喂回模型/展示给用户的理由
    pub reason: String,
}

/// 规则集（TOML 顶层 `[[rules]]`）。
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// 规则按声明顺序匹配，首条命中生效
    pub rules: Vec<Rule>,
}

impl Config {
    /// 从 TOML 文本解析规则集。
    ///
    /// # Errors
    /// TOML 语法错误或字段不符合 schema（含未知字段）时返回解析错误。
    pub fn from_toml(text: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(text)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::min_ident_chars, clippy::missing_docs_in_private_items)]
    use super::*;

    const FIXTURE: &str = r#"
[[rules]]
name     = "rm-recursive-force"
cmd      = "rm"
all      = [["-r", "--recursive"], ["-f", "--force"]]
decision = "deny"
reason   = "rm 递归+强制"

[[rules]]
name     = "git-push-main"
cmd      = "git"
subcmd   = "push"
args_re  = ["^(main|master)$", ":(main|master)$"]
decision = "ask"
reason   = "推主分支"
"#;

    #[test]
    fn parses_rule_set() -> Result<(), toml::de::Error> {
        let config = Config::from_toml(FIXTURE)?;
        assert_eq!(config.rules.len(), 2);
        assert_eq!(config.rules[0].name, "rm-recursive-force");
        assert_eq!(config.rules[0].decision, Decision::Deny);
        assert_eq!(config.rules[0].all.len(), 2);
        assert_eq!(config.rules[1].subcmd.as_deref(), Some("push"));
        assert_eq!(config.rules[1].decision, Decision::Ask);
        Ok(())
    }

    #[test]
    fn unknown_field_is_rejected() {
        let bad = "[[rules]]\nname='x'\ncmd='y'\ndecision='deny'\nreason='r'\nbogus=1";
        assert!(Config::from_toml(bad).is_err());
    }

    #[test]
    fn decision_renders_lowercase() {
        assert_eq!(Decision::Deny.as_str(), "deny");
        assert_eq!(Decision::Ask.as_str(), "ask");
    }
}
