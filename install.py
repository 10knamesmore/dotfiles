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

import os
import sys
import shutil
import argparse
import platform
from datetime import datetime
from pathlib import Path
from typing import Dict, List, Optional, Tuple
from contextlib import contextmanager


class Colors:
    """终端颜色常量"""

    BOLD = "\033[1m"
    GREEN = "\033[0;32m"
    YELLOW = "\033[1;33m"
    RED = "\033[0;31m"
    BLUE = "\033[0;34m"
    CYAN = "\033[0;36m"
    MAGENTA = "\033[0;35m"
    RESET = "\033[0m"


class DotfilesInstaller:
    def __init__(self, dry_run: bool = False):
        self.dry_run = dry_run
        self.dotfiles_dir = Path(__file__).parent.absolute()
        self.home_dir = Path.home()

        # 时间戳用于备份目录
        self.timestamp = datetime.now().strftime("%Y-%m-%dT%H:%M:%S")

        # 目录路径
        self.backup_dir = self.dotfiles_dir / "backup"
        self.generated_dir = self.dotfiles_dir / "generated"
        self.scripts_dir = self.generated_dir / "scripts"
        self.skills_dir = self.dotfiles_dir / "general" / "skills"

        # 模板变量映射
        self.templates = {
            "ZSH_CUSTOM_TEMPLATE": str(self.dotfiles_dir / "static" / "omz_custom"),
            "DOT_TEMPLATE": f"cd {self.dotfiles_dir}",
            "SCRIPTS_DIR_TEMPLATE": str(self.scripts_dir),
            "SKILLS_DIR_TEMPLATE": str(self.skills_dir),
        }

        # 渲染的模板记录
        self.rendered_templates: Dict[str, str] = {}

        # 检测操作系统
        self.os_type = self.detect_os()

        # 确保必要的目录存在
        self.ensure_directories()

    @contextmanager
    def stage(self, title: str, color: str = Colors.CYAN):
        """阶段上下文管理器，用于显示执行阶段"""
        # 计算标题长度（中文字符算2个宽度）
        title_width = self.str_width(title)
        border_width = title_width + 4  # 两边各留2个空格

        print(f"\n{color}{'━' * border_width}{Colors.RESET}")
        print(f"{color}  {title}{Colors.RESET}")
        print(f"{color}{'━' * border_width}{Colors.RESET}")
        yield
        print()

    def str_width(self, text: str) -> int:
        """计算字符串的显示宽度（中文字符算2个宽度）"""
        width = 0
        for char in text:
            # 中文字符的 Unicode 范围
            if "\u4e00" <= char <= "\u9fff":
                width += 2
            else:
                width += 1
        return width

    def indent(self, message: str, level: int = 1) -> str:
        """缩进消息"""
        indent_str = "  " * level
        return f"{indent_str}{message}"

    def info(self, message: str, indent_level: int = 1) -> None:
        """输出信息消息"""
        print(
            f"{Colors.GREEN}[info]{Colors.RESET}: {self.indent(message, indent_level)}"
        )

    def warn(self, message: str, indent_level: int = 1) -> None:
        """输出警告消息"""
        print(
            f"{Colors.YELLOW}[warn]{Colors.RESET}: {self.indent(message, indent_level)}"
        )

    def error(self, message: str, indent_level: int = 1) -> None:
        """输出错误消息"""
        print(
            f"{Colors.RED}[error]{Colors.RESET}: {self.indent(message, indent_level)}"
        )

    def action(self, message: str, indent_level: int = 2) -> None:
        """输出操作消息"""
        print(f"{Colors.MAGENTA}↳{Colors.RESET} {self.indent(message, indent_level)}")

    def success(self, message: str, indent_level: int = 1) -> None:
        """输出成功消息"""
        print(f"{Colors.GREEN}✓{Colors.RESET} {self.indent(message, indent_level)}")

    def error_exit(self, message: str) -> None:
        """输出错误消息并退出"""
        self.error(message, 0)
        sys.exit(1)

    def ensure_directories(self) -> None:
        """确保必要的目录存在"""
        with self.stage("初始化目录"):
            directories = [self.generated_dir, self.scripts_dir, self.backup_dir]
            for directory in directories:
                if not directory.exists():
                    self.action(f"创建目录: {directory}")
                    if not self.dry_run:
                        directory.mkdir(parents=True, exist_ok=True)

    def detect_os(self) -> str:
        """检测操作系统类型"""
        system = platform.system()
        if system == "Darwin":
            return "macos"
        elif system == "Linux":
            return "linux"
        else:
            return "other"

    def backup_file(self, target: Path) -> None:
        """
        备份目标文件或目录，或移除符号链接

        如果目标不存在，则不进行任何操作。
        如果是符号链接（无论是否有效），直接删除。
        如果是普通文件或目录，进行备份。
        """
        if not target.exists() and not target.is_symlink():
            return

        # 如果是符号链接，直接删除
        if target.is_symlink():
            self.action(f"删除符号链接: {target}")
            if not self.dry_run:
                target.unlink()
            return

        # 如果是普通文件或目录，进行备份
        if target.is_file() or target.is_dir():
            # 判断父目录是否是 .config
            parent_dir = target.parent.name
            if parent_dir == ".config":
                backup_path = self.backup_dir / self.timestamp / ".config" / target.name
            else:
                backup_path = self.backup_dir / self.timestamp / target.name

            # 确保备份目录存在
            if not self.dry_run:
                backup_path.parent.mkdir(parents=True, exist_ok=True)

            self.action(f"备份: {target} -> {backup_path}")
            if not self.dry_run:
                shutil.move(str(target), str(backup_path))
            return

        # 不支持的类型
        self.error_exit(f"不支持的类型! 请检查: {target}")

    def create_symlink(self, source: Path, target: Path) -> None:
        """创建符号链接"""
        # 确保目标目录存在
        if not self.dry_run:
            target.parent.mkdir(parents=True, exist_ok=True)

        # 备份或删除现有文件
        self.backup_file(target)

        self.action(f"链接: {target} → {source}")
        if not self.dry_run:
            try:
                target.symlink_to(source)
            except Exception as e:
                self.error_exit(f"创建符号链接失败: {target} -> {source}\n错误: {e}")

    def process_directory(
        self, source_dir: Path, target_base: Optional[Path] = None
    ) -> None:
        """
        遍历 source_dir 目录下的所有文件和文件夹
        并将文件以符号链接方式安装到用户主目录下，目录结构保持一致
        """
        if target_base is None:
            target_base = self.home_dir

        if not source_dir.exists() or not source_dir.is_dir():
            self.warn(f"目录不存在: {source_dir}")
            return

        # 遍历目录中的所有条目
        for source_path in source_dir.iterdir():
            source_entry = source_path.name

            # 忽略特定文件
            if source_entry == ".DS_Store":
                continue
            if source_entry == "skills":
                continue
            # 忽略 .template 文件（它们会被渲染，而不是直接链接）
            if source_entry.endswith(".template"):
                continue

            target_path = target_base / source_entry

            if source_entry == ".config" and source_path.is_dir():
                # 处理 .config 子目录
                for sub_source_path in source_path.iterdir():
                    sub_entry = sub_source_path.name
                    if sub_entry == ".DS_Store":
                        continue
                    if sub_entry.endswith(".template"):
                        continue

                    sub_target_path = target_path / sub_entry
                    self.create_symlink(sub_source_path, sub_target_path)

            elif source_entry == "scripts" and source_path.is_dir():
                # 处理 scripts 目录，链接到 generated/scripts
                for script_path in source_path.iterdir():
                    script_entry = script_path.name
                    if script_entry == ".DS_Store":
                        continue

                    script_target_path = self.scripts_dir / script_entry
                    self.create_symlink(script_path, script_target_path)

            else:
                # 普通文件或目录
                self.create_symlink(source_path, target_path)

    def render_template(self, template_path: Path, output_path: Path) -> None:
        """渲染模板文件"""
        generated_path = self.generated_dir / template_path.name.replace(
            ".template", ""
        )

        # 复制模板文件
        self.action(f"渲染模板: {template_path.name} → {generated_path.name}")
        if not self.dry_run:
            shutil.copy2(template_path, generated_path)

            # 替换模板变量
            content = generated_path.read_text()
            for key, value in self.templates.items():
                if key in content:
                    content = content.replace(key, value)
                    self.rendered_templates[key] = value

            generated_path.write_text(content)

        # 创建符号链接
        if output_path.exists() or output_path.is_symlink():
            if output_path.is_symlink():
                self.action(f"删除现有符号链接: {output_path}")
                if not self.dry_run:
                    output_path.unlink()
            elif output_path.exists():
                self.action(f"删除现有文件: {output_path}")
                if not self.dry_run:
                    output_path.unlink()

        self.action(f"创建符号链接: {output_path} → {generated_path}")
        if not self.dry_run:
            output_path.symlink_to(generated_path)

    def post_process(self) -> None:
        """安装后的处理（模板替换等）"""
        with self.stage("模板处理"):
            # 处理 .zshrc.template
            zshrc_template = self.dotfiles_dir / "general" / ".zshrc.template"
            zshrc_target = self.home_dir / ".zshrc"

            # 删除可能存在的 .zshrc.template 符号链接
            zshrc_template_link = self.home_dir / ".zshrc.template"
            if zshrc_template_link.exists() or zshrc_template_link.is_symlink():
                if not self.dry_run:
                    if zshrc_template_link.is_symlink():
                        zshrc_template_link.unlink()

            self.render_template(zshrc_template, zshrc_target)

            # 显示渲染的模板变量
            if self.rendered_templates:
                self.info("渲染的模板变量:")
                for key, value in self.rendered_templates.items():
                    self.action(
                        f"{Colors.YELLOW}{Colors.BOLD}{key}{Colors.RESET} → {value}"
                    )

    def link_skills(self) -> None:
        """配置 Codex / Copilot / Claude skills 软链接"""
        if not self.skills_dir.exists():
            self.warn(f"skills 目录不存在: {self.skills_dir}")
            return

        with self.stage("链接 AI Skills"):
            # Codex
            codex_dir = self.home_dir / ".codex"
            codex_skills = codex_dir / "skills"
            if codex_dir.exists():
                self.create_symlink(self.skills_dir, codex_skills)
            else:
                self.warn("未找到 ~/.codex, 跳过")

            # Copilot
            copilot_dir = self.home_dir / ".copilot"
            copilot_skills = copilot_dir / "skills"
            if copilot_dir.exists():
                self.create_symlink(self.skills_dir, copilot_skills)
            else:
                self.warn("未找到 ~/.copilot, 跳过")

            # Claude
            claude_dir = self.home_dir / ".claude"
            claude_skills = claude_dir / "skills"
            if claude_dir.exists():
                self.create_symlink(self.skills_dir, claude_skills)
            else:
                self.warn("未找到 ~/.claude, 跳过")

    def run(self) -> None:
        """运行安装程序"""
        if self.dry_run:
            print(f"\n{Colors.BLUE}{'━' * 40}{Colors.RESET}")
            print(f"{Colors.BLUE}        DRY-RUN 模式        {Colors.RESET}")
            print(f"{Colors.BLUE}      仅预览，不执行操作      {Colors.RESET}")
            print(f"{Colors.BLUE}{'━' * 40}{Colors.RESET}\n")

        with self.stage("开始安装", Colors.GREEN):
            self.info(f"系统检测: {self.os_type}")

            if self.os_type == "other":
                self.error_exit("不支持的操作系统")

        # 安装通用配置
        with self.stage("安装通用配置"):
            general_dir = self.dotfiles_dir / "general"
            self.process_directory(general_dir)

        # 安装系统特定配置
        if self.os_type == "macos":
            with self.stage("安装 macOS 配置"):
                macos_dir = self.dotfiles_dir / "macos"
                self.process_directory(macos_dir)
        elif self.os_type == "linux":
            with self.stage("安装 Linux 配置"):
                linux_dir = self.dotfiles_dir / "linux"
                self.process_directory(linux_dir)

        # 链接 skills
        self.link_skills()

        # 后处理（模板渲染）
        self.post_process()

        if self.dry_run:
            with self.stage("DRY-RUN 完成", Colors.BLUE):
                self.success("预览结束，未执行任何实际操作")
        else:
            with self.stage("安装完成", Colors.GREEN):
                self.success(f"备份目录: {self.backup_dir}")
                self.success("所有配置已成功安装！")


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
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="预览模式，显示将要执行的操作但不实际执行",
    )

    args = parser.parse_args()

    # 运行安装程序
    installer = DotfilesInstaller(dry_run=args.dry_run)
    installer.run()


if __name__ == "__main__":
    main()
