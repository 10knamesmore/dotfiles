//! 端到端：起真实 `agent-hook kimi-pretool`，验证 Kimi Code PreToolUse 协议、
//! ask 降级语义与 FetchURL → WebFetch 工具名映射。
#![allow(clippy::min_ident_chars, clippy::missing_docs_in_private_items)] // 集成测试只暴露测试入口，不构成 crate API。

use std::io::Write as _;

use assert_cmd::Command;
use tempfile::NamedTempFile;

/// 覆盖 Kimi Code 可直接表达的 deny、无法表达的 ask、FetchURL 映射，以及静默放行。
const RULES: &str = r#"
[[bash]]
name     = "rm-recursive"
cmd      = "rm"
all      = [["-r", "-R", "--recursive"]]
decision = "deny"
reason   = "rm 递归删除"

[[bash]]
name     = "git-push"
cmd      = "git"
subcmd   = "push"
decision = "ask"
reason   = "git 推送需要用户确认"

[[tool]]
name     = "no-webfetch-example"
tool     = "WebFetch"
where    = { url = { domain = "example.com" } }
decision = "deny"
reason   = "example.com 禁止抓取"
"#;

/// 仓库中的真实生产规则；Kimi Code adapter 必须与 Claude 共用这份语义。
const PROD_RULES: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../tree/home/.claude/hooks/pretool.toml"
));

/// 把规则 fixture 写入独立临时文件。
fn rules_file(contents: &str) -> Result<NamedTempFile, Box<dyn std::error::Error>> {
    let mut file = NamedTempFile::new()?;
    file.write_all(contents.as_bytes())?;
    Ok(file)
}

/// 构造 Kimi Code Bash PreToolUse 信封。
fn bash_envelope(command: &str) -> String {
    serde_json::json!({
        "session_id": "thr_test",
        "hook_event_name": "PreToolUse",
        "tool_name": "Bash",
        "tool_input": { "command": command }
    })
    .to_string()
}

/// 构造 Kimi Code FetchURL PreToolUse 信封。
fn fetch_url_envelope(url: &str) -> String {
    serde_json::json!({
        "session_id": "thr_test",
        "hook_event_name": "PreToolUse",
        "tool_name": "FetchURL",
        "tool_input": { "url": url }
    })
    .to_string()
}

/// 运行 Kimi Code adapter 并返回 stdout。
fn run(
    rules_path: &std::path::Path,
    stdin_json: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let output = Command::cargo_bin("agent-hook")?
        .args(["kimi-pretool", "--rules"])
        .arg(rules_path)
        .env("AGENT_HOOK_AUDIT_LOG", "/dev/null")
        .write_stdin(stdin_json)
        .output()?;
    assert!(
        output.status.success(),
        "Kimi Code hook 必须保持 fail-open exit 0"
    );
    Ok(String::from_utf8(output.stdout)?)
}

/// 提取 Kimi Code PreToolUse 的 permissionDecision 与理由。
fn decision(stdout: &str) -> Result<Option<(String, String)>, serde_json::Error> {
    if stdout.trim().is_empty() {
        return Ok(None);
    }
    let value: serde_json::Value = serde_json::from_str(stdout)?;
    let output = &value["hookSpecificOutput"];
    Ok(output["permissionDecision"]
        .as_str()
        .zip(output["permissionDecisionReason"].as_str())
        .map(|(decision, reason)| (decision.to_owned(), reason.to_owned())))
}

#[test]
fn deny_keeps_kimi_supported_shape() -> Result<(), Box<dyn std::error::Error>> {
    let rules = rules_file(RULES)?;
    let stdout = run(rules.path(), &bash_envelope("rm -rf ~/build"))?;
    assert_eq!(
        decision(&stdout)?,
        Some(("deny".to_owned(), "rm 递归删除".to_owned())),
        "Kimi Code 支持的 deny 应保持原规则理由"
    );
    Ok(())
}

#[test]
fn ask_degrades_to_explicit_deny() -> Result<(), Box<dyn std::error::Error>> {
    let rules = rules_file(RULES)?;
    let stdout = run(rules.path(), &bash_envelope("git push origin main"))?;
    let Some((actual, reason)) = decision(&stdout)? else {
        return Err("ask 不能静默放行".into());
    };
    assert_eq!(actual, "deny");
    assert!(
        reason.contains("Kimi Code PreToolUse 暂不支持 ask"),
        "降级理由应解释为何 hard deny: {reason}"
    );
    assert!(
        reason.contains("git 推送需要用户确认"),
        "降级理由应保留原规则语义: {reason}"
    );
    Ok(())
}

#[test]
fn fetch_url_matches_shared_webfetch_rule() -> Result<(), Box<dyn std::error::Error>> {
    let rules = rules_file(RULES)?;
    let stdout = run(
        rules.path(),
        &fetch_url_envelope("https://example.com/page"),
    )?;
    assert_eq!(
        decision(&stdout)?,
        Some(("deny".to_owned(), "example.com 禁止抓取".to_owned())),
        "FetchURL 应改写为 WebFetch 后命中共享规则"
    );
    Ok(())
}

#[test]
fn production_webfetch_rule_covers_fetch_url() -> Result<(), Box<dyn std::error::Error>> {
    let rules = rules_file(PROD_RULES)?;
    let stdout = run(
        rules.path(),
        &fetch_url_envelope("https://github.com/owner/repo"),
    )?;
    let Some((actual, reason)) = decision(&stdout)? else {
        return Err("生产 WebFetch 规则应覆盖 Kimi 的 FetchURL".into());
    };
    assert_eq!(actual, "deny");
    assert!(reason.contains("gh CLI"));
    Ok(())
}

#[test]
fn unmatched_command_stays_silent() -> Result<(), Box<dyn std::error::Error>> {
    let rules = rules_file(RULES)?;
    let stdout = run(rules.path(), &bash_envelope("git status --short"))?;
    assert!(stdout.trim().is_empty());
    Ok(())
}

#[test]
fn malformed_stdin_fails_open() -> Result<(), Box<dyn std::error::Error>> {
    let rules = rules_file(RULES)?;
    let stdout = run(rules.path(), "这不是 JSON")?;
    assert!(stdout.trim().is_empty(), "坏 stdin 必须静默放行: {stdout}");
    Ok(())
}
