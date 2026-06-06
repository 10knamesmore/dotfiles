//! 端到端：起真实 `cc-hook bash-guard` 二进制，stdin 喂 hook JSON，断言三档决策与 fail-open。
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::min_ident_chars,
    clippy::missing_docs_in_private_items
)] // 集成测试惯例

use std::io::Write as _;

use assert_cmd::Command;
use tempfile::NamedTempFile;

/// 生产规则集的等价 fixture（与 tree/home/.claude/hooks/bash-guard.toml 同步维护）。
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

/// 把规则 fixture 落成临时文件，返回句柄（drop 即清理）。
fn rules_file() -> NamedTempFile {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(RULES.as_bytes()).unwrap();
    file
}

/// 跑一次 bash-guard：返回 (stdout, exit success)。
fn run(rules_path: &std::path::Path, stdin_json: &str) -> (String, bool) {
    let output = Command::cargo_bin("cc-hook")
        .unwrap()
        .args(["bash-guard", "--rules"])
        .arg(rules_path)
        .write_stdin(stdin_json)
        .output()
        .unwrap();
    (String::from_utf8_lossy(&output.stdout).into_owned(), output.status.success())
}

/// 包一层 hook JSON 信封。
fn envelope(command: &str) -> String {
    serde_json::json!({"tool_input": {"command": command}}).to_string()
}

#[test]
fn verdict_table_end_to_end() {
    // (命令, 期望 permissionDecision；None = 无输出静默放行)
    let cases: &[(&str, Option<&str>)] = &[
        ("rm -rf /tmp/foo", Some("deny")),
        ("cd /tmp && rm -fr build", Some("deny")),
        ("cd /tmp\nrm -rf build", Some("deny")),
        ("cat list | rm -rf x", Some("deny")),
        ("command rm -rf x", Some("deny")),
        ("rm --recursive --force x", Some("deny")),
        ("git push -f origin dev", Some("ask")),
        ("git push --force-with-lease origin dev", Some("ask")),
        ("git push origin main", Some("ask")),
        ("git push origin feat:main", Some("ask")),
        ("git reset --hard HEAD~1", Some("ask")),
        ("git clean -fd", Some("ask")),
        ("git add -A", None),
        ("ls -la && cargo build", None),
        ("rm -r /tmp/foo", None),
        ("rm my-perf-report.txt", None),
        (r#"echo "rm -rf /""#, None),
        ("git push", None),
        ("rm -1 weird", None),
    ];

    let rules = rules_file();
    for (command, expected) in cases {
        let (stdout, ok) = run(rules.path(), &envelope(command));
        assert!(ok, "必须 exit 0（fail-open 契约）: {command}");
        match expected {
            Some(decision) => {
                let parsed = serde_json::from_str::<serde_json::Value>(&stdout);
                assert!(parsed.is_ok(), "应输出决策 JSON: {command} → {stdout}");
                let value = parsed.unwrap();
                assert_eq!(
                    value["hookSpecificOutput"]["permissionDecision"], *decision,
                    "command: {command}"
                );
            }
            None => assert!(stdout.is_empty(), "应静默放行: {command} → {stdout}"),
        }
    }
}

#[test]
fn missing_rules_file_fails_open() {
    let (stdout, ok) = run(std::path::Path::new("/nonexistent/rules.toml"), &envelope("rm -rf /"));
    assert!(ok);
    assert!(stdout.is_empty(), "规则缺失必须放行");
}

#[test]
fn broken_rules_file_fails_open_with_stderr_notice() {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(b"not [ valid toml").unwrap();
    let output = Command::cargo_bin("cc-hook")
        .unwrap()
        .args(["bash-guard", "--rules"])
        .arg(file.path())
        .write_stdin(envelope("rm -rf /"))
        .output()
        .unwrap();
    assert!(output.status.success(), "坏规则也必须 exit 0");
    assert!(output.stdout.is_empty(), "坏规则不输出决策");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("规则解析失败"),
        "stderr 必须留痕，防守卫静默失效"
    );
}

#[test]
fn malformed_stdin_fails_open() {
    let rules = rules_file();
    for bad in ["", "not json", "{}", r#"{"tool_input":{"command":42}}"#] {
        let (stdout, ok) = run(rules.path(), bad);
        assert!(ok, "stdin: {bad}");
        assert!(stdout.is_empty(), "stdin: {bad}");
    }
}
