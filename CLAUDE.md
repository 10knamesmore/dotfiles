## 项目概述

这是一个跨平台 dotfiles 管理仓库，使用符号链接和模板系统来管理 macOS 和 Linux 的配置文件。

## 核心命令

### 安装与配置

```bash
# 安装 dotfiles（创建符号链接、渲染模板）
python install.py

# 预览模式（不实际执行操作）
python install.py --dry-run

# 导出当前系统的包列表（更新 Brewfile 或 pacman.txt）
./scripts/pkg-export

# 在新机器上安装包（预览模式）
./scripts/pkg-install --dry-run

# 实际安装包
./scripts/pkg-install
```

### 日常开发

```bash
# 快速跳转到 dotfiles 目录
dot

# 快速跳转到 skills 目录
skill
```

## 目录架构

### 配置文件组织

- **general/**: 跨平台通用配置（Neovim、Kitty、Yazi、Starship 等）
- **macos/**: macOS 专属配置（yabai、skhd、Brewfile）
- **linux/**: Linux 专属配置（i3、Hypr、Niri、Waybar、pacman.txt）
- **static/**: 不创建符号链接的静态文件（Oh My Zsh 自定义插件和主题）

### 生成与备份

- **generated/**: 模板渲染后的配置文件（符号链接指向此处）
  - `.zshrc`: 从 `general/.zshrc.template` 渲染而来
  - `scripts/`: 所有平台脚本的符号链接集合
- **backup/**: 安装时自动备份的旧配置（按时间戳组织）

### AI 工具集成

- **general/skills/**: 自定义 AI skills（Codex/Copilot/Claude 共享）
  - 安装时会自动链接到 `~/.codex/skills`、`~/.copilot/skills`、`~/.claude/skills`
  - 包含文档生成、测试、前端开发等多个 skills

## 模板系统

### 模板变量

在 `.template` 文件中使用以下占位符，`install.sh` 会自动替换：

| 变量                   | 替换为                            |
| ---------------------- | --------------------------------- |
| `ZSH_CUSTOM_TEMPLATE`  | `$DOTFILES_DIR/static/omz_custom` |
| `DOT_TEMPLATE`         | `cd $DOTFILES_DIR`                |
| `SCRIPTS_DIR_TEMPLATE` | `$DOTFILES_DIR/generated/scripts` |
| `SKILLS_DIR_TEMPLATE`  | `$DOTFILES_DIR/general/skills`    |

### 渲染流程

1. `install.sh` 将 `.template` 文件复制到 `generated/` 目录
2. 使用 `sed` 替换所有模板变量
3. 在 `$HOME` 创建符号链接指向 `generated/` 中渲染后的文件

## 符号链接规则

### 自动链接

- `general/`、`macos/`、`linux/` 下的文件和 `.config/` 子目录会自动链接到 `$HOME`
- `scripts/` 目录内容会链接到 `generated/scripts/` 并添加到 PATH

### 不链接

- `static/` 目录（需要被引用而非链接）
- `general/skills/` 目录（通过 `link_skills()` 函数专门处理）
- `.DS_Store` 文件（自动忽略）

## 包管理

### macOS (Homebrew)

- 配置文件: `macos/Brewfile`
- 导出: `brew bundle dump --force --file=macos/Brewfile`
- 安装: `brew bundle install --file=macos/Brewfile`

### Linux (Arch)

- 官方包: `linux/pacman.txt`
- AUR 包: `linux/pacman-aur.txt`
- 导出: 分别使用 `pacman -Qqe` 和 `pacman -Qqm`
- 安装: `sudo pacman -S --needed - < linux/pacman.txt`

## 修改配置文件

当修改配置文件时：

1. **直接修改已链接的文件**: 编辑 `$HOME/.zshrc` 等价于编辑源文件（因为是符号链接）
2. **修改模板文件**: 编辑 `general/.zshrc.template` 后必须运行 `python install.py` 重新渲染
3. **添加新配置**: 在对应平台目录添加文件后运行 `python install.py` 创建符号链接

## 特殊处理

### 平台特定逻辑

- `install.sh` 通过 `detect_os()` 自动检测系统类型（macos/linux）
- 只安装对应平台的配置文件
- `.zshrc` 会根据平台加载 `.zshrc_macos` 或 `.zshrc_linux`

### 备份机制

- 安装前会自动备份现有配置到 `backup/时间戳/`
- 符号链接会被直接删除（不备份）
- 普通文件和目录会被移动到备份目录

## 主要配置工具

- **Shell**: Zsh with Oh My Zsh
- **编辑器**: Neovim
- **终端**: Kitty
- **文件管理器**: Yazi
- **提示符**: Starship
- **窗口管理**:
  - Linux: i3 / Hypr / Niri
  - macOS: yabai + skhd
