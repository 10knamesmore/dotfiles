//! `dots bootstrap` —— 装机：packages + toolchains + sync（§11）。
//!
//! 包清单进 `packages/*.txt`（加包改文本不改代码）；toolchains 逐项幂等探测跳过。

use std::fs;
use std::process::Command;

use super::{Result, find_repo_root};
use crate::render;

/// 包后端。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Backend {
    /// Arch pacman（+ AUR）。
    Pacman,
    /// macOS Homebrew。
    Brew,
    /// Debian/Ubuntu apt。
    Apt,
}

impl Backend {
    /// 探测当前系统后端。
    fn detect() -> Option<Self> {
        if cfg!(target_os = "macos") {
            return Some(Self::Brew);
        }
        let osr = fs::read_to_string("/etc/os-release").unwrap_or_default();
        if osr.contains("arch") || osr.contains("ID_LIKE=arch") {
            Some(Self::Pacman)
        } else if osr.contains("debian") || osr.contains("ubuntu") {
            Some(Self::Apt)
        } else {
            None
        }
    }

    /// 包清单文件名。
    fn pkg_file(self) -> &'static str {
        match self {
            Self::Pacman => "pacman.txt",
            Self::Brew => "brew.txt",
            Self::Apt => "apt.txt",
        }
    }
}

/// 运行 bootstrap。
pub fn run() -> Result<()> {
    let repo_root = find_repo_root()?;
    render::header("dots bootstrap");

    let Some(backend) = Backend::detect() else {
        render::warn("无法识别包后端（仅支持 pacman/brew/apt），跳过装包");
        return crate::cmd::sync::run(false);
    };
    render::ok(&format!("包后端：{backend:?}"));

    install_packages(&repo_root, backend);
    install_toolchains(&repo_root);

    render::header("收尾：dots sync");
    crate::cmd::sync::run(false)
}

/// 读 packages/<backend>.txt 安装系统包（按行，# 注释跳过）。
fn install_packages(repo_root: &std::path::Path, backend: Backend) {
    let path = repo_root.join("packages").join(backend.pkg_file());
    let Ok(content) = fs::read_to_string(&path) else {
        render::warn(&format!("无包清单 {}，跳过", path.display()));
        return;
    };
    let pkgs: Vec<&str> = content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect();
    if pkgs.is_empty() {
        return;
    }
    let ok = match backend {
        Backend::Pacman => run_cmd(
            "sudo",
            &[
                &["pacman", "-S", "--needed", "--noconfirm"],
                pkgs.as_slice(),
            ]
            .concat(),
        ),
        Backend::Apt => run_cmd(
            "sudo",
            &[&["apt", "install", "-y"], pkgs.as_slice()].concat(),
        ),
        Backend::Brew => run_cmd("brew", &[&["install"], pkgs.as_slice()].concat()),
    };
    if !ok {
        render::warn("装包出错（部分包可能已存在），继续");
    }
}

/// 安装 toolchains（uv/starship/zoxide…），逐项 `which` 幂等跳过。
fn install_toolchains(repo_root: &std::path::Path) {
    // toolchains.toml 解析从简：每行 `name = "shell 命令"`；name 已存在则跳过。
    let path = repo_root.join("packages").join("toolchains.toml");
    let Ok(content) = fs::read_to_string(&path) else {
        render::warn("无 toolchains.toml，跳过工具链");
        return;
    };
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((name, cmd)) = line.split_once('=') else {
            continue;
        };
        let name = name.trim();
        let cmd = cmd.trim().trim_matches('"');
        if which(name) {
            render::ok(&format!("{name} 已装，跳过"));
            continue;
        }
        render::ok(&format!("安装 {name}…"));
        run_cmd("sh", &["-c", cmd]);
    }
}

/// `command -v` 探测。
fn which(name: &str) -> bool {
    Command::new("sh")
        .args(["-c", &format!("command -v {name}")])
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

/// 跑命令，返回是否成功。
fn run_cmd(prog: &str, args: &[&str]) -> bool {
    Command::new(prog)
        .args(args)
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}
