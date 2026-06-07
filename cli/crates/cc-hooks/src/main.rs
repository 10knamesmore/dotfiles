//! cc-hook：Claude Code hooks 统一入口。
//!
//! 子命令 = CC 生命周期事件（pretool → PreToolUse；将来 posttool/stop 同理）；
//! 事件内的工具差异全部下沉到规则 TOML。业务函数不做 IO，统一返回
//! [`cc_hooks::outcome::HookRun`]，由 [`wire`] 落地（stdout/stderr/exit code）。
//! **任何失败都静默放行（exit 0）**——fail-open 铁律。

use std::io::Read;
use std::path::PathBuf;

use cc_hooks::envelope::{self, PreToolUseOutput};
use cc_hooks::outcome::HookRun;
use cc_hooks::{engine, rules};
use clap::{Parser, Subcommand};
use serde::Serialize;

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
    match cli.command {
        Command::Pretool { rules } => wire(pretool(rules)),
    }
}

/// 统一落地：notice → stderr，output → 序列化进 stdout，code → 退出码。
///
/// 序列化失败吞掉（fail-open：宁可静默放行也不输出半截 JSON）。
fn wire<T: Serialize>(run: HookRun<T>) {
    if let Some(notice) = run.notice {
        eprintln!("{notice}");
    }
    if let Some(output) = run.output
        && let Ok(line) = serde_json::to_string(&output)
    {
        println!("{line}");
    }
    if run.code != 0 {
        std::process::exit(run.code);
    }
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
    let Some((tool_name, tool_input)) = envelope::parse_pretool(&stdin_text) else {
        return HookRun::silent();
    };

    // Bash：argv 引擎（词法/旗标簇）；命中即返回
    if tool_name == "Bash"
        && let Some(command) = tool_input
            .get("command")
            .and_then(serde_json::Value::as_str)
        && let Some(rule) = engine::check_bash(&config, command)
    {
        return HookRun::decision(PreToolUseOutput::new(rule.decision, &rule.reason));
    }

    // 通用工具规则（含 Bash 的非 command 字段场景）
    if let Some(rule) = engine::check_tool(&config, &tool_name, &tool_input) {
        return HookRun::decision(PreToolUseOutput::new(rule.decision, &rule.reason));
    }
    HookRun::silent()
}

/// 缺省规则路径：`$HOME/.claude/hooks/pretool.toml`。
fn default_rules_path() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".claude/hooks/pretool.toml"))
}
