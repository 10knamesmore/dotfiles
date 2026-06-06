//! CLI 定义与分发（clap derive）。

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use clap_complete::Shell;
use dots_core::LinkMode;

use crate::cmd;

/// 通用 Result 别名。
pub type Result<T> = color_eyre::Result<T>;

/// dots —— 用软链接把仓库里的配置同步到这台机器（改仓库即生效）。
#[derive(Debug, Parser)]
#[command(name = "dots", version, about, long_about = None)]
pub struct Args {
    /// 要执行的命令。
    #[command(subcommand)]
    pub command: Command,
}

/// dots 的命令。
#[derive(Debug, Subcommand)]
pub enum Command {
    /// 把仓库配置链接到 home，让改动生效（幂等）。
    Sync {
        /// 只看会做什么，不实际改动。
        #[arg(long)]
        dry_run: bool,
    },
    /// 看看哪些配置已生效、哪些没链上、哪些被改乱了。
    Status,
    /// 把 home 里现成的 dotfile 收进仓库管理。
    Adopt {
        /// 要收进来的文件路径（可多个）。
        paths: Vec<PathBuf>,
        /// 放到哪一层：home=通用，home.linux / home.macos=平台专属。
        #[arg(long)]
        layer: Option<String>,
        /// dir=整个目录，children=逐个子项，file=逐个文件。
        #[arg(long)]
        mode: Option<String>,
    },
    /// 撤销上一次 adopt 或 unlink。
    Undo,
    /// 不再管某个文件，把它还回家目录。
    Unlink {
        /// 要取消管理的文件（家目录侧）。
        path: PathBuf,
        /// 只断开链接，仓库里的副本保留。
        #[arg(long)]
        keep_in_repo: bool,
    },
    /// 体检：找出链接、本机配置、脚本里的问题（只读）。
    Doctor,
    /// 管理加密保存的密码等敏感信息。
    Secret {
        /// secret 的子命令。
        #[command(subcommand)]
        action: SecretCmd,
    },
    /// 新机器一键装好：装软件包和工具链，再同步配置。
    Bootstrap,
    /// 生成 shell 自动补全脚本。
    Completions {
        /// 目标 shell（bash / zsh / fish…）。
        shell: Shell,
    },
}

/// secret 的子命令。
#[derive(Debug, Subcommand)]
pub enum SecretCmd {
    /// 新增或修改一条 secret（输入时不回显）。
    Set {
        /// secret 的名字。
        key: String,
    },
    /// 列出已有的 secret 名字（不显示内容）。
    List,
}

/// 解析 `mode` 字符串。
fn parse_mode(text: &Option<String>) -> Option<LinkMode> {
    match text.as_deref() {
        Some("children") => Some(LinkMode::Children),
        Some("file") => Some(LinkMode::File),
        Some("dir") => Some(LinkMode::Dir),
        _ => None,
    }
}

/// 解析参数并分发执行。
pub fn run() -> Result<ExitCode> {
    let args = Args::parse();
    match args.command {
        Command::Sync { dry_run } => {
            cmd::sync::run(dry_run)?;
            Ok(ExitCode::SUCCESS)
        }
        Command::Status => {
            let green = cmd::status::run()?;
            Ok(if green {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            })
        }
        Command::Adopt { paths, layer, mode } => {
            cmd::adopt::run(
                &paths,
                &cmd::adopt::AdoptOpts {
                    layer,
                    mode: parse_mode(&mode),
                },
            )?;
            Ok(ExitCode::SUCCESS)
        }
        Command::Undo => {
            cmd::undo::run()?;
            Ok(ExitCode::SUCCESS)
        }
        Command::Unlink { path, keep_in_repo } => {
            cmd::unlink::run(&path, keep_in_repo)?;
            Ok(ExitCode::SUCCESS)
        }
        Command::Doctor => {
            let green = cmd::doctor::run()?;
            Ok(if green {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            })
        }
        Command::Secret { action } => {
            let action = match action {
                SecretCmd::Set { key } => cmd::secret::SecretAction::Set { key },
                SecretCmd::List => cmd::secret::SecretAction::List,
            };
            cmd::secret::run(&action)?;
            Ok(ExitCode::SUCCESS)
        }
        Command::Bootstrap => {
            cmd::bootstrap::run()?;
            Ok(ExitCode::SUCCESS)
        }
        Command::Completions { shell } => {
            cmd::completions::run(shell)?;
            Ok(ExitCode::SUCCESS)
        }
    }
}
