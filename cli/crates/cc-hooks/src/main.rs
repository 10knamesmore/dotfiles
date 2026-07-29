//! cc-hook：Claude Code hooks 统一入口。
//!
//! 子命令 = CC 生命周期事件（pretool → PreToolUse；将来 posttool/stop 同理）；
//! 事件内的工具差异全部下沉到规则 TOML。业务函数不做 IO，统一返回
//! [`cc_hooks::outcome::HookRun`]，由 [`wire`] 落地（stdout/stderr/exit code）。
//! **任何失败都静默放行（exit 0）**——fail-open 铁律。

use std::io::Read as _;
use std::path::PathBuf;

use cc_hooks::common::outcome::HookRun;
use cc_hooks::common::wire;
use cc_hooks::pretool::envelope::PreToolUseOutput;
use cc_hooks::pretool::{evaluation, rules};
use clap::{Parser, Subcommand};

/// CLI 入口定义。
#[derive(Parser)]
#[command(name = "cc-hook", about = "Claude Code hooks 统一入口", version)]
struct Cli {
    /// hook 事件子命令
    #[command(subcommand)]
    command: Command,
}

/// 各 hook 事件，一个子命令一份配置。
#[derive(Subcommand)]
enum Command {
    /// PreToolUse（matcher `*`）：按规则表拦截（deny/ask），其余静默放行
    Pretool {
        /// 规则 TOML 路径（缺省 ~/.claude/hooks/pretool.toml）
        #[arg(long)]
        rules: Option<PathBuf>,
    },
}

fn main() {
    let cli = Cli::parse();
    let audit_path = audit_log_path();
    match cli.command {
        Command::Pretool { rules } => wire::emit(pretool(rules), audit_path.as_deref()),
    }
}

/// 审计日志路径：`CC_HOOK_AUDIT_LOG` 优先（可指向 /dev/null 禁用），否则 `$HOME/.claude/cc-hook.log`。
fn audit_log_path() -> Option<PathBuf> {
    if let Some(custom) = std::env::var_os("CC_HOOK_AUDIT_LOG") {
        return Some(PathBuf::from(custom));
    }
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".claude/cc-hook.log"))
}

/// pretool 业务：读规则 → 解析负载 → Bash 走 argv 引擎，其余工具走字段匹配器。
///
/// 配置缺失/stdin 坏掉 → 静默放行；规则文件存在但解析失败 → 留痕后放行，
/// 避免守卫静默失效无人知。
fn pretool(rules_path: Option<PathBuf>) -> HookRun<PreToolUseOutput> {
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
                "cc-hook pretool: 规则解析失败（已放行）{}: {error}",
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
        .with_audit(matched.audit(matched.decision.as_str()))
}

/// 缺省规则路径：`$HOME/.claude/hooks/pretool.toml`。
fn default_rules_path() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".claude/hooks/pretool.toml"))
}
