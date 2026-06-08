//! `dots bootstrap` —— 装机：packages + toolchains + sync（§11）。
//!
//! 包清单进 `packages/*.txt`（加包改文本不改代码）；toolchains 逐项幂等探测跳过。

use std::cell::RefCell;
use std::fs;
use std::process::Command;
use std::rc::Rc;

use super::{Result, current_os, find_repo_root, home_dir, os_str};
use crate::hooks::{EffectState, ToolchainFilter, activate_host};
use crate::lua::{LuaCtx, eval_manifest};
use crate::render;
use crate::state::State;

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

    // 0) 新机 host 引导：未登记 + 交互终端 → 问别名/工具链组，写 ~/.config/dots/host + dots.lua。
    //    必须在 host_toolchain_filter 与末尾 sync 之前——它们会重读 dots.lua/别名文件，从而命中新块。
    crate::onboard::maybe_onboard(&repo_root, &home_dir()?)?;

    let Some(backend) = Backend::detect() else {
        render::warn("无法识别包后端（仅支持 pacman/brew/apt），跳过装包");
        return crate::cmd::sync::run(false);
    };
    render::ok(&format!("包后端：{backend:?}"));

    install_packages(&repo_root, backend);
    if backend == Backend::Pacman {
        install_aur_packages(&repo_root);
    }
    let filter = host_toolchain_filter(&repo_root);
    install_toolchains(&repo_root, filter.as_ref());

    render::header("收尾：dots sync");
    crate::cmd::sync::run(false)
}

/// 按行解析包清单（`#` 注释、空行跳过）。
fn parse_pkg_list(content: &str) -> Vec<&str> {
    content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect()
}

/// 读 packages/<backend>.txt 安装系统包（按行，# 注释跳过）。
fn install_packages(repo_root: &std::path::Path, backend: Backend) {
    let path = repo_root.join("packages").join(backend.pkg_file());
    let Ok(content) = fs::read_to_string(&path) else {
        render::warn(&format!("无包清单 {}，跳过", path.display()));
        return;
    };
    let pkgs = parse_pkg_list(&content);
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
        Backend::Brew => match ensure_brew() {
            Some(bin) => run_cmd(&bin, &[&["install"], pkgs.as_slice()].concat()),
            None => {
                render::warn("Homebrew 自举失败，跳过装包");
                false
            }
        },
    };
    if !ok {
        render::warn("装包出错（部分包可能已存在），继续");
    }
}

/// 安装 AUR 包（仅 Pacman 后端）：aur.txt 非空时先确保 paru（缺则 makepkg 自举），再 `paru -S`。
fn install_aur_packages(repo_root: &std::path::Path) {
    let path = repo_root.join("packages").join("aur.txt");
    let Ok(content) = fs::read_to_string(&path) else {
        return;
    };
    let pkgs = parse_pkg_list(&content);
    if pkgs.is_empty() {
        return;
    }
    if !which("paru") && !bootstrap_paru() {
        render::warn("paru 自举失败，跳过 AUR 包");
        return;
    }
    let ok = run_cmd(
        "paru",
        &[&["-S", "--needed", "--noconfirm"], pkgs.as_slice()].concat(),
    );
    if !ok {
        render::warn("AUR 装包出错（部分包可能已存在），继续");
    }
}

/// Homebrew 安装后的预期路径（安装脚本不改当前进程 PATH，须按架构定位）。
fn brew_path_for_arch(arch: &str) -> &'static str {
    if arch == "aarch64" {
        "/opt/homebrew/bin/brew"
    } else {
        "/usr/local/bin/brew"
    }
}

/// 定位可用的 brew：PATH 里有就用裸名，否则按架构探安装路径。
fn find_brew() -> Option<String> {
    if which("brew") {
        return Some("brew".to_owned());
    }
    let path = brew_path_for_arch(std::env::consts::ARCH);
    std::path::Path::new(path).exists().then(|| path.to_owned())
}

/// 确保 brew 可用：缺则官方脚本自举（NONINTERACTIVE；脚本会顺带装 Xcode CLT）。
fn ensure_brew() -> Option<String> {
    if let Some(bin) = find_brew() {
        return Some(bin);
    }
    render::ok("自举 Homebrew（官方安装脚本）…");
    let ok = run_cmd(
        "sh",
        &[
            "-c",
            "NONINTERACTIVE=1 /bin/bash -c \"$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)\"",
        ],
    );
    if !ok {
        return None;
    }
    find_brew()
}

/// 自举 paru：git clone AUR 源码 + makepkg -si（依赖 pacman.txt 已装的 base-devel/git）。
fn bootstrap_paru() -> bool {
    render::ok("自举 paru（git clone + makepkg）…");
    let build_dir = std::env::temp_dir().join("dots-paru-bootstrap");
    let _ = fs::remove_dir_all(&build_dir);
    let dir = build_dir.to_string_lossy();
    run_cmd(
        "git",
        &[
            "clone",
            "--depth=1",
            "https://aur.archlinux.org/paru.git",
            &dir,
        ],
    ) && run_cmd(
        "sh",
        &["-c", &format!("cd '{dir}' && makepkg -si --noconfirm")],
    )
}

