//! agent-hook：多个 coding agent 共用的本地守卫入口。
//!
//! 当前提供 Codex PreToolUse adapter；共享判定逻辑位于 `cc_hooks::pretool::evaluation`。
//! 任何读取或解析失败都静默放行，避免损坏 agent 的正常工具调用。

use std::io::Read as _;
use std::path::PathBuf;

use cc_hooks::common::outcome::HookRun;
use cc_hooks::common::wire;
use cc_hooks::pretool::codex::PreToolUseOutput;
use cc_hooks::pretool::evaluation;
use cc_hooks::pretool::rules;
use clap::{Parser, Subcommand};

/// 中立 hook CLI。
#[derive(Parser)]
#[command(
    name = "agent-hook",
    about = "Coding agent 共用的本地守卫入口",
    version
)]
struct Cli {
    /// harness 对应的 hook 事件。
    #[command(subcommand)]
    command: Command,
}

/// 当前支持的 harness adapter。
#[derive(Subcommand)]
enum Command {
    /// Codex PreToolUse：共享 deny 原样拦截，ask 降级为 hard deny。
    CodexPretool {
        /// 规则 TOML；缺省 `~/.codex/pretool.toml`。
        #[arg(long)]
        rules: Option<PathBuf>,
    },
}

/// 解析 CLI 并运行对应 adapter。
fn main() {
    let cli = Cli::parse();
    let audit_path = audit_log_path();
    match cli.command {
        Command::CodexPretool { rules } => wire::emit(codex_pretool(rules), audit_path.as_deref()),
    }
}

/// 读取共享规则并生成 Codex PreToolUse 结果。
fn codex_pretool(rules_path: Option<PathBuf>) -> HookRun<PreToolUseOutput> {
    let Some(path) = rules_path.or_else(default_rules_path) else {
        return HookRun::silent();
    };
    let Ok(text) = std::fs::read_to_string(&path) else {
        return HookRun::silent();
    };
    let config = match rules::Config::from_toml(&text) {
        Ok(config) => config,
        Err(error) => {
            return HookRun::silent().with_notice(format!(
                "agent-hook codex-pretool: 规则解析失败（已放行）{}: {error}",
                path.display()
            ));
        }
    };
    let mut stdin_text = String::new();
    if std::io::stdin().read_to_string(&mut stdin_text).is_err() {
        return HookRun::silent();
    }
    let Some(matched) = evaluation::evaluate(&config, &stdin_text) else {
        return HookRun::silent();
    };
    HookRun::decision(PreToolUseOutput::new(matched.decision, &matched.reason))
        .with_audit(matched.audit("deny"))
}

/// 返回 Codex adapter 的缺省共享规则路径。
fn default_rules_path() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".codex/pretool.toml"))
}

/// 返回 Codex adapter 的审计日志路径。
fn audit_log_path() -> Option<PathBuf> {
    if let Some(custom) = std::env::var_os("AGENT_HOOK_AUDIT_LOG") {
        return Some(PathBuf::from(custom));
    }
    std::env::var_os("HOME")
        .map(|home| PathBuf::from(home).join(".local/state/agent-hook/audit.log"))
}
