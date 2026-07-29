//! agent-hook：多个 coding agent 共用的本地守卫入口。
//!
//! 当前提供 Codex 与 Kimi Code 的 PreToolUse adapter；共享判定逻辑位于
//! `cc_hooks::pretool::evaluation`。任何读取或解析失败都静默放行，避免损坏
//! agent 的正常工具调用。

use std::io::Read as _;
use std::path::PathBuf;

use cc_hooks::common::outcome::HookRun;
use cc_hooks::common::wire;
use cc_hooks::pretool::codex::PreToolUseOutput as CodexPreToolUseOutput;
use cc_hooks::pretool::evaluation;
use cc_hooks::pretool::kimi::{self, PreToolUseOutput as KimiPreToolUseOutput};
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
    /// Kimi Code PreToolUse：同 Codex 语义，额外把 FetchURL 改写为 WebFetch。
    KimiPretool {
        /// 规则 TOML；缺省 `~/.kimi-code/pretool.toml`。
        #[arg(long)]
        rules: Option<PathBuf>,
    },
}

/// 解析 CLI 并运行对应 adapter。
fn main() {
    let cli = Cli::parse();
    let audit_path = audit_log_path();
    match cli.command {
        Command::CodexPretool { rules } => wire::emit(
            codex_pretool(rules.or_else(default_codex_rules_path)),
            audit_path.as_deref(),
        ),
        Command::KimiPretool { rules } => wire::emit(
            kimi_pretool(rules.or_else(default_kimi_rules_path)),
            audit_path.as_deref(),
        ),
    }
}

/// 读取共享规则并生成 Codex PreToolUse 结果。
fn codex_pretool(rules_path: Option<PathBuf>) -> HookRun<CodexPreToolUseOutput> {
    let Some(path) = rules_path else {
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
    HookRun::decision(CodexPreToolUseOutput::new(
        matched.decision,
        &matched.reason,
    ))
    .with_audit(matched.audit("deny"))
}

/// 读取共享规则并生成 Kimi Code PreToolUse 结果。
fn kimi_pretool(rules_path: Option<PathBuf>) -> HookRun<KimiPreToolUseOutput> {
    let Some(path) = rules_path else {
        return HookRun::silent();
    };
    let Ok(text) = std::fs::read_to_string(&path) else {
        return HookRun::silent();
    };
    let config = match rules::Config::from_toml(&text) {
        Ok(config) => config,
        Err(error) => {
            return HookRun::silent().with_notice(format!(
                "agent-hook kimi-pretool: 规则解析失败（已放行）{}: {error}",
                path.display()
            ));
        }
    };
    let mut stdin_text = String::new();
    if std::io::stdin().read_to_string(&mut stdin_text).is_err() {
        return HookRun::silent();
    }
    let stdin_text = kimi::rewrite_stdin(&stdin_text);
    let Some(matched) = evaluation::evaluate(&config, &stdin_text) else {
        return HookRun::silent();
    };
    HookRun::decision(KimiPreToolUseOutput::new(matched.decision, &matched.reason))
        .with_audit(matched.audit("deny"))
}

/// 返回 Codex adapter 的缺省共享规则路径。
fn default_codex_rules_path() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".codex/pretool.toml"))
}

/// 返回 Kimi Code adapter 的缺省共享规则路径。
fn default_kimi_rules_path() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".kimi-code/pretool.toml"))
}

/// 返回审计日志路径（各 adapter 共用）。
fn audit_log_path() -> Option<PathBuf> {
    if let Some(custom) = std::env::var_os("AGENT_HOOK_AUDIT_LOG") {
        return Some(PathBuf::from(custom));
    }
    std::env::var_os("HOME")
        .map(|home| PathBuf::from(home).join(".local/state/agent-hook/audit.log"))
}