/// toolchains.toml 的一条安装项。
struct ToolchainEntry {
    /// 探测名（`command -v` 用）；`!` 后缀已剥离。
    name: String,
    /// 安装 shell 命令。
    cmd: String,
    /// 名带 `!` 后缀：跳过探测每次执行（rustup target 等无 binary 可探测，靠命令自身幂等）。
    always: bool,
    /// 所属组（toml `[节头]`；首个节头前的裸条目归 "core"）。
    group: String,
}

/// 解析 toolchains.toml（从简：每行 `name = "shell 命令"`，`name!` 表示总是执行；
/// `[组名]` 节头切换其后条目的归属组）。
fn parse_toolchains(content: &str) -> Vec<ToolchainEntry> {
    let mut group = "core".to_owned();
    let mut entries = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        // 节头（允许行尾注释）：之后的条目归该组
        if let Some(rest) = line.strip_prefix('[')
            && let Some((name, _)) = rest.split_once(']')
        {
            group = name.trim().to_owned();
            continue;
        }
        let Some((name, cmd)) = line.split_once('=') else {
            continue;
        };
        // TOML 裸键不允许 `!`，带 `!` 的键写成 "name!" 引号形式，此处剥掉。
        let name = name.trim().trim_matches('"');
        let cmd = cmd.trim().trim_matches('"');
        let (name, always) = match name.strip_suffix('!') {
            Some(stripped) => (stripped, true),
            None => (name, false),
        };
        entries.push(ToolchainEntry {
            name: name.to_owned(),
            cmd: cmd.to_owned(),
            always,
            group: group.clone(),
        });
    }
    entries
}

/// 按 dots.lua 的 `toolchains{}` 声明过滤条目。
///
/// # Return:
///   `(保留条目, 未知组名)`——声明里引用了清单中不存在的组按未知上报（防拼写错静默漏装）。
fn filter_toolchains(
    entries: Vec<ToolchainEntry>,
    filter: Option<&ToolchainFilter>,
) -> (Vec<ToolchainEntry>, Vec<String>) {
    let Some(filter) = filter else {
        return (entries, Vec::new());
    };
    let known: rustc_hash::FxHashSet<&str> =
        entries.iter().map(|entry| entry.group.as_str()).collect();
    let listed = match filter {
        ToolchainFilter::Only(groups) | ToolchainFilter::Skip(groups) => groups,
    };
    let unknown: Vec<String> = listed
        .iter()
        .filter(|name| !known.contains(name.as_str()))
        .cloned()
        .collect();
    let kept = entries
        .into_iter()
        .filter(|entry| match filter {
            ToolchainFilter::Only(groups) => groups.contains(&entry.group),
            ToolchainFilter::Skip(groups) => !groups.contains(&entry.group),
        })
        .collect();
    (kept, unknown)
}

/// 安装 toolchains（uv/starship/zoxide…），逐项 `which` 幂等跳过；`name!` 条目每次执行。
fn install_toolchains(repo_root: &std::path::Path, filter: Option<&ToolchainFilter>) {
    let path = repo_root.join("packages").join("toolchains.toml");
    let Ok(content) = fs::read_to_string(&path) else {
        render::warn("无 toolchains.toml，跳过工具链");
        return;
    };
    let (entries, unknown) = filter_toolchains(parse_toolchains(&content), filter);
    if !unknown.is_empty() {
        render::warn(&format!(
            "toolchains{{}} 引用了清单中不存在的组：{}（检查 dots.lua 拼写）",
            unknown.join(", ")
        ));
    }
    if let Some(filter) = filter {
        let desc = match filter {
            ToolchainFilter::Only(groups) => format!("only: {}", groups.join(", ")),
            ToolchainFilter::Skip(groups) => format!("skip: {}", groups.join(", ")),
        };
        render::ok(&format!("toolchains 范围（dots.lua）：{desc}"));
    }
    for entry in entries {
        if !entry.always && which(&entry.name) {
            render::ok(&format!("{} 已装，跳过", entry.name));
            continue;
        }
        render::ok(&format!("安装 {}…", entry.name));
        run_cmd("sh", &["-c", &entry.cmd]);
    }
}

