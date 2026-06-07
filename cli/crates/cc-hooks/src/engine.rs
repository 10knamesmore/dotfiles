//! 规则匹配：hook 负载 × 规则集 → 首条命中。
//!
//! - [`check_bash`]：原始命令串逐段（链式命令拆开）过 `[[bash]]` 规则
//! - [`check_tool`]：`tool_name` + `tool_input` 字段过 `[[tool]]` 规则

use std::collections::BTreeSet;

use regex_lite::Regex;

use crate::argv;
use crate::rules::{BashRule, Config, Matcher, ToolRule};

/// 对原始命令串逐段（链式命令拆开）逐规则匹配，返回首条命中。
///
/// 全部不中返回 `None`（= 守卫无意见，走正常权限流程）。
#[must_use]
pub fn check_bash<'cfg>(config: &'cfg Config, raw: &str) -> Option<&'cfg BashRule> {
    argv::each_command(raw)
        .iter()
        .find_map(|segment| config.bash.iter().find(|rule| bash_hit(rule, segment)))
}

/// 工具负载逐规则匹配：工具名精确相等且 `where` 各字段匹配器全中。
///
/// 字段缺失或值非字符串 → 该条件不中（fail-open 朝放行倾斜）。
#[must_use]
pub fn check_tool<'cfg>(
    config: &'cfg Config,
    tool_name: &str,
    tool_input: &serde_json::Value,
) -> Option<&'cfg ToolRule> {
    config.tool.iter().find(|rule| {
        rule.tool == tool_name
            && rule.conditions.iter().all(|(field, matcher)| {
                tool_input
                    .get(field)
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|value| matcher_hit(matcher, value))
            })
    })
}

/// 单字段值是否满足匹配器：给定的各种类 AND，每种类数组值内 OR。
fn matcher_hit(matcher: &Matcher, value: &str) -> bool {
    if let Some(alts) = &matcher.equals
        && !alts.any(|alt| value == alt)
    {
        return false;
    }
    if let Some(alts) = &matcher.contains
        && !alts.any(|alt| value.contains(alt))
    {
        return false;
    }
    if let Some(alts) = &matcher.prefix
        && !alts.any(|alt| value.starts_with(alt))
    {
        return false;
    }
    if let Some(alts) = &matcher.suffix
        && !alts.any(|alt| value.ends_with(alt))
    {
        return false;
    }
    if let Some(alts) = &matcher.glob
        && !alts.any(|alt| glob_hit(alt, value))
    {
        return false;
    }
    if let Some(alts) = &matcher.domain
        && !alts.any(|alt| domain_hit(alt, value))
    {
        return false;
    }
    if let Some(alts) = &matcher.re
        && !alts.any(|alt| Regex::new(alt).is_ok_and(|regex| regex.is_match(value)))
    {
        return false;
    }
    true
}

/// git 风格 glob 命中（`**/.env` 命中 `.env` 与 `a/b/.env`）。坏 glob 视为不中。
fn glob_hit(pattern: &str, value: &str) -> bool {
    globset::Glob::new(pattern).is_ok_and(|glob| glob.compile_matcher().is_match(value))
}

/// URL 域名命中：host 全等或以 `.<domain>` 结尾（含子域）。
///
/// 手剥 scheme/userinfo/端口，不引 url crate；大小写不敏感。
fn domain_hit(domain: &str, url: &str) -> bool {
    let rest = url.split_once("://").map_or(url, |(_, tail)| tail);
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    // `user@host` 形态取 @ 之后；`host:port` 剥端口
    let host = authority.rsplit('@').next().unwrap_or(authority);
    let host = host.split(':').next().unwrap_or(host).to_ascii_lowercase();
    let domain = domain.to_ascii_lowercase();
    host == domain || host.strip_suffix(&domain).is_some_and(|head| head.ends_with('.'))
}

