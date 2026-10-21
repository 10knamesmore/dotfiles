//! Cargo 安装区分 crates.io 与本地 package 两种来源。

use std::path::Path;
use std::process::Command;

use dots_core::Os;
use dots_core::manifest::{CargoBinaryDeclaration, CargoBinarySource};

use super::run_install_command;
use crate::cmd::{Result, absolute_source, absolute_target};

/// 跳过平台不匹配的声明；本地 package 强制重建，crates.io 更新交给 Cargo 判断。
pub(super) fn run(
    declaration: &CargoBinaryDeclaration,
    repo_root: &Path,
    home: &Path,
    os: Os,
    dry_run: bool,
) -> Result<()> {
    let description = match &declaration.source {
        CargoBinarySource::Local { binary, .. } => format!("local binary `{binary}`"),
        CargoBinarySource::CratesIo { package, .. } => format!("crates.io package `{package}`"),
    };
    if declaration
        .platforms
        .as_ref()
        .is_some_and(|platforms| !platforms.contains(&os))
    {
        println!("  跳过 {description}：不适用于当前平台");
        return Ok(());
    }
    match &declaration.source {
        CargoBinarySource::Local { path, binary, root } => {
            let path = absolute_source(path, repo_root, home)?;
            let root = absolute_target(root, home)?;
            run_install_command(
                Command::new("cargo")
                    .args(["install", "--locked", "--force", "--path"])
                    .arg(path)
                    .args(["--bin", binary, "--root"])
                    .arg(root),
                &description,
                dry_run,
            )
        }
        CargoBinarySource::CratesIo { package, binaries } => {
            let mut command = Command::new("cargo");
            command.args(["install", "--locked"]);
            for binary in binaries {
                command.args(["--bin", binary]);
            }
            command.arg(package);
            run_install_command(&mut command, &description, dry_run)
        }
    }
}
