//! `dots install`：执行当前 Manifest 中的 Cargo binary 安装声明。

use std::path::Path;
use std::process::Command;

use color_eyre::eyre::{Context, eyre};
use dots_core::manifest::CargoBinaryDeclaration;

use super::{
    Result, absolute_source, absolute_target, current_os, find_repo_root, home_dir, load_manifest,
};
use crate::render;

/// 执行所有启用的 Cargo binary 声明。
///
/// 每项 declaration 都直接映射为一次 `cargo install`。没有声明时成功返回，不执行任何
/// 卸载操作，也不替调用方添加 `--force`。
pub fn run() -> Result<()> {
    let repo_root = find_repo_root()?;
    let home = home_dir()?;
    let manifest = load_manifest(&repo_root, &home, current_os())?;
    let declarations = &manifest.cargo_binaries;

    render::header("dots install");
    for declaration in declarations {
        execute_declaration(declaration, &repo_root, &home)?;
    }
    render::ok(&format!(
        "已执行 {} 条 Cargo binary 声明",
        declarations.len()
    ));
    Ok(())
}

/// 执行一条 Cargo binary declaration。
fn execute_declaration(
    declaration: &CargoBinaryDeclaration,
    repo_root: &Path,
    home: &Path,
) -> Result<()> {
    match declaration {
        CargoBinaryDeclaration::Workspace { path, binary, root } => {
            let path = absolute_source(path, repo_root, home)?;
            let root = absolute_target(root, home)?;
            eprintln!("  install workspace binary {binary}");
            run_cargo_install(
                Command::new("cargo")
                    .args(["install", "--locked", "--path"])
                    .arg(path)
                    .args(["--bin", binary, "--root"])
                    .arg(root),
                &format!("workspace binary `{binary}`"),
            )
        }
        CargoBinaryDeclaration::CratesIo { package, binaries } => {
            eprintln!("  install crates.io package {package}");
            let mut command = Command::new("cargo");
            command.args(["install", "--locked"]);
            for binary in binaries {
                command.args(["--bin", binary]);
            }
            command.arg(package);
            run_cargo_install(&mut command, &format!("crates.io package `{package}`"))
        }
    }
}

/// 执行已经完整构造的 `cargo install`，保留 Cargo 的输出和退出状态语义。
fn run_cargo_install(command: &mut Command, description: &str) -> Result<()> {
    let status = command
        .status()
        .wrap_err_with(|| format!("无法启动 cargo install：{description}"))?;
    if !status.success() {
        return Err(eyre!(
            "cargo install {description} 失败，exit status：{status}"
        ));
    }
    render::ok(description);
    Ok(())
}
