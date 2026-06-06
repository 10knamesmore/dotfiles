//! 规则匹配：原始命令串 × 规则集 → 首条命中。

use std::collections::BTreeSet;

use regex_lite::Regex;

use crate::argv;
use crate::rules::{Config, Rule};

/// 对原始命令串逐段（链式命令拆开）逐规则匹配，返回首条命中。
///
/// 全部不中返回 `None`（= 守卫无意见，走正常权限流程）。
#[must_use]
pub fn check<'cfg>(config: &'cfg Config, raw: &str) -> Option<&'cfg Rule> {
    argv::each_command(raw)
        .iter()
        .find_map(|segment| config.rules.iter().find(|rule| hit(rule, segment)))
}

/// 单段 argv 是否命中规则的全部条件（条件间 AND）。
fn hit(rule: &Rule, segment: &[String]) -> bool {
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

    /// 生产规则集的等价 fixture（与 bash-guard.toml 同步维护）。
    const RULES: &str = r#"
[[rules]]
name     = "rm-recursive-force"
cmd      = "rm"
all      = [["-r", "--recursive"], ["-f", "--force"]]
decision = "deny"
reason   = "rm 递归+强制"

[[rules]]
name     = "git-push-force"
cmd      = "git"
subcmd   = "push"
any      = ["-f", "--force", "--force-with-lease"]
decision = "ask"
reason   = "强制推送"

[[rules]]
name     = "git-push-main"
cmd      = "git"
subcmd   = "push"
args_re  = ["^(main|master)$", ":(main|master)$"]
decision = "ask"
reason   = "推主分支"

[[rules]]
name     = "git-reset-hard"
cmd      = "git"
subcmd   = "reset"
any      = ["--hard"]
decision = "ask"
reason   = "丢弃改动"

[[rules]]
name     = "git-clean-force"
cmd      = "git"
subcmd   = "clean"
all      = [["-f", "--force"]]
decision = "ask"
reason   = "丢弃改动"
"#;

    /// (输入命令, 期望命中：None=静默放行)
    const CASES: &[(&str, Option<(&str, Decision)>)] = &[
        // ── deny ──
        ("rm -rf /tmp/foo", Some(("rm-recursive-force", Decision::Deny))),
        ("cd /tmp && rm -fr build", Some(("rm-recursive-force", Decision::Deny))),
        ("cd /tmp\nrm -rf build", Some(("rm-recursive-force", Decision::Deny))),
        ("cat list | rm -rf x", Some(("rm-recursive-force", Decision::Deny))),
        ("command rm -rf x", Some(("rm-recursive-force", Decision::Deny))),
        ("rm --recursive --force x", Some(("rm-recursive-force", Decision::Deny))),
        // ── ask ──
        ("git push -f origin dev", Some(("git-push-force", Decision::Ask))),
        ("git push --force-with-lease origin dev", Some(("git-push-force", Decision::Ask))),
        ("git push origin main", Some(("git-push-main", Decision::Ask))),
        ("git push origin feat:main", Some(("git-push-main", Decision::Ask))),
        ("git reset --hard HEAD~1", Some(("git-reset-hard", Decision::Ask))),
        ("git clean -fd", Some(("git-clean-force", Decision::Ask))),
        // ── 静默放行 ──
        ("git add -A", None),
        ("ls -la && cargo build", None),
        ("rm -r /tmp/foo", None),
        ("rm my-perf-report.txt", None),
        (r#"echo "rm -rf /""#, None),
        ("git push", None),
        ("rm -1 weird", None),
    ];

    #[test]
    fn verdict_table() -> Result<(), toml::de::Error> {
        let config = Config::from_toml(RULES)?;
        for (command, expected) in CASES {
            let verdict =
                check(&config, command).map(|rule| (rule.name.as_str(), rule.decision));
            assert_eq!(verdict, *expected, "command: {command}");
        }
        Ok(())
    }

    #[test]
    fn invalid_regex_in_rule_is_ignored_fail_open() -> Result<(), toml::de::Error> {
        let config = Config::from_toml(
            "[[rules]]\nname='x'\ncmd='git'\nargs_re=['(']\ndecision='ask'\nreason='r'",
        )?;
        assert!(check(&config, "git anything").is_none());
        Ok(())
    }
}
