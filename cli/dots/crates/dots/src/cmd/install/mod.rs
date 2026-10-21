//! `dots install`：跳过其他平台的清单，先安装系统包，再安装 Cargo binary。

mod cargo;
mod packages;

use std::process::Command;

use color_eyre::eyre::{Context, eyre};

use super::{Result, current_os, find_repo_root, home_dir, load_manifest};
use crate::render;
use packages::PackageManager;

/// dry-run 仍查询已安装的系统包，但不执行任何安装命令。
pub fn run(dry_run: bool) -> Result<()> {
    let repo_root = find_repo_root()?;
    let home = home_dir()?;
    let os = current_os();
    let manifest = load_manifest(&repo_root, &home, os)?;
    let install = &manifest.install;

    render::header("dots install");
    packages::run(PackageManager::Pacman, &install.pacman, os, dry_run)?;
    packages::run(PackageManager::Brew, &install.brew, os, dry_run)?;
    if !install.cargo.is_empty() {
        render::header("cargo");
    }
    for declaration in &install.cargo {
        cargo::run(declaration, &repo_root, &home, os, dry_run)?;
    }
    render::ok(if dry_run {
        "安装预览完成，未执行安装"
    } else {
        "安装完成"
    });
    Ok(())
}

/// 打印实际参数；真实安装继承终端，使 sudo 和包管理器可以直接交互。
fn run_install_command(command: &mut Command, description: &str, dry_run: bool) -> Result<()> {
    println!("  {description}");
    render::suggest(&format!("{command:?}"));
    if dry_run {
        return Ok(());
    }
    let status = command
        .status()
        .wrap_err_with(|| format!("无法启动安装命令：{description}"))?;
    if !status.success() {
        return Err(eyre!("安装失败：{description}，exit status：{status}"));
    }
    render::ok(description);
    Ok(())
}