/// 读 dots.lua 并激活当前 host 块，取 `toolchains{}` 声明。
///
/// 任何一步失败/未命中 host → `None`（= 全装，保持无声明时的旧行为）。
/// 以 dry-run 构造 EffectState：host 块若调用 `dots.run` 等 effect 原语，
/// 此处只声明不落盘——真正的副作用属于收尾的 sync 阶段。
fn host_toolchain_filter(repo_root: &std::path::Path) -> Option<ToolchainFilter> {
    let src = fs::read_to_string(repo_root.join("dots.lua")).ok()?;
    let home = home_dir().ok()?;
    let hostname = crate::hosts::current();
    let ctx = LuaCtx {
        host: hostname.clone(),
        os: os_str(current_os()).to_owned(),
        home: home.display().to_string(),
        repo: repo_root.display().to_string(),
    };
    let (manifest, handles) = eval_manifest(&src, &ctx).ok()?;
    let effect = Rc::new(RefCell::new(EffectState::new(
        repo_root.to_path_buf(),
        home,
        crate::timestamp(),
        /* dry_run */ true,
        State::default(),
    )));
    activate_host(&hostname, &manifest, &handles, &effect).ok()?;
    effect.borrow().toolchain_filter.clone()
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

#[cfg(test)]
mod tests {
    #![allow(clippy::min_ident_chars, clippy::missing_docs_in_private_items)]
    use super::*;

    #[test]
    fn pkg_list_skips_comments_and_blanks() {
        let content = "# 注释\ngit\n\n  neovim  \n# tail\nzsh";
        assert_eq!(parse_pkg_list(content), vec!["git", "neovim", "zsh"]);
    }

    #[test]
    fn toolchain_plain_entry_probes_by_name() {
        let entries = parse_toolchains(r#"uv = "curl -LsSf https://astral.sh/uv/install.sh | sh""#);
        assert_eq!(entries.len(), 1);
        let e = &entries[0];
        assert_eq!(e.name, "uv");
        assert_eq!(e.cmd, "curl -LsSf https://astral.sh/uv/install.sh | sh");
        assert!(!e.always);
    }

    #[test]
    fn toolchain_bang_suffix_means_always_run() {
        let entries =
            parse_toolchains(r#"rustup-wasm32! = "rustup target add wasm32-unknown-unknown""#);
        assert_eq!(entries.len(), 1);
        let e = &entries[0];
        assert_eq!(e.name, "rustup-wasm32");
        assert_eq!(e.cmd, "rustup target add wasm32-unknown-unknown");
        assert!(e.always);
    }

    #[test]
    fn brew_path_follows_arch() {
        // Apple Silicon 与 Intel 的 Homebrew 前缀不同，安装脚本不改当前进程 PATH。
        assert_eq!(brew_path_for_arch("aarch64"), "/opt/homebrew/bin/brew");
        assert_eq!(brew_path_for_arch("x86_64"), "/usr/local/bin/brew");
    }

    #[test]
    fn toolchain_quoted_key_is_legal_toml_and_strips_quotes() {
        // TOML 裸键不允许 `!`，带 `!` 的键必须写成 "name!" 引号形式。
        let entries =
            parse_toolchains(r#""rustup-wasm32!" = "rustup target add wasm32-unknown-unknown""#);
        assert_eq!(entries.len(), 1);
        let e = &entries[0];
        assert_eq!(e.name, "rustup-wasm32");
        assert!(e.always);
    }

    #[test]
    fn toolchain_skips_comments_blanks_and_malformed() {
        let content = "# 注释\n\n这行没有等号\nfoo = \"bar\"";
        let entries = parse_toolchains(content);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "foo");
    }

    #[test]
    fn toolchain_groups_default_core_and_follow_headers() {
        // 节头前的裸条目归 core（向后兼容）；节头可带行尾注释
        let content = "uv = \"u\"\n[dev]  # 开发机才装\ncargo-watch = \"c\"\n[ai]\nclaude = \"x\"";
        let entries = parse_toolchains(content);
        let got: Vec<(&str, &str)> = entries
            .iter()
            .map(|e| (e.name.as_str(), e.group.as_str()))
            .collect();
        assert_eq!(
            got,
            vec![("uv", "core"), ("cargo-watch", "dev"), ("claude", "ai")]
        );
    }

    #[test]
    fn toolchain_filter_only_keeps_listed_groups() {
        let entries = parse_toolchains("uv = \"u\"\n[dev]\nprek = \"p\"");
        let filter = ToolchainFilter::Only(vec!["core".to_owned()]);
        let (kept, unknown) = filter_toolchains(entries, Some(&filter));
        let names: Vec<&str> = kept.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["uv"]);
        assert!(unknown.is_empty());
    }

    #[test]
    fn toolchain_filter_skip_drops_listed_groups() {
        let entries = parse_toolchains("uv = \"u\"\n[ai]\nclaude = \"x\"\n[js]\npnpm = \"p\"");
        let filter = ToolchainFilter::Skip(vec!["ai".to_owned(), "js".to_owned()]);
        let (kept, unknown) = filter_toolchains(entries, Some(&filter));
        let names: Vec<&str> = kept.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["uv"]);
        assert!(unknown.is_empty());
    }

    #[test]
    fn toolchain_filter_reports_unknown_groups() {
        // 拼写错的组名要浮出来，不能静默装了个寂寞
        let entries = parse_toolchains("uv = \"u\"\n[dev]\nprek = \"p\"");
        let filter = ToolchainFilter::Only(vec!["croe".to_owned()]);
        let (kept, unknown) = filter_toolchains(entries, Some(&filter));
        assert!(kept.is_empty());
        assert_eq!(unknown, vec!["croe".to_owned()]);
    }

    #[test]
    fn toolchain_no_filter_keeps_all() {
        let entries = parse_toolchains("uv = \"u\"\n[dev]\nprek = \"p\"");
        let (kept, unknown) = filter_toolchains(entries, None);
        assert_eq!(kept.len(), 2);
        assert!(unknown.is_empty());
    }
}
