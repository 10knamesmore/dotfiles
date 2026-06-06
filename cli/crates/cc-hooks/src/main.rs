//! cc-hook：Claude Code hooks 统一入口。
//!
//! 子命令 = 各 hook 用途；协议共性（stdin JSON、`hookSpecificOutput`、fail-open）住 lib。
//! bin 只做 clap 分发与 stdin/stdout 管道，**任何失败都静默放行（exit 0）**。

use std::io::Read;
use std::path::PathBuf;

use clap::{Parser, Subcommand};

/// CLI 入口定义。
#[derive(Parser)]
#[command(name = "cc-hook", about = "Claude Code hooks 统一入口", version)]
struct Cli {
    /// hook 用途子命令
    #[command(subcommand)]
    command: Command,
}

/// 各 hook 用途，一个子命令一份独立逻辑与配置。
#[derive(Subcommand)]
enum Command {
    /// PreToolUse(Bash)：按规则表拦截高危命令（deny/ask），其余静默放行
    BashGuard {
        /// 规则 TOML 路径（缺省 ~/.claude/hooks/bash-guard.toml）
        #[arg(long)]
        rules: Option<PathBuf>,
    },
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::BashGuard { rules } => bash_guard(rules),
    }
}

/// bash-guard 管道：读规则 → 读 stdin → 引擎判定 → 渲染决策。
///
/// 配置缺失/stdin 坏掉 → 直接返回（放行）；规则文件存在但解析失败 →
/// stderr 留一句话（claude --debug 可见）再放行，避免守卫静默失效无人知。
fn bash_guard(rules_path: Option<PathBuf>) {
    let Some(path) = rules_path.or_else(default_rules_path) else {
        return;
    };
    let Ok(text) = std::fs::read_to_string(&path) else {
        return;
    };
    let config = match cc_hooks::rules::Config::from_toml(&text) {
        Ok(config) => config,
        Err(error) => {
            eprintln!("cc-hook bash-guard: 规则解析失败（已放行）{}: {error}", path.display());
            return;
        }
    };

    let mut stdin_text = String::new();
    if std::io::stdin().read_to_string(&mut stdin_text).is_err() {
        return;
    }
    let Some(command) = cc_hooks::envelope::extract_command(&stdin_text) else {
        return;
    };

    if let Some(rule) = cc_hooks::engine::check(&config, &command) {
        println!("{}", cc_hooks::envelope::render(rule));
    }
}

/// 缺省规则路径：`$HOME/.claude/hooks/bash-guard.toml`。
fn default_rules_path() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".claude/hooks/bash-guard.toml"))
}