/// 单段 argv 是否命中 bash 规则的全部条件（条件间 AND）。
fn bash_hit(rule: &BashRule, segment: &[String]) -> bool {
    let words = strip_command_prefix(segment);
    if words.first().map(String::as_str) != Some(rule.cmd.as_str()) {
        return false;
    }
    let positional_start = match &rule.subcmd {
        Some(subcmd) => {
            if words.get(1).map(String::as_str) != Some(subcmd.as_str()) {
                return false;
            }
            2
        }
        None => 1,
    };
    let flags = argv::short_flags(words);

    let any_ok = rule.any.is_empty() || rule.any.iter().any(|alt| word_hit(alt, words, &flags));
    let all_ok = rule
        .all
        .iter()
        .all(|group| group.iter().any(|alt| word_hit(alt, words, &flags)));
    let args_re_ok = rule.args_re.is_empty() || {
        let tail = words.get(positional_start..).unwrap_or(&[]);
        rule.args_re.iter().any(|pattern| {
            // 坏正则视为不命中（fail-open），不拖垮整条规则之外的判定
            Regex::new(pattern).is_ok_and(|matcher| tail.iter().any(|arg| matcher.is_match(arg)))
        })
    };
    any_ok && all_ok && args_re_ok
}

/// 词形命中：`-x`（单杠+单字母）查短旗标簇，其余按字面词查 argv。
fn word_hit(alt: &str, words: &[String], flags: &BTreeSet<char>) -> bool {
    let mut chars = alt.chars();
    if let (Some('-'), Some(letter), None) = (chars.next(), chars.next(), chars.next())
        && letter.is_ascii_alphabetic()
    {
        return flags.contains(&letter);
    }
    words.iter().any(|word| word == alt)
}

/// 剥掉 POSIX `command` 内建前缀（`command rm …` → `rm …`）。
fn strip_command_prefix(segment: &[String]) -> &[String] {
    match segment.first() {
        Some(head) if head == "command" => segment.get(1..).unwrap_or(&[]),
        _ => segment,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::min_ident_chars, clippy::missing_docs_in_private_items)]
    use super::*;
    use crate::rules::Decision;

    /// bash 规则 fixture（与 tree/home/.claude/hooks/pretool.toml 同步维护）。
    const RULES: &str = r#"
[[bash]]
name     = "rm-recursive-force"
cmd      = "rm"
all      = [["-r", "--recursive"], ["-f", "--force"]]
decision = "deny"
reason   = "rm 递归+强制"

[[bash]]
name     = "git-push"
cmd      = "git"
subcmd   = "push"
decision = "ask"
reason   = "git 推送"

[[bash]]
name     = "git-reset-hard"
cmd      = "git"
subcmd   = "reset"
any      = ["--hard"]
decision = "ask"
reason   = "丢弃改动"

[[bash]]
name     = "git-clean-force"
cmd      = "git"
subcmd   = "clean"
all      = [["-f", "--force"]]
decision = "ask"
reason   = "丢弃改动"

