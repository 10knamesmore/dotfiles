//! 多个 coding agent 共用的本地 PreToolUse 守卫入口。

use std::io::Read as _;
use std::path::PathBuf;

use agent_hooks::common::outcome::HookRun;
use agent_hooks::common::wire;
use agent_hooks::pretool::envelope::PreToolUseOutput;
use agent_hooks::pretool::rules::Decision;
use agent_hooks::pretool::{claude, codex, evaluation, kimi, rules};
use clap::{Parser, Subcommand};

/// 中立 hook CLI。
#[derive(Parser)]
#[command(
    name = "agent-hook",
    about = "Coding agent 共用的启发式工具调用守卫",
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
    /// Claude Code PreToolUse：保留共享规则的原生 deny/ask 决策。
    #[command(name = "claude-pretool")]
    Claude {
        /// 规则 TOML；缺省 `~/.claude/hooks/pretool.toml`。
        #[arg(long)]
        rules: Option<PathBuf>,
    },
    /// Codex PreToolUse：共享 deny 原样拦截，ask 降级为 hard deny。
    #[command(name = "codex-pretool")]
    Codex {
        /// 规则 TOML；缺省 `~/.codex/pretool.toml`。
        #[arg(long)]
        rules: Option<PathBuf>,
    },
    /// Kimi Code PreToolUse：ask 降级为 deny，并把 FetchURL 改写为 WebFetch。
    #[command(name = "kimi-pretool")]
    Kimi {
        /// 规则 TOML；缺省 `~/.kimi-code/pretool.toml`。
        #[arg(long)]
        rules: Option<PathBuf>,
    },
}

/// 支持的 agent harness 及其协议差异。
#[derive(Clone, Copy)]
enum Adapter {
    /// Claude Code adapter。
    Claude,
    /// Codex adapter。
    Codex,
    /// Kimi Code adapter。
    Kimi,
}

impl Adapter {
    /// 返回诊断中使用的 CLI 子命令名。
    const fn command_name(self) -> &'static str {
        match self {
            Self::Claude => "claude-pretool",
            Self::Codex => "codex-pretool",
            Self::Kimi => "kimi-pretool",
        }
    }

    /// 返回当前 harness 的缺省规则副本路径。
    fn default_rules_path(self) -> Option<PathBuf> {
        let suffix = match self {
            Self::Claude => ".claude/hooks/pretool.toml",
            Self::Codex => ".codex/pretool.toml",
            Self::Kimi => ".kimi-code/pretool.toml",
        };
        std::env::var_os("HOME").map(|home| PathBuf::from(home).join(suffix))
    }

    /// 把 harness 输入规范化为共享规则使用的工具名。
    fn rewrite_stdin(self, stdin_text: &str) -> String {
        match self {
            Self::Kimi => kimi::rewrite_stdin(stdin_text),
            Self::Claude | Self::Codex => stdin_text.to_owned(),
        }
    }

    /// 把共享规则决策转换为当前 harness 的输出协议。
    fn output(self, decision: Decision, reason: &str) -> PreToolUseOutput {
        match self {
            Self::Claude => claude::output(decision, reason),
            Self::Codex => codex::output(decision, reason),
            Self::Kimi => kimi::output(decision, reason),
        }
    }
}

/// 解析 CLI 并运行对应 adapter。
fn main() {
    let cli = Cli::parse();
    let (adapter, rules_path) = match cli.command {
        Command::Claude { rules } => (Adapter::Claude, rules),
        Command::Codex { rules } => (Adapter::Codex, rules),
        Command::Kimi { rules } => (Adapter::Kimi, rules),
    };
    let rules_path = rules_path.or_else(|| adapter.default_rules_path());
    wire::emit(pretool(adapter, rules_path));
}

/// 读取共享规则并生成指定 harness 的 PreToolUse 结果。
fn pretool(adapter: Adapter, rules_path: Option<PathBuf>) -> HookRun<PreToolUseOutput> {
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
                "agent-hook {}: 规则解析失败（已放行）{}: {error}",
                adapter.command_name(),
                path.display()
            ));
        }
    };
    let mut stdin_text = String::new();
    if std::io::stdin().read_to_string(&mut stdin_text).is_err() {
        return HookRun::silent();
    }
    let stdin_text = adapter.rewrite_stdin(&stdin_text);
    let Some(matched) = evaluation::evaluate(&config, &stdin_text) else {
        return HookRun::silent();
    };
    HookRun::decision(adapter.output(matched.decision, &matched.reason))
}
