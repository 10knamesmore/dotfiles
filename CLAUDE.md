## 项目概述

这是一个跨平台 dotfiles 仓库，当前真实安装入口是 `install.py`。

仓库的主要职责不是包管理，而是：

- 将 `general/`、`macos/`、`linux/` 中的配置链接到 `$HOME`
- 渲染 `general/.zshrc_dotfiles.template` 到 `generated/.zshrc_dotfiles`
- 维护一个可追加内容的 `~/.zshrc` stub
- 聚合脚本到 `generated/scripts/`
- 把 `general/skills/` 链接到各类 AI 工具目录

## 核心命令

### 安装与预览

```bash
# 实际安装 dotfiles
python install.py

# 预览将执行的变更
python install.py --dry-run
```

### 日常开发

```bash
# 快速跳转到 dotfiles 目录
dot

# 快速跳转到 skills 目录
skill
```

## 目录架构

### 配置来源

- `general/`: 跨平台通用配置
- `macos/`: macOS 专属配置
- `linux/`: Linux 专属配置
- `static/`: 被模板引用但不直接链接的静态资源
- `common/`: 共享资料，目前包含 VS Code 用户配置

### 生成与备份

- `generated/`: `install.py` 生成的文件
  - `.zshrc_dotfiles`: 由 `general/.zshrc_dotfiles.template` 渲染
  - `scripts/`: 从 `general/scripts/` 和平台 `scripts/` 聚合出的符号链接
- `backup/`: 安装时自动备份的旧文件

### AI 工具集成

- `general/skills/`: 自定义 AI skills 源目录
- 安装时会尝试链接到：
  - `~/.codex/skills`
  - `~/.copilot/skills`
  - `~/.claude/skills`

## 模板系统

### 模板变量

在 `.template` 文件中使用以下占位符，`install.py` 会负责替换：

| 变量 | 替换为 |
| --- | --- |
| `ZSH_CUSTOM_TEMPLATE` | `$DOTFILES_DIR/static/omz_custom` |
| `DOT_TEMPLATE` | `cd $DOTFILES_DIR` |
| `SCRIPTS_DIR_TEMPLATE` | `$DOTFILES_DIR/generated/scripts` |
| `SKILLS_DIR_TEMPLATE` | `$DOTFILES_DIR/general/skills` |

### 当前渲染流程

1. 复制 `general/.zshrc_dotfiles.template` 到 `generated/.zshrc_dotfiles`
2. 替换模板变量
3. 创建 `~/.zshrc_dotfiles -> generated/.zshrc_dotfiles`
4. 创建或保留 `~/.zshrc` stub，并在其中 `source "$HOME/.zshrc_dotfiles"`

## 符号链接规则

### 自动链接

- `general/`、`macos/`、`linux/` 下的普通文件会映射到 `$HOME`
- `*/.config/*` 下的一级子目录会映射到 `$HOME/.config/`
- `general/scripts/` 和平台 `scripts/` 会聚合到 `generated/scripts/`

### 特殊跳过项

- `skills/` 目录不走常规链接流程，而是由 `link_skills()` 单独处理
- `*.template` 文件不会直接链接到 `$HOME`
- `.DS_Store` 会被忽略

## 修改配置时的约定

1. 修改主 shell 配置时，编辑 `general/.zshrc_dotfiles.template`
2. 修改平台差异时，编辑 `macos/.zshrc_macos` 或 `linux/.zshrc_linux`
3. 修改模板后，运行 `python install.py` 重新渲染
4. 不要把 `~/.zshrc` 当作主配置文件维护，它是 install.py 生成的 stub
5. 新增配置文件后，运行 `python install.py` 创建链接

## 备份机制

- 安装前若目标是普通文件或目录，会移动到 `backup/时间戳/`
- 若目标是符号链接，会直接删除并重建
- `--dry-run` 只预览动作，不执行写入

## 当前仓库事实

- 顶层 `README.md` 是 `docs/README.md` 的符号链接
- `AGENTS.md` 是 `CLAUDE.md` 的符号链接
- 当前仓库没有顶层 `scripts/pkg-install` 或 `scripts/pkg-export`
- 当前仓库没有 `macos/Brewfile`、`linux/pacman.txt`、`linux/pacman-aur.txt`
- 如果代理需要描述安装方式，应以 `python install.py` 为准，不要再引用旧的 `install.sh`

## 主要配置工具

- Shell: Zsh with Oh My Zsh
- 编辑器: Neovim / Vim
- 终端: Kitty
- 文件管理器: Yazi
- 多路复用: Zellij
- 提示符: Starship
- macOS: yabai / skhd / sketchybar / fcitx5
- Linux: hypr / niri / waybar / systemd user config