[[tool]]
name     = "webfetch-github"
tool     = "WebFetch"
where    = { url = { domain = "github.com" } }
decision = "deny"
reason   = "GitHub 一律 gh"
"#;

    /// (输入命令, 期望命中：None=静默放行)
    const BASH_CASES: &[(&str, Option<(&str, Decision)>)] = &[
        // ── deny ──
        ("rm -rf /tmp/foo", Some(("rm-recursive-force", Decision::Deny))),
        ("cd /tmp && rm -fr build", Some(("rm-recursive-force", Decision::Deny))),
        ("cd /tmp\nrm -rf build", Some(("rm-recursive-force", Decision::Deny))),
        ("cat list | rm -rf x", Some(("rm-recursive-force", Decision::Deny))),
        ("command rm -rf x", Some(("rm-recursive-force", Decision::Deny))),
        ("rm --recursive --force x", Some(("rm-recursive-force", Decision::Deny))),
        // ── ask ──
        ("git push origin dev", Some(("git-push", Decision::Ask))),
        ("git push -f origin dev", Some(("git-push", Decision::Ask))),
        ("git reset --hard HEAD~1", Some(("git-reset-hard", Decision::Ask))),
        ("git clean -fd", Some(("git-clean-force", Decision::Ask))),
        // ── 静默放行 ──
        ("git add -A", None),
        ("ls -la && cargo build", None),
        ("rm -r /tmp/foo", None),
        ("rm my-perf-report.txt", None),
        (r#"echo "rm -rf /""#, None),
        ("rm -1 weird", None),
    ];

    #[test]
    fn bash_verdict_table() -> Result<(), toml::de::Error> {
        let config = Config::from_toml(RULES)?;
        for (command, expected) in BASH_CASES {
            let verdict =
                check_bash(&config, command).map(|rule| (rule.name.as_str(), rule.decision));
            assert_eq!(verdict, *expected, "command: {command}");
        }
        Ok(())
    }

    #[test]
    fn tool_rule_matches_domain_with_subdomains() -> Result<(), toml::de::Error> {
        let config = Config::from_toml(RULES)?;
        let hits = |url: &str| {
            let input = serde_json::json!({ "url": url });
            check_tool(&config, "WebFetch", &input).is_some()
        };
        assert!(hits("https://github.com/foo/bar"));
        assert!(hits("https://gist.github.com/x"));
        assert!(hits("HTTPS://GITHUB.COM/UP"));
        assert!(hits("https://user@github.com:8443/path"));
        assert!(!hits("https://raw.githubusercontent.com/a/b"));
        assert!(!hits("https://notgithub.com/"));
        assert!(!hits("https://example.com/github.com/decoy"));
        Ok(())
    }

    #[test]
    fn tool_rule_requires_matching_tool_name_and_field() -> Result<(), toml::de::Error> {
        let config = Config::from_toml(RULES)?;
        let input = serde_json::json!({ "url": "https://github.com/x" });
        assert!(check_tool(&config, "WebSearch", &input).is_none(), "工具名不同不命中");
        let no_field = serde_json::json!({ "prompt": "hi" });
        assert!(check_tool(&config, "WebFetch", &no_field).is_none(), "字段缺失不命中");
        let wrong_type = serde_json::json!({ "url": 42 });
        assert!(check_tool(&config, "WebFetch", &wrong_type).is_none(), "非字符串不命中");
        Ok(())
    }

    #[test]
    fn matcher_kinds_cover_vocabulary() {
        let matcher = |toml_text: &str| -> Matcher {
            toml::from_str(toml_text).unwrap_or_default()
        };
        assert!(matcher_hit(&matcher(r#"equals = "main""#), "main"));
        assert!(!matcher_hit(&matcher(r#"equals = "main""#), "main2"));
        assert!(matcher_hit(&matcher(r#"contains = ["aa", "bb"]"#), "xbbx"));
        assert!(matcher_hit(&matcher(r#"prefix = "pre-""#), "pre-x"));
        assert!(matcher_hit(&matcher(r#"suffix = ".age""#), "secrets.age"));
        assert!(matcher_hit(&matcher(r#"glob = "**/.env""#), ".env"), "** 可匹配空前缀");
        assert!(matcher_hit(&matcher(r#"glob = "**/.env""#), "/a/b/.env"));
        assert!(!matcher_hit(&matcher(r#"glob = "**/.env""#), "/a/b/env.zsh"));
        assert!(matcher_hit(&matcher(r#"re = "^v\\d+$""#), "v42"));
        // 同匹配器多种类 AND
        let both = matcher("prefix = \"a\"\nsuffix = \"z\"");
        assert!(matcher_hit(&both, "a-to-z"));
        assert!(!matcher_hit(&both, "a-to-b"));
        // 全空匹配器恒真（只断言字段存在）
        assert!(matcher_hit(&Matcher::default(), "anything"));
    }

    #[test]
    fn bad_glob_and_regex_fail_open() -> Result<(), toml::de::Error> {
        let config = Config::from_toml(
            "[[bash]]\nname='x'\ncmd='git'\nargs_re=['(']\ndecision='ask'\nreason='r'",
        )?;
        assert!(check_bash(&config, "git anything").is_none());
        let bad_glob: Matcher = toml::from_str(r#"glob = "[""#).unwrap_or_default();
        assert!(!matcher_hit(&bad_glob, "anything"), "坏 glob 视为不中");
        Ok(())
    }
}
