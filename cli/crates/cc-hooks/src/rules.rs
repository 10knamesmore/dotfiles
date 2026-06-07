//! pretool 规则的 TOML schema。
//!
//! 规则文件即配置（`~/.claude/hooks/pretool.toml`），改规则无需重编译。
//! 两种规则形态：`[[bash]]`（argv 引擎：词法/旗标簇/子命令）与 `[[tool]]`
//! （任意工具：`tool_name` + `tool_input` 字段的具名匹配器）。
//! 一条规则内给定的条件取 AND；匹配器数组值内层取 OR。

use std::collections::BTreeMap;

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

/// Bash 专用规则（作用于 `tool_input.command` 的 argv 分词结果）。
///
/// 词形约定（`any` / `all` 内的元素）：
/// - `-x`（单杠+单字母）→ 短旗标字母，可命中合写簇（`-rf` 含 `r` 与 `f`）
/// - 其余（`--word` 或裸词）→ argv 里的字面词
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BashRule {
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

/// 通用工具规则：`tool` 匹配 `tool_name`，`where` 各字段匹配器全部命中（AND）。
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolRule {
    /// 规则名（报告/调试用）
    pub name: String,
    /// 目标工具名（精确匹配，如 "WebFetch" / "Read"）
    pub tool: String,
    /// 字段名 → 匹配器；多字段 AND。字段值非字符串视为不命中。
    #[serde(rename = "where", default)]
    pub conditions: BTreeMap<String, Matcher>,
    /// 命中后的决策
    pub decision: Decision,
    /// 喂回模型/展示给用户的理由
    pub reason: String,
}

/// 单值或数组：TOML 里 `"x"` 与 `["x", "y"]` 等价写法，数组内任一命中（OR）。
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum OneOrMany {
    /// 单个备选值
    One(String),
    /// 多个备选值（任一命中）
    Many(Vec<String>),
}

impl OneOrMany {
    /// 任一备选值满足谓词即真。
    pub fn any(&self, pred: impl Fn(&str) -> bool) -> bool {
        match self {
            Self::One(value) => pred(value),
            Self::Many(values) => values.iter().any(|value| pred(value)),
        }
    }
}

/// 字段匹配器：意图词汇替代裸正则，`re` 仅作兜底。
///
/// 同一匹配器内给定的多个种类取 AND（如 `prefix` 且 `suffix`）；
/// 每个种类的数组值内层取 OR。全空匹配器恒真（= 只看工具名/字段存在性）。
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Matcher {
    /// 全等
    #[serde(default)]
    pub equals: Option<OneOrMany>,
    /// 含子串
    #[serde(default)]
    pub contains: Option<OneOrMany>,
    /// 前缀
    #[serde(default)]
    pub prefix: Option<OneOrMany>,
    /// 后缀
    #[serde(default)]
    pub suffix: Option<OneOrMany>,
    /// 路径 glob（`**` 跨目录，git 风格）
    #[serde(default)]
    pub glob: Option<OneOrMany>,
    /// URL 域名（含子域：`gist.github.com` 命中 `github.com`）
    #[serde(default)]
    pub domain: Option<OneOrMany>,
    /// 正则兜底（仅在上述词汇表达不了时用）
    #[serde(default)]
    pub re: Option<OneOrMany>,
}

/// 规则集（TOML 顶层 `[[bash]]` 与 `[[tool]]`）。
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Bash 规则，按声明顺序匹配，首条命中生效
    #[serde(default)]
    pub bash: Vec<BashRule>,
    /// 通用工具规则，按声明顺序匹配，首条命中生效
    #[serde(default)]
    pub tool: Vec<ToolRule>,
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
[[bash]]
name     = "rm-recursive-force"
cmd      = "rm"
all      = [["-r", "--recursive"], ["-f", "--force"]]
decision = "deny"
reason   = "rm 递归+强制"

[[tool]]
name     = "gh-not-webfetch-github"
tool     = "WebFetch"
where    = { url = { domain = "github.com" } }
decision = "deny"
reason   = "GitHub 一律 gh"

[[tool]]
name     = "dotenv"
tool     = "Read"
where    = { file_path = { glob = ["**/.env", "**/.env.*"] } }
decision = "ask"
reason   = "敏感文件"
"#;

    #[test]
    fn parses_both_rule_kinds() -> Result<(), toml::de::Error> {
        let config = Config::from_toml(FIXTURE)?;
        assert_eq!(config.bash.len(), 1);
        assert_eq!(config.tool.len(), 2);
        assert_eq!(config.bash[0].decision, Decision::Deny);
        assert_eq!(config.tool[0].tool, "WebFetch");
        assert!(config.tool[0].conditions.contains_key("url"));
        assert_eq!(config.tool[1].decision, Decision::Ask);
        Ok(())
    }

    #[test]
    fn one_or_many_accepts_both_shapes() -> Result<(), toml::de::Error> {
        let config = Config::from_toml(FIXTURE)?;
        let url_matcher = config.tool[0].conditions.get("url").map(|m| &m.domain);
        assert!(matches!(url_matcher, Some(Some(OneOrMany::One(_)))));
        let glob_matcher = config.tool[1].conditions.get("file_path").map(|m| &m.glob);
        assert!(matches!(glob_matcher, Some(Some(OneOrMany::Many(_)))));
        Ok(())
    }

    #[test]
    fn unknown_field_is_rejected() {
        let bad_bash = "[[bash]]\nname='x'\ncmd='y'\ndecision='deny'\nreason='r'\nbogus=1";
        assert!(Config::from_toml(bad_bash).is_err());
        let bad_matcher =
            "[[tool]]\nname='x'\ntool='Read'\nwhere={f={fuzzy='x'}}\ndecision='deny'\nreason='r'";
        assert!(Config::from_toml(bad_matcher).is_err());
    }

    #[test]
    fn decision_renders_lowercase() {
        assert_eq!(Decision::Deny.as_str(), "deny");
        assert_eq!(Decision::Ask.as_str(), "ask");
    }
}
