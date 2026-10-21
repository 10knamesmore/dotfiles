//! 系统包只补装缺失项；平台不匹配时不探测或调用对应包管理器。

use std::path::Path;
use std::process::{Command, Stdio};

use color_eyre::eyre::{Context, eyre};
use dots_core::Os;
use rustc_hash::FxHashSet;

use super::run_install_command;
use crate::cmd::Result;
use crate::render;

/// 两种显式声明来源；Homebrew 清单限定 macOS，不采用 Linuxbrew。
#[derive(Clone, Copy)]
pub(super) enum PackageManager {
    /// 以 `/etc/arch-release` 识别 Arch 系 Linux。
    Pacman,
    /// macOS 的 Homebrew formula。
    Brew,
}

impl PackageManager {
    /// CLI 输出与进程名使用的包管理器名称。
    fn name(self) -> &'static str {
        match self {
            Self::Pacman => "pacman",
            Self::Brew => "brew",
        }
    }

    /// 按平台判定清单是否适用，不以包管理器是否恰好在 PATH 上决定。
    fn applies_to(self, os: Os) -> bool {
        match self {
            Self::Pacman => os == Os::Linux && Path::new("/etc/arch-release").exists(),
            Self::Brew => os == Os::Macos,
        }
    }

    /// 查询该管理器记录的已安装包；失败时停止，不改用其他安装来源。
    fn installed_packages(self) -> Result<FxHashSet<String>> {
        let mut command = Command::new(self.name());
        match self {
            Self::Pacman => command.arg("-Qq"),
            Self::Brew => command.args(["list", "--formula", "-1"]),
        };
        let output = command
            .stderr(Stdio::inherit())
            .output()
            .wrap_err_with(|| format!("无法查询 {} 已安装包", self.name()))?;
        if !output.status.success() {
            return Err(eyre!(
                "查询 {} 已安装包失败，exit status：{}",
                self.name(),
                output.status
            ));
        }
        let names = String::from_utf8(output.stdout)?;
        Ok(names.lines().map(str::to_owned).collect())
    }

    /// 安装只传入缺失包；不刷新系统包数据库或发起全系统升级。
    fn install_command(self, missing: &[&String]) -> Command {
        let mut command = match self {
            Self::Pacman => {
                let mut command = Command::new("sudo");
                command.args(["pacman", "-S", "--needed", "--"]);
                command
            }
            Self::Brew => {
                let mut command = Command::new("brew");
                command.args(["install", "--formula", "--"]);
                command
            }
        };
        command.args(missing);
        command
    }
}

/// 非当前平台的非空清单显示跳过；当前平台按包管理器数据库判断是否已安装。
pub(super) fn run(
    manager: PackageManager,
    packages: &[String],
    os: Os,
    dry_run: bool,
) -> Result<()> {
    if packages.is_empty() {
        return Ok(());
    }
    if !manager.applies_to(os) {
        println!("  跳过 {}：不适用于当前平台", manager.name());
        return Ok(());
    }
    render::header(manager.name());
    let installed = manager.installed_packages()?;
    let mut missing = Vec::new();
    for package in packages {
        if installed.contains(package) {
            render::ok(&format!("已安装 {package}"));
        } else {
            missing.push(package);
        }
    }
    if !missing.is_empty() {
        run_install_command(
            &mut manager.install_command(&missing),
            &format!("{} 补装 {} 个包", manager.name(), missing.len()),
            dry_run,
        )?;
    }
    Ok(())
}
