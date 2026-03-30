#!/usr/bin/env python3
"""
dotfiles 安装脚本。

功能：
  1. 检测操作系统类型（macos 或 linux）
  2. 可选执行 bootstrap，为全新环境安装基础工具链
  3. 备份已有配置到 dotfiles/backup/时间戳/ 目录
  4. 将 dotfiles 目录下的文件以符号链接方式安装到用户主目录
  5. 渲染模板文件并创建受管的 zsh stub
  6. 支持 dry-run 模式（预览将要执行的操作）
"""

from __future__ import annotations

import argparse
import os
import platform
import shlex
import shutil
import subprocess
import sys
from contextlib import contextmanager
from datetime import datetime
from pathlib import Path
from typing import final


class Colors:
    BOLD: str = "\033[1m"
    DIM: str = "\033[2m"
    GREEN: str = "\033[0;32m"
    YELLOW: str = "\033[1;33m"
    RED: str = "\033[0;31m"
    BLUE: str = "\033[0;34m"
    CYAN: str = "\033[0;36m"
    RESET: str = "\033[0m"


@final
class BootstrapConfig:
    HOMEBREW_INSTALL = (
        'NONINTERACTIVE=1 /bin/bash -c "$(curl -fsSL '
        'https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"'
    )
    NVM_INSTALL = (
        'PROFILE=/dev/null bash -c "$(curl -fsSL '
        'https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.3/install.sh)"'
    )
    OH_MY_ZSH_INSTALL = (
        'RUNZSH=no CHSH=no KEEP_ZSHRC=yes sh -c "$(curl -fsSL '
        'https://raw.githubusercontent.com/ohmyzsh/ohmyzsh/master/tools/install.sh)" '
        '"" --unattended'
    )
    RUSTUP_INSTALL = (
        'curl --proto "=https" --tlsv1.2 -fsSL https://sh.rustup.rs '
        "| sh -s -- -y --profile minimal --default-toolchain stable"
    )
    UV_INSTALL = "curl -LsSf https://astral.sh/uv/install.sh | sh"
    STARSHIP_INSTALL = "curl -fsSL https://starship.rs/install.sh | sh -s -- -y"
    NEOVIM_STABLE_BASE = "https://github.com/neovim/neovim/releases/download/stable"


