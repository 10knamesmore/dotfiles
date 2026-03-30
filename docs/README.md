# Dotfiles

跨平台个人 dotfiles 仓库，当前以 `install.py` 为安装入口。

它的职责很明确：

- 将 `general/`、`macos/`、`linux/` 中的配置链接到 `$HOME`
- 渲染 `general/.zshrc_dotfiles.template` 到 `generated/.zshrc_dotfiles`
- 生成一个受管的 `~/.zshrc` stub，并从中加载 `~/.zshrc_dotfiles`
- 聚合 `general/scripts/` 和平台脚本到 `generated/scripts/`
- 将 `general/skills/` 链接到 `~/.codex/skills`、`~/.copilot/skills`、`~/.claude/skills`

## 仓库结构

```text
dotfiles/
├── common/                 # 非 Home 目录映射的共享配置
│   └── vscode/User/
├── docs/                   # 文档
├── general/                # 通用配置
│   ├── .alias
│   ├── .config/
│   ├── .zshenv
│   ├── .zshrc_dotfiles.template
│   ├── scripts/
│   └── skills/
├── macos/                  # macOS 专属配置
│   ├── .config/
│   └── .zshrc_macos
├── linux/                  # Linux 专属配置
│   ├── .config/
│   ├── .zshrc_linux
│   └── scripts/
├── static/                 # 被模板引用但不直接链接的静态资源
│   └── omz_custom/
├── generated/              # install.py 生成的文件与脚本聚合目录
├── backup/                 # 安装时的备份
├── install.py              # 主安装脚本
├── README.md               # 指向 docs/README.md 的符号链接
└── AGENTS.md               # 指向 CLAUDE.md 的符号链接
```

## 快速开始

```bash
git clone <your-repo-url> ~/dotfiles
cd ~/dotfiles

# 先看将要执行什么
python install.py --dry-run

# 实际安装
python install.py
```

安装完成后建议重新加载 shell：

```bash
source ~/.zshrc
```

## 安装行为

`install.py` 会按如下规则工作：

1. 自动检测当前系统是 `macos` 还是 `linux`
2. 先安装 `general/`，再安装对应平台目录
3. 跳过 `.DS_Store`、`skills/` 和所有 `*.template`
4. 将 `general/scripts/` 与平台 `scripts/` 链接到 `generated/scripts/`
5. 渲染 `general/.zshrc_dotfiles.template`
6. 创建或保留 `~/.zshrc` stub，使其他工具仍可安全向 `~/.zshrc` 末尾追加内容

## Zsh 结构

当前 Zsh 配置不是“直接把模板渲染成 `~/.zshrc`”，而是两层结构：

- `~/.zshrc`
  - 由 `install.py` 管理的 stub 文件
  - 只负责 `source "$HOME/.zshrc_dotfiles"`
- `~/.zshrc_dotfiles`
  - 指向 `generated/.zshrc_dotfiles`
  - 由 `general/.zshrc_dotfiles.template` 渲染而来
- `~/.zshrc_linux` / `~/.zshrc_macos`
  - 平台附加配置
  - 在 `~/.zshrc_dotfiles` 中按平台加载

这意味着：

- 想改主配置时，编辑 `general/.zshrc_dotfiles.template`
- 想改平台差异时，编辑 `linux/.zshrc_linux` 或 `macos/.zshrc_macos`
- 不要把 `~/.zshrc` 当作主配置文件维护

模板中目前使用的变量有：

| 变量 | 作用 |
| --- | --- |
| `ZSH_CUSTOM_TEMPLATE` | 指向 `static/omz_custom` |
| `DOT_TEMPLATE` | 生成 `dot` alias |
| `SCRIPTS_DIR_TEMPLATE` | 指向 `generated/scripts` |
| `SKILLS_DIR_TEMPLATE` | 生成 `skill` alias |

## 修改配置的正确方式

### 修改已有配置

- 修改 `general/`、`macos/`、`linux/` 中的源文件
- 如果改的是模板文件，执行一次 `python install.py`
- 如果改的是普通已链接文件，通常重新执行 `python install.py` 也没问题

### 添加新配置

把文件放到对应目录后重新运行：

```bash
python install.py
```

例如：

- 通用 CLI 配置放到 `general/`
- macOS 专属配置放到 `macos/`
- Linux 专属配置放到 `linux/`

## AI Skills

`general/skills/` 会在安装时链接到以下目录：

- `~/.codex/skills`
- `~/.copilot/skills`
- `~/.claude/skills`

因此这个仓库同时也是本地 AI skills 的来源仓库。

## 备份与冲突处理

安装时如果目标位置已经存在内容：

- 如果是符号链接，直接删除后重建
- 如果是普通文件或目录，移动到 `backup/<timestamp>/`

查看预览：

```bash
python install.py --dry-run
```

查看备份：

```bash
ls -la backup
```

## 当前仓库里实际存在的主要配置

- Shell: Zsh + Oh My Zsh
- Editor: Neovim / Vim
- Terminal: Kitty
- File manager: Yazi
- Multiplexer: Zellij
- Prompt: Starship
- macOS WM: Yabai + SketchyBar + skhd
- Linux WM: Hyprland / Niri / Waybar
- AI tooling: Codex / Copilot / Claude skills

## 注意事项

- 顶层 `README.md` 是指向 `docs/README.md` 的符号链接
- `AGENTS.md` 是指向 `CLAUDE.md` 的符号链接
- 当前仓库没有顶层 `scripts/pkg-install` 或 `scripts/pkg-export`
- 当前仓库也没有 `macos/Brewfile`、`linux/pacman.txt` 这类包清单文件

如果将来重新引入包管理脚本或清单，文档应与实际文件结构一起更新。
