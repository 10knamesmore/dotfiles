//! 端到端：用**真实生产规则表** `tree/home/.claude/hooks/pretool.toml`（`include_str!` 编译期
//! 内联，改规则即触发本测试重编译）跑真 `cc-hook` 二进制，断言关键命令的决策。
//!
//! 测试直接读取生产规则表，不维护会与生产规则漂移的副本。
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::min_ident_chars,
    clippy::missing_docs_in_private_items
)] // 集成测试惯例

use std::io::Write as _;

use assert_cmd::Command;
use tempfile::NamedTempFile;

/// 生产规则表原文（编译期内联；路径相对 crate 根，改 toml 即重编译本测试）。
const PROD_RULES: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../tree/home/.claude/hooks/pretool.toml"
));

/// 把生产规则落成临时文件供 `--rules` 读取（drop 即清理）。
fn prod_rules_file() -> NamedTempFile {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(PROD_RULES.as_bytes()).unwrap();
    file
}

/// 跑一次 Bash 决策：返回 (decision 或 None=静默, exit 成功)。
fn bash_decision(rules: &std::path::Path, command: &str) -> (Option<String>, bool) {
    let stdin =
        serde_json::json!({"tool_name": "Bash", "tool_input": {"command": command}}).to_string();
    let output = Command::cargo_bin("cc-hook")
        .unwrap()
        .args(["pretool", "--rules"])
        .arg(rules)
        .env("CC_HOOK_AUDIT_LOG", "/dev/null")
        .write_stdin(stdin)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let decision = (!stdout.trim().is_empty())
        .then(|| serde_json::from_str::<serde_json::Value>(&stdout).ok())
        .flatten()
        .and_then(|value| {
            value["hookSpecificOutput"]["permissionDecision"]
                .as_str()
                .map(str::to_owned)
        });
    (decision, output.status.success())
}

#[test]
fn production_rules_parse_and_decide() {
    let rules = prod_rules_file();
    // (命令, 期望决策：None=静默放行)
    let cases: &[(&str, Option<&str>)] = &[
        // ── 既有规则（证明读到的是真生产表，而非合成 fixture）──
        ("rm -rf ~/x", Some("ask")),         // 白名单外需确认
        ("rm -rf /usr", Some("ask")),        // 白名单外需确认
        ("rm -rf /tmp/x", None),             // 白名单 /tmp 内放行
        ("rm -rf /", Some("ask")),           // 根目录需确认
        ("grep foo file.txt", Some("deny")), // prefer-rg（合成 fixture 里没有）
        ("git push origin main", None),      // 未配置 push 守卫，静默放行
        ("git add -A", None),
        // ── chmod 全局可写 ──
        ("chmod 777 file", Some("ask")),
        ("chmod -R 777 dir", Some("ask")),
        ("chmod 0777 file", Some("ask")),
        ("chmod o+w file", Some("ask")),
        ("chmod 644 file", None), // 正常权限位不误伤
        // ── npm → pnpm ──
        ("npm install left-pad", Some("deny")),
        ("npm run build", Some("deny")),
        ("npx prettier --write x", None), // npx 不拦
        ("pnpm install", None),           // pnpm 放行
    ];
    for (command, want) in cases {
        let (decision, ok) = bash_decision(rules.path(), command);
        assert!(ok, "必须 exit 0（fail-open 契约）: {command}");
        assert_eq!(decision.as_deref(), *want, "command: {command}");
    }
}
