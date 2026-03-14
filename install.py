#!/usr/bin/env python3
"""
dotfiles 安装脚本 (Python 版本)

功能：
  1. 检测操作系统类型（macos 或 linux）
  2. 备份已有配置文件到 dotfiles/backup/时间戳/ 目录
  3. 将 dotfiles 目录下的文件以符号链接方式安装到用户主目录，保持目录结构一致
  4. 支持通用配置和系统特定配置（general、macos、linux）
  5. 支持 dry-run 模式（预览将要执行的操作）

用法：
  python install.py                    # 正常安装
  python install.py --dry-run          # 预览模式，不实际执行
  python install.py -h                 # 显示帮助信息
"""

import sys
import shutil
import argparse
import platform
from datetime import datetime
from pathlib import Path
from typing import final
from contextlib import contextmanager


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
class DotfilesInstaller:
    ZSHRC_MARKER: str = "# DOTFILES_MANAGED:"

    def __init__(self, dry_run: bool = False):
        self.dry_run = dry_run
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
        self.ensure_directories()

    # ── 输出方法 ──────────────────────────────────────────────────────

    def _fmt_path(self, path: Path) -> str:
        """将 home 目录缩写为 ~"""
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

    # ── 核心逻辑 ──────────────────────────────────────────────────────

    def detect_os(self) -> str:
        system = platform.system()
        if system == "Darwin":
            return "macos"
        elif system == "Linux":
            return "linux"
        return "other"

    def ensure_directories(self) -> None:
        with self.stage("初始化", icon="◎"):
            for directory in [self.generated_dir, self.scripts_dir, self.backup_dir]:
                if not directory.exists():
                    self.act_mkdir(directory)
                    if not self.dry_run:
                        directory.mkdir(parents=True, exist_ok=True)

    def backup_file(self, target: Path) -> None:
        """备份目标文件或目录，或移除符号链接"""
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
        """创建符号链接"""
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
        """遍历目录并将文件以符号链接方式安装到主目录"""
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
        """渲染模板文件并创建符号链接"""
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
        """安装后的处理（模板替换等）"""
        with self.stage("模板处理", icon="⊙"):
            # 清理旧版遗留的 .zshrc.template 符号链接
            zshrc_template_link = self.home_dir / ".zshrc.template"
            if zshrc_template_link.is_symlink():
                self.act_remove(zshrc_template_link)
                if not self.dry_run:
                    zshrc_template_link.unlink()

            # 渲染 .zshrc_dotfiles.template → generated/.zshrc_dotfiles
            # 并创建符号链接 ~/.zshrc_dotfiles → generated/.zshrc_dotfiles
            zshrc_dotfiles_template = (
                self.dotfiles_dir / "general" / ".zshrc_dotfiles.template"
            )
            zshrc_dotfiles_target = self.home_dir / ".zshrc_dotfiles"
            self.render_template(zshrc_dotfiles_template, zshrc_dotfiles_target)

            # 处理 ~/.zshrc（stub 文件，保留软件追加内容）
            self._setup_zshrc_stub(self.home_dir / ".zshrc")

            # dry-run 时回退显示所有候选变量
            vars_to_show = self.rendered_templates or self.templates
            self.info("模板变量:")
            for key, value in vars_to_show.items():
                print(
                    f"     {Colors.YELLOW}{Colors.BOLD}{key}{Colors.RESET} {Colors.DIM}→ {value}{Colors.RESET}"
                )

    def _setup_zshrc_stub(self, zshrc: Path) -> None:
        """创建或保留 ~/.zshrc stub 文件"""
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
            else:
                self.backup_file(zshrc)

        self.act_create(zshrc, "stub")
        if not self.dry_run:
            _ = zshrc.write_text(stub_content)

    def link_skills(self) -> None:
        """配置 Codex / Copilot / Claude skills 软链接"""
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
        """运行安装程序"""
        if self.dry_run:
            print(
                f"\n  {Colors.BLUE}{Colors.BOLD}◈ DRY-RUN{Colors.RESET}  {Colors.DIM}仅预览，不执行任何操作{Colors.RESET}"
            )

        with self.stage("开始安装", Colors.GREEN, "★"):
            self.info(f"系统: {self.os_type}")
            if self.os_type == "other":
                self.error_exit("不支持的操作系统")

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
    """主函数"""
    parser = argparse.ArgumentParser(
        description="dotfiles 安装脚本",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
示例:
  %(prog)s                    # 正常安装
  %(prog)s --dry-run          # 预览模式，显示将要执行的操作但不实际执行
  %(prog)s -h                 # 显示帮助信息
        """,
    )
    _ = parser.add_argument(
        "--dry-run",
        action="store_true",
        help="预览模式，显示将要执行的操作但不实际执行",
    )
    args = parser.parse_args()
    DotfilesInstaller(dry_run=args.dry_run).run()


if __name__ == "__main__":
    main()
