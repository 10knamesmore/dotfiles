//! CLI 定义与分发（clap derive）。

use std::process::ExitCode;

use clap::{Parser, Subcommand};

use crate::cmd;

/// 通用 Result 别名。
pub type Result<T> = color_eyre::Result<T>;

/// dots —— 收敛仓库声明的配置 Resource，并执行 Cargo binary 安装。
#[derive(Debug, Parser)]
#[command(
    name = "dots",
    version,
    about,
    long_about = None,
    disable_help_subcommand = true
)]
pub struct Args {
    /// 要执行的命令。
    #[command(subcommand)]
    pub command: Command,
}

/// dots 的命令。
#[derive(Debug, Subcommand)]
pub enum Command {
    /// 执行所有 Cargo binary 声明，只安装或升级，不卸载。
    Install,
    /// 收敛当前 Desired Set，并安全创建、更新或删除 Resource。
    Sync {
        /// 只看会做什么，不实际改动。
        #[arg(long)]
        dry_run: bool,
    },
    /// 只读展示与 sync 相同的创建、更新、删除、Collision 和 Drift Plan。
    Status,
    /// 不修改真实对象，只从 Applied Inventory 放弃 ownership。
    Forget {
        /// `status` 输出的 Resource selector，也可传 filesystem path。
        resource: String,
    },
}

/// 解析参数并分发执行。
pub fn run() -> Result<ExitCode> {
    let args = Args::parse();
    match args.command {
        Command::Install => {
            cmd::install::run()?;
            Ok(ExitCode::SUCCESS)
        }
        Command::Sync { dry_run } => {
            let healthy = cmd::sync::run(dry_run)?;
            Ok(if healthy {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            })
        }
        Command::Status => {
            let green = cmd::status::run()?;
            Ok(if green {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            })
        }
        Command::Forget { resource } => {
            cmd::forget::run(&resource)?;
            Ok(ExitCode::SUCCESS)
        }
    }
}
