//! 端到端：起真实 `cc-hook pretool` 二进制，stdin 喂 hook JSON，断言决策与 fail-open。
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::min_ident_chars,
    clippy::missing_docs_in_private_items
)] // 集成测试惯例

use std::io::Write as _;

use assert_cmd::Command;
use tempfile::NamedTempFile;

/// 生产规则集的等价 fixture（与 tree/home/.claude/hooks/pretool.toml 同步维护）。
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
reason   = "git 推送需要用户确认"

[[bash]]
name     = "git-reset-hard"
cmd      = "git"
subcmd   = "reset"
any      = ["--hard"]
decision = "ask"
reason   = "丢弃改动"

[[tool]]
name     = "gh-not-webfetch-github"
tool     = "WebFetch"
where    = { url = { domain = "github.com" } }
decision = "deny"
reason   = "GitHub 一律 gh CLI"

[[tool]]
name     = "no-dotenv-read"
tool     = "Read"
where    = { file_path = { glob = ["**/.env", "**/.env.*"] } }
decision = "deny"
reason   = ".env 可能含密钥"

[[tool]]
name     = "matcher-kinds-probe"
tool     = "Probe"
where    = { alpha = { prefix = "pre-", suffix = "-end" }, beta = { contains = ["midA", "midB"] } }
decision = "ask"
reason   = "多匹配器 AND / 数组 OR 探针"
"#;

/// 把规则 fixture 落成临时文件，返回句柄（drop 即清理）。
fn rules_file() -> NamedTempFile {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(RULES.as_bytes()).unwrap();
    file
}

/// 跑一次 pretool：返回 (stdout, exit success)。
fn run(rules_path: &std::path::Path, stdin_json: &str) -> (String, bool) {
    let output = Command::cargo_bin("cc-hook")
        .unwrap()
        .args(["pretool", "--rules"])
        .arg(rules_path)
        .write_stdin(stdin_json)
        .output()
        .unwrap();
    (String::from_utf8_lossy(&output.stdout).into_owned(), output.status.success())
}

/// Bash 工具的 hook JSON 信封。
fn bash_envelope(command: &str) -> String {
    serde_json::json!({"tool_name": "Bash", "tool_input": {"command": command}}).to_string()
}

/// 任意工具的 hook JSON 信封。
fn tool_envelope(tool: &str, input: serde_json::Value) -> String {
    serde_json::json!({"tool_name": tool, "tool_input": input}).to_string()
}

/// 断言一次调用的决策（None = 无输出静默放行），统一校验 exit 0 契约。
fn assert_verdict(rules: &NamedTempFile, stdin_json: &str, expected: Option<&str>, label: &str) {
    let (stdout, ok) = run(rules.path(), stdin_json);
    assert!(ok, "必须 exit 0（fail-open 契约）: {label}");
    match expected {
        Some(decision) => {
            let parsed = serde_json::from_str::<serde_json::Value>(&stdout);
            assert!(parsed.is_ok(), "应输出决策 JSON: {label} → {stdout}");
            let value = parsed.unwrap();
            assert_eq!(
                value["hookSpecificOutput"]["permissionDecision"], *decision,
                "case: {label}"
            );
        }
        None => assert!(stdout.is_empty(), "应静默放行: {label} → {stdout}"),
    }
}

#[test]
fn bash_verdict_table_end_to_end() {
    let cases: &[(&str, Option<&str>)] = &[
        ("rm -rf /tmp/foo", Some("deny")),
        ("cd /tmp && rm -fr build", Some("deny")),
        ("command rm -rf x", Some("deny")),
        ("git push origin dev", Some("ask")),
        ("git reset --hard HEAD~1", Some("ask")),
        ("git add -A", None),
        ("rm -r /tmp/foo", None),
        (r#"echo "rm -rf /""#, None),
    ];
    let rules = rules_file();
    for (command, expected) in cases {
        assert_verdict(&rules, &bash_envelope(command), *expected, command);
    }
}

#[test]
fn webfetch_domain_rule_end_to_end() {
    let rules = rules_file();
    // (url, 期望)：域名含子域命中；其他域、相似域放行
    let cases: &[(&str, Option<&str>)] = &[
        ("https://github.com/foo/bar", Some("deny")),
        ("https://www.github.com/foo", Some("deny")),
        ("https://gist.github.com/x/1", Some("deny")),
        ("https://raw.githubusercontent.com/a/b/main/x.txt", None),
        ("https://notgithub.com/foo", None),
        ("https://example.com/github.com/decoy", None),
    ];
    for (url, expected) in cases {
        let stdin = tool_envelope("WebFetch", serde_json::json!({"url": url, "prompt": "读一下"}));
        assert_verdict(&rules, &stdin, *expected, url);
    }
}

#[test]
fn read_glob_rule_end_to_end() {
    let rules = rules_file();
    let cases: &[(&str, Option<&str>)] = &[
        ("/home/u/proj/.env", Some("deny")),
        ("/home/u/proj/.env.local", Some("deny")),
        (".env", Some("deny")),
        ("/home/u/.config/dots/env.zsh", None),
        ("/home/u/proj/environment.md", None),
    ];
    for (path, expected) in cases {
        let stdin = tool_envelope("Read", serde_json::json!({"file_path": path}));
        assert_verdict(&rules, &stdin, *expected, path);
    }
}

#[test]
fn matcher_kinds_and_or_semantics() {
    let rules = rules_file();
    // 同字段多匹配器 AND（prefix 且 suffix）；数组 OR（contains 任一）；字段间 AND
    let cases: &[(serde_json::Value, Option<&str>)] = &[
        (serde_json::json!({"alpha": "pre-x-end", "beta": "has midA inside"}), Some("ask")),
        (serde_json::json!({"alpha": "pre-x-end", "beta": "has midB inside"}), Some("ask")),
        (serde_json::json!({"alpha": "pre-x-end", "beta": "no hit"}), None), // beta 不中
        (serde_json::json!({"alpha": "pre-x", "beta": "midA"}), None),       // suffix 不中
        (serde_json::json!({"beta": "midA"}), None),                          // 字段缺失
    ];
    for (input, expected) in cases {
        let stdin = tool_envelope("Probe", input.clone());
        assert_verdict(&rules, &stdin, *expected, &input.to_string());
    }
}

#[test]
fn unmatched_tool_passes_silently() {
    let rules = rules_file();
    let stdin = tool_envelope("Edit", serde_json::json!({"file_path": "/tmp/x.rs"}));
    assert_verdict(&rules, &stdin, None, "无规则工具");
}

#[test]
fn missing_rules_file_fails_open() {
    let (stdout, ok) =
        run(std::path::Path::new("/nonexistent/rules.toml"), &bash_envelope("rm -rf /"));
    assert!(ok);
    assert!(stdout.is_empty(), "规则缺失必须放行");
}

#[test]
fn broken_rules_file_fails_open_with_stderr_notice() {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(b"not [ valid toml").unwrap();
    let output = Command::cargo_bin("cc-hook")
        .unwrap()
        .args(["pretool", "--rules"])
        .arg(file.path())
        .write_stdin(bash_envelope("rm -rf /"))
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
    for bad in [
        "",
        "not json",
        "{}",
        r#"{"tool_input":{"command":"rm -rf /"}}"#, // 缺 tool_name
        r#"{"tool_name":"Bash","tool_input":{"command":42}}"#,
    ] {
        let (stdout, ok) = run(rules.path(), bad);
        assert!(ok, "stdin: {bad}");
        assert!(stdout.is_empty(), "stdin: {bad}");
    }
}
