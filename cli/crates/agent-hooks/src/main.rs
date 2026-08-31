//! 多个 coding agent 共用的本地 PreToolUse 守卫入口。

use std::io::Read as _;
use std::path::PathBuf;

use agent_hooks::common::outcome::HookRun;
use agent_hooks::common::wire;
use agent_hooks::pretool::envelope::PreToolUseOutput;
use agent_hooks::pretool::rules::Decision;
use agent_hooks::pretool::{claude, codex, evaluation, kimi, pi, rules};
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
    /// Pi tool_call：保留 deny/ask，由 extension 处理交互确认。
    #[command(name = "pi-pretool")]
    Pi {
        /// 规则 TOML；缺省 `~/.pi/agent/pretool.toml`。
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
    match cli.command {
        Command::Claude { rules } => run_harness_pretool(Adapter::Claude, rules),
        Command::Codex { rules } => run_harness_pretool(Adapter::Codex, rules),
        Command::Kimi { rules } => run_harness_pretool(Adapter::Kimi, rules),
        Command::Pi { rules } => wire::emit(pi_pretool(rules.or_else(pi_rules_path))),
    }
}

/// 运行输出 Claude-style PreToolUse 信封的 harness adapter。
fn run_harness_pretool(adapter: Adapter, rules_path: Option<PathBuf>) {
    let rules_path = rules_path.or_else(|| adapter.default_rules_path());
    wire::emit(pretool(adapter, rules_path));
}

/// 返回 Pi adapter 的缺省规则副本路径。
fn pi_rules_path() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".pi/agent/pretool.toml"))
}

/// 读取共享规则并返回 Pi extension 的中立决策。
fn pi_pretool(rules_path: Option<PathBuf>) -> HookRun<pi::PiPreToolOutput> {
    let Some(path) = rules_path else {
        return HookRun::decision(pi::PiPreToolOutput::allow())
            .with_notice("agent-hook pi-pretool: HOME 不可用，未加载规则（已放行）".to_owned());
    };
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) => {
            return HookRun::decision(pi::PiPreToolOutput::allow()).with_notice(format!(
                "agent-hook pi-pretool: 规则读取失败（已放行）{}: {error}",
                path.display()
            ));
        }
    };
    let config = match rules::Config::from_toml(&text) {
        Ok(config) => config,
        Err(error) => {
            return HookRun::decision(pi::PiPreToolOutput::allow()).with_notice(format!(
                "agent-hook pi-pretool: 规则解析失败（已放行）{}: {error}",
                path.display()
            ));
        }
    };
    let mut stdin_text = String::new();
    if let Err(error) = std::io::stdin().read_to_string(&mut stdin_text) {
        return HookRun::decision(pi::PiPreToolOutput::allow()).with_notice(format!(
            "agent-hook pi-pretool: stdin 读取失败（已放行）: {error}"
        ));
    }
    let Some(stdin_text) = pi::rewrite_stdin(&stdin_text) else {
        return HookRun::decision(pi::PiPreToolOutput::allow())
            .with_notice("agent-hook pi-pretool: 调用信封解析失败（已放行）".to_owned());
    };
    let Some(matched) = evaluation::evaluate(&config, &stdin_text) else {
        return HookRun::decision(pi::PiPreToolOutput::allow());
    };
    HookRun::decision(pi::PiPreToolOutput::from_decision(
        matched.decision,
        &matched.reason,
    ))
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