@final
class DotfilesInstaller:
    ZSHRC_MARKER: str = "# DOTFILES_MANAGED:"
    NVM_DIRNAME: str = ".nvm"
    CARGO_BIN_DIRNAME: str = ".cargo/bin"

    def __init__(self, dry_run: bool = False, bootstrap: bool = False):
        self.dry_run = dry_run
        self.bootstrap = bootstrap
        self.dotfiles_dir = Path(__file__).parent.absolute()
        self.home_dir = Path.home()
        self.timestamp = datetime.now().strftime("%Y-%m-%dT%H:%M:%S")

        self.backup_dir = self.dotfiles_dir / "backup"
        self.generated_dir = self.dotfiles_dir / "generated"
        self.scripts_dir = self.generated_dir / "scripts"
        self.skills_dir = self.dotfiles_dir / "general" / "skills"

        self.templates = {
            "ZSH_CUSTOM_TEMPLATE": str(self.dotfiles_dir / "static" / "omz_custom"),
            "DOT_TEMPLATE": f"cd {self.dotfiles_dir}",
            "SCRIPTS_DIR_TEMPLATE": str(self.scripts_dir),
            "SKILLS_DIR_TEMPLATE": str(self.skills_dir),
        }
        self.rendered_templates: dict[str, str] = {}
        self.stats = {"linked": 0, "removed": 0, "backed_up": 0, "rendered": 0}

        self.os_type = self.detect_os()
        self.os_release = self.read_os_release()
        self.ensure_directories()

    # ── 输出方法 ──────────────────────────────────────────────────────

    def _fmt_path(self, path: Path) -> str:
        try:
            return "~/" + str(path.relative_to(self.home_dir))
        except ValueError:
            return str(path)

    @contextmanager
    def stage(self, title: str, color: str = Colors.CYAN, icon: str = "◆"):
        print(f"\n{color}{Colors.BOLD}  {icon} {title}{Colors.RESET}")
        print(f"{Colors.DIM}  {'─' * 48}{Colors.RESET}")
        yield
        print()

    def info(self, message: str) -> None:
        print(f"  {Colors.DIM}·{Colors.RESET}  {message}")

    def warn(self, message: str) -> None:
        print(f"  {Colors.YELLOW}⚠{Colors.RESET}  {message}")

    def error(self, message: str) -> None:
        print(f"  {Colors.RED}✖{Colors.RESET}  {Colors.BOLD}{message}{Colors.RESET}")

    def success(self, message: str) -> None:
        print(f"  {Colors.GREEN}✔{Colors.RESET}  {message}")

    def act_link(self, target: Path, source: Path) -> None:
        t, s = self._fmt_path(target), self._fmt_path(source)
        print(f"  {Colors.GREEN}+{Colors.RESET}  {t} {Colors.DIM}→ {s}{Colors.RESET}")
        self.stats["linked"] += 1

    def act_remove(self, path: Path) -> None:
        print(
            f"  {Colors.RED}×{Colors.RESET}  {Colors.DIM}{self._fmt_path(path)}{Colors.RESET}"
        )
        self.stats["removed"] += 1

    def act_backup(self, src: Path) -> None:
        print(
            f"  {Colors.YELLOW}↩{Colors.RESET}  {self._fmt_path(src)} {Colors.DIM}→ backup/{Colors.RESET}"
        )
        self.stats["backed_up"] += 1

    def act_render(self, template: str, output: str) -> None:
        print(
            f"  {Colors.CYAN}⊙{Colors.RESET}  {Colors.DIM}{template}{Colors.RESET} → {Colors.DIM}{output}{Colors.RESET}"
        )
        self.stats["rendered"] += 1

    def act_skip(self, message: str) -> None:
        print(f"  {Colors.DIM}⊘  {message}{Colors.RESET}")

    def act_mkdir(self, path: Path) -> None:
        print(
            f"  {Colors.BLUE}◎{Colors.RESET}  {Colors.DIM}mkdir {self._fmt_path(path)}{Colors.RESET}"
        )

    def act_create(self, path: Path, label: str = "") -> None:
        suffix = f" {Colors.DIM}({label}){Colors.RESET}" if label else ""
        print(f"  {Colors.GREEN}+{Colors.RESET}  {self._fmt_path(path)}{suffix}")

    def error_exit(self, message: str) -> None:
        self.error(message)
        sys.exit(1)

    def _stats_line(self) -> str:
        s = self.stats
        C = Colors
        return (
            f"{C.GREEN}+{C.RESET} {s['linked']} 链接  "
            f"{C.RED}×{C.RESET} {s['removed']} 删除  "
            f"{C.YELLOW}↩{C.RESET} {s['backed_up']} 备份  "
            f"{C.CYAN}⊙{C.RESET} {s['rendered']} 渲染"
        )

    # ── 平台与命令检测 ────────────────────────────────────────────────

    def detect_os(self) -> str:
        system = platform.system()
        if system == "Darwin":
            return "macos"
        if system == "Linux":
            return "linux"
        return "other"

    def read_os_release(self) -> dict[str, str]:
        if self.os_type != "linux":
            return {}
        os_release = Path("/etc/os-release")
        if not os_release.exists():
            return {}

        result: dict[str, str] = {}
        for line in os_release.read_text().splitlines():
            if "=" not in line or line.startswith("#"):
                continue
            key, value = line.split("=", 1)
            result[key] = value.strip().strip('"')
        return result

    def detect_package_backend(self) -> str:
        if self.os_type == "macos":
            return "brew"

        distro_id = self.os_release.get("ID", "").lower()
        distro_like = self.os_release.get("ID_LIKE", "").lower()

        if distro_id == "ubuntu":
            return "apt"
        if distro_id == "arch" or "arch" in distro_like:
            return "pacman"

        self.error_exit("bootstrap 仅支持 macOS、Ubuntu 和 Arch Linux。")
        return "unsupported"

    def command_exists(self, command: str) -> bool:
        return shutil.which(command) is not None

    def brew_bin(self) -> str:
        brew = shutil.which("brew")
        if brew:
            return brew

        for candidate in ("/opt/homebrew/bin/brew", "/usr/local/bin/brew"):
            if Path(candidate).exists():
                return candidate

        self.error_exit("未找到 Homebrew 可执行文件。")
        return "brew"

    def cargo_bin(self) -> Path | None:
        local_cargo = self.home_dir / self.CARGO_BIN_DIRNAME / "cargo"
        if local_cargo.exists():
            return local_cargo

        cargo = shutil.which("cargo")
        if cargo:
            return Path(cargo)
        return None

    def nvm_script(self) -> Path:
        return self.home_dir / self.NVM_DIRNAME / "nvm.sh"

    def execute(
        self,
        argv: list[str],
        *,
        env: dict[str, str] | None = None,
        label: str | None = None,
    ) -> None:
        command_str = shlex.join(argv)
        if label:
            self.info(f"{label}: {command_str}")
        else:
            self.info(command_str)

        if self.dry_run:
            self.act_skip(f"DRY-RUN: {command_str}")
            return

        merged_env = os.environ.copy()
        if env:
            merged_env.update(env)

        try:
            subprocess.run(argv, check=True, env=merged_env)
        except subprocess.CalledProcessError as exc:
            self.error_exit(f"命令执行失败: {command_str}\n退出码: {exc.returncode}")

    def execute_shell(
        self,
        script: str,
        *,
        label: str | None = None,
        env: dict[str, str] | None = None,
    ) -> None:
        self.execute(["/bin/bash", "-lc", script], env=env, label=label)

    # ── bootstrap ────────────────────────────────────────────────────

    def bootstrap_environment(self) -> None:
        backend = self.detect_package_backend()
        with self.stage("Bootstrap 基础环境", icon="⬢"):
            self.info(f"后端: {backend}")
            self.bootstrap_package_manager(backend)
            self.bootstrap_system_packages(backend)
            self.bootstrap_neovim(backend)
            self.bootstrap_nvm_node_pnpm()
            self.bootstrap_rustup()
            self.bootstrap_rust_cli_tools()
            self.bootstrap_starship()
            self.bootstrap_uv()
            self.bootstrap_oh_my_zsh()
            self.print_post_bootstrap_hints()

    def bootstrap_package_manager(self, backend: str) -> None:
        if backend != "brew" or self.command_exists("brew"):
            return
        self.execute_shell(BootstrapConfig.HOMEBREW_INSTALL, label="安装 Homebrew")

    def bootstrap_system_packages(self, backend: str) -> None:
        packages_by_backend = {
            "brew": ["git", "curl", "zsh", "neovim", "fzf", "ripgrep"],
            "apt": [
                "git",
                "curl",
                "zsh",
                "fzf",
                "ripgrep",
                "build-essential",
                "pkg-config",
                "libssl-dev",
            ],
            "pacman": [
                "git",
                "curl",
                "zsh",
                "neovim",
                "fzf",
                "ripgrep",
                "base-devel",
                "pkgconf",
                "openssl",
            ],
        }

        packages = packages_by_backend[backend]
        if backend == "brew":
            self.execute(
                [self.brew_bin(), "install", *packages], label="安装基础系统包"
            )
            return

        if backend == "apt":
            self.execute(["sudo", "apt-get", "update"], label="刷新 apt 索引")
            self.execute(
                ["sudo", "apt-get", "install", "-y", *packages],
                label="安装基础系统包",
            )
            return

        self.execute(
            ["sudo", "pacman", "-Sy", "--needed", "--noconfirm", *packages],
            label="安装基础系统包",
        )

    def bootstrap_neovim(self, backend: str) -> None:
        if self.command_exists("nvim"):
            self.act_skip("nvim 已安装")
            return

        if backend == "apt":
            self.bootstrap_neovim_official_linux()
            return

        self.warn("未找到 nvim，请确认系统包安装是否成功")

    def bootstrap_neovim_official_linux(self) -> None:
        arch_map = {
            "x86_64": "x86_64",
            "amd64": "x86_64",
            "aarch64": "arm64",
            "arm64": "arm64",
        }
        machine = platform.machine().lower()
        asset_arch = arch_map.get(machine)
        if asset_arch is None:
            self.error_exit(f"Ubuntu bootstrap 暂不支持该架构的 Neovim 安装: {machine}")

        local_bin = self.home_dir / ".local" / "bin"
        local_share = self.home_dir / ".local" / "share"
        nvim_dir = local_share / "nvim"
        archive_name = f"nvim-linux-{asset_arch}.tar.gz"
        download_url = f"{BootstrapConfig.NEOVIM_STABLE_BASE}/{archive_name}"

        install_script = f"""
set -euo pipefail
mkdir -p {shlex.quote(str(local_bin))}
mkdir -p {shlex.quote(str(local_share))}
tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT
curl -fsSL {shlex.quote(download_url)} -o "$tmp_dir/{archive_name}"
rm -rf {shlex.quote(str(nvim_dir))}
tar -xzf "$tmp_dir/{archive_name}" -C "$tmp_dir"
mv "$tmp_dir/nvim-linux-{asset_arch}" {shlex.quote(str(nvim_dir))}
ln -snf {shlex.quote(str(nvim_dir / "bin" / "nvim"))} {shlex.quote(str(local_bin / "nvim"))}
"""
        self.execute_shell(install_script, label="安装官方 Neovim stable")

    def bootstrap_nvm_node_pnpm(self) -> None:
        nvm_script = self.nvm_script()
        if not nvm_script.exists():
            self.execute_shell(BootstrapConfig.NVM_INSTALL, label="安装 nvm")
        else:
            self.act_skip("nvm 已安装")

        nvm_bootstrap = (
            f'export NVM_DIR="{self.home_dir / self.NVM_DIRNAME}"\n'
            '[ -s "$NVM_DIR/nvm.sh" ] && . "$NVM_DIR/nvm.sh"\n'
        )

        node_versions_dir = self.home_dir / self.NVM_DIRNAME / "versions" / "node"
        has_nvm_node = node_versions_dir.exists() and any(node_versions_dir.iterdir())

        if not has_nvm_node:
            self.execute_shell(
                nvm_bootstrap + "nvm install --lts\nnvm alias default lts/*",
                label="安装 Node.js LTS",
            )
        else:
            self.act_skip("nvm 管理的 Node.js 已安装")

        if not self.command_exists("pnpm"):
            self.execute_shell(
                nvm_bootstrap + "npm install -g pnpm",
                label="安装 pnpm",
            )
        else:
            self.act_skip("pnpm 已安装")

    def bootstrap_rustup(self) -> None:
        rustup_bin = self.home_dir / self.CARGO_BIN_DIRNAME / "rustup"
        if rustup_bin.exists() or self.command_exists("rustup"):
            self.act_skip("rustup 已安装")
            return

        self.execute_shell(BootstrapConfig.RUSTUP_INSTALL, label="安装 rustup + stable")

    def bootstrap_rust_cli_tools(self) -> None:
        cargo = self.cargo_bin()
        if cargo is None:
            self.warn("未找到 cargo，跳过 eza / zellij 安装")
            return

        for tool in ("eza", "zellij"):
            if (
                self.command_exists(tool)
                or (self.home_dir / self.CARGO_BIN_DIRNAME / tool).exists()
            ):
                self.act_skip(f"{tool} 已安装")
                continue
            self.execute(
                [str(cargo), "install", tool, "--locked"],
                label=f"安装 {tool}",
            )

    def bootstrap_starship(self) -> None:
        if self.command_exists("starship"):
            self.act_skip("starship 已安装")
            return
        self.execute_shell(BootstrapConfig.STARSHIP_INSTALL, label="安装 starship")

    def bootstrap_uv(self) -> None:
        if self.command_exists("uv") or (self.home_dir / ".local/bin/uv").exists():
            self.act_skip("uv 已安装")
            return
        self.execute_shell(BootstrapConfig.UV_INSTALL, label="安装 uv")

    def bootstrap_oh_my_zsh(self) -> None:
        if (self.home_dir / ".oh-my-zsh").exists():
            self.act_skip("Oh My Zsh 已安装")
            return
        self.execute_shell(BootstrapConfig.OH_MY_ZSH_INSTALL, label="安装 Oh My Zsh")

    def print_post_bootstrap_hints(self) -> None:
        with self.stage("Bootstrap 后提示", icon="☰"):
            self.info("可复制执行的后续命令:")
            print(f"     {Colors.YELLOW}exec zsh{Colors.RESET}")
            print(
                f'     {Colors.YELLOW}source "{self.home_dir / self.NVM_DIRNAME / "nvm.sh"}"{Colors.RESET}'
            )
            print(f'     {Colors.YELLOW}chsh -s "$(command -v zsh)"{Colors.RESET}')

    # ── 核心逻辑 ──────────────────────────────────────────────────────

    def ensure_directories(self) -> None:
        with self.stage("初始化", icon="◎"):
            for directory in [self.generated_dir, self.scripts_dir, self.backup_dir]:
                if not directory.exists():
                    self.act_mkdir(directory)
                    if not self.dry_run:
                        directory.mkdir(parents=True, exist_ok=True)

    def backup_file(self, target: Path) -> None:
        if not target.exists() and not target.is_symlink():
            return
        if target.is_symlink():
            self.act_remove(target)
            if not self.dry_run:
                target.unlink()
            return
        if target.is_file() or target.is_dir():
            parent_dir = target.parent.name
            if parent_dir == ".config":
                backup_path = self.backup_dir / self.timestamp / ".config" / target.name
            else:
                backup_path = self.backup_dir / self.timestamp / target.name
            if not self.dry_run:
                backup_path.parent.mkdir(parents=True, exist_ok=True)
            self.act_backup(target)
            if not self.dry_run:
                _ = shutil.move(str(target), str(backup_path))
            return
        self.error_exit(f"不支持的类型! 请检查: {target}")

    def create_symlink(self, source: Path, target: Path) -> None:
        if not self.dry_run:
            target.parent.mkdir(parents=True, exist_ok=True)
        self.backup_file(target)
        self.act_link(target, source)
        if not self.dry_run:
            try:
                target.symlink_to(source)
            except Exception as e:
                self.error_exit(f"创建符号链接失败: {target} → {source}\n错误: {e}")

    def process_directory(
        self, source_dir: Path, target_base: Path | None = None
    ) -> None:
        if target_base is None:
            target_base = self.home_dir
        if not source_dir.exists() or not source_dir.is_dir():
            self.warn(f"目录不存在: {source_dir}")
            return
        for source_path in sorted(source_dir.iterdir()):
            source_entry = source_path.name
            if source_entry in (".DS_Store", "skills") or source_entry.endswith(
                ".template"
            ):
                continue
            target_path = target_base / source_entry
            if source_entry == ".config" and source_path.is_dir():
                for sub_source_path in sorted(source_path.iterdir()):
                    sub_entry = sub_source_path.name
                    if sub_entry == ".DS_Store" or sub_entry.endswith(".template"):
                        continue
                    self.create_symlink(sub_source_path, target_path / sub_entry)
            elif source_entry == "scripts" and source_path.is_dir():
                for script_path in sorted(source_path.iterdir()):
                    if script_path.name == ".DS_Store":
                        continue
                    self.create_symlink(
                        script_path, self.scripts_dir / script_path.name
                    )
            else:
                self.create_symlink(source_path, target_path)

    def render_template(self, template_path: Path, output_path: Path) -> None:
        generated_path = self.generated_dir / template_path.name.replace(
            ".template", ""
        )
        self.act_render(template_path.name, generated_path.name)
        if not self.dry_run:
            _ = shutil.copy2(template_path, generated_path)
            content = generated_path.read_text()
            for key, value in self.templates.items():
                if key in content:
                    content = content.replace(key, value)
                    self.rendered_templates[key] = value
            _ = generated_path.write_text(content)

        if output_path.is_symlink():
            self.act_remove(output_path)
            if not self.dry_run:
                output_path.unlink()
        elif output_path.exists():
            self.act_remove(output_path)
            if not self.dry_run:
                output_path.unlink()

        self.act_link(output_path, generated_path)
        if not self.dry_run:
            output_path.symlink_to(generated_path)

    def post_process(self) -> None:
        with self.stage("模板处理", icon="⊙"):
            zshrc_template_link = self.home_dir / ".zshrc.template"
            if zshrc_template_link.is_symlink():
                self.act_remove(zshrc_template_link)
                if not self.dry_run:
                    zshrc_template_link.unlink()

            zshrc_dotfiles_template = (
                self.dotfiles_dir / "general" / ".zshrc_dotfiles.template"
            )
            zshrc_dotfiles_target = self.home_dir / ".zshrc_dotfiles"
            self.render_template(zshrc_dotfiles_template, zshrc_dotfiles_target)

            self._setup_zshrc_stub(self.home_dir / ".zshrc")

            vars_to_show = self.rendered_templates or self.templates
            self.info("模板变量:")
            for key, value in vars_to_show.items():
                print(
                    f"     {Colors.YELLOW}{Colors.BOLD}{key}{Colors.RESET} {Colors.DIM}→ {value}{Colors.RESET}"
                )

    def _setup_zshrc_stub(self, zshrc: Path) -> None:
        stub_content = (
            "# DOTFILES_MANAGED: generated by dotfiles/install.py\n"
            "# dotfiles 配置在 ~/.zshrc_dotfiles（由 install.py 管理，请勿直接编辑）。\n"
            "# 软件安装程序（conda、nvm 等）可安全地在此文件末尾追加内容。\n"
            "\n"
            'source "$HOME/.zshrc_dotfiles"\n'
        )
        if zshrc.is_symlink():
            self.act_remove(zshrc)
            if not self.dry_run:
                zshrc.unlink()
        elif zshrc.exists():
            first_line = zshrc.read_text().split("\n")[0]
            if self.ZSHRC_MARKER in first_line:
                self.act_skip("~/.zshrc 已管理，保留软件追加内容")
                return
            self.backup_file(zshrc)

        self.act_create(zshrc, "stub")
        if not self.dry_run:
            _ = zshrc.write_text(stub_content)

    def link_skills(self) -> None:
        if not self.skills_dir.exists():
            self.warn(f"skills 目录不存在: {self.skills_dir}")
            return
        with self.stage("链接 AI Skills", icon="◇"):
            for _, dir_name in [
                ("Codex", ".codex"),
                ("Copilot", ".copilot"),
                ("Claude", ".claude"),
            ]:
                tool_dir = self.home_dir / dir_name
                if tool_dir.exists():
                    self.create_symlink(self.skills_dir, tool_dir / "skills")
                else:
                    self.warn(f"未找到 ~/{dir_name}, 跳过")

    def run(self) -> None:
        if self.dry_run:
            print(
                f"\n  {Colors.BLUE}{Colors.BOLD}◈ DRY-RUN{Colors.RESET}  {Colors.DIM}仅预览，不执行任何操作{Colors.RESET}"
            )

        with self.stage("开始安装", Colors.GREEN, "★"):
            self.info(f"系统: {self.os_type}")
            if self.os_type == "other":
                self.error_exit("不支持的操作系统")

        if self.bootstrap:
            self.bootstrap_environment()

        with self.stage("安装通用配置"):
            self.process_directory(self.dotfiles_dir / "general")

        if self.os_type == "macos":
            with self.stage("安装 macOS 配置"):
                self.process_directory(self.dotfiles_dir / "macos")
        elif self.os_type == "linux":
            with self.stage("安装 Linux 配置"):
                self.process_directory(self.dotfiles_dir / "linux")

        self.link_skills()
        self.post_process()

        if self.dry_run:
            with self.stage("预览完成", Colors.BLUE, "◈"):
                self.success("未执行任何实际操作")
                self.info(self._stats_line())
        else:
            with self.stage("安装完成", Colors.GREEN, "✔"):
                self.success("所有配置已成功安装！")
                self.info(self._stats_line())
                self.info(f"备份目录: {self.backup_dir}")


def main():
    parser = argparse.ArgumentParser(
        description="dotfiles 安装脚本",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
示例:
  %(prog)s                    # 正常安装
  %(prog)s --bootstrap        # 先安装基础环境，再安装 dotfiles
  %(prog)s --dry-run          # 预览模式，显示将要执行的操作但不实际执行
  %(prog)s -h                 # 显示帮助信息
        """,
    )
    _ = parser.add_argument(
        "--dry-run",
        action="store_true",
        help="预览模式，显示将要执行的操作但不实际执行",
    )
    _ = parser.add_argument(
        "--bootstrap",
        action="store_true",
        help="安装全新环境所需的基础工具链，然后继续安装 dotfiles",
    )
    args = parser.parse_args()
    DotfilesInstaller(dry_run=args.dry_run, bootstrap=args.bootstrap).run()


if __name__ == "__main__":
    main()
