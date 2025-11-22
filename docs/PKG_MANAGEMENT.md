# 📦 包管理器版本管理

<!--toc:start-->
- [📦 包管理器版本管理](#📦-包管理器版本管理)
  - [🎯 支持的包管理器](#🎯-支持的包管理器)
  - [📝 使用方法](#📝-使用方法)
    - [导出当前系统的包列表](#导出当前系统的包列表)
    - [在新机器上安装包](#在新机器上安装包)
  - [🔧 手动管理](#🔧-手动管理)
    - [macOS (Homebrew)](#macos-homebrew)
    - [Linux (pacman)](#linux-pacman)
    - [Linux (apt)](#linux-apt)
  - [📂 文件结构](#📂-文件结构)
  - [💡 最佳实践](#💡-最佳实践)
    - [1. 定期导出包列表](#1-定期导出包列表)
    - [2. 分离核心包和可选包](#2-分离核心包和可选包)
    - [3. 使用 .gitignore 排除备份文件](#3-使用-gitignore-排除备份文件)
    - [4. 新机器初始化流程](#4-新机器初始化流程)
  - [🔗 相关链接](#🔗-相关链接)
  - [📊 统计信息](#📊-统计信息)
<!--toc:end-->


本项目支持对系统包管理器安装的包进行版本管理，可以轻松在不同机器间同步软件环境。

## 🎯 支持的包管理器

| 操作系统 | 包管理器 | 配置文件 |
|---------|---------|---------|
| macOS | Homebrew | `macos/Brewfile` |
| Arch Linux | pacman | `linux/pacman.txt` |
| Arch Linux | AUR (yay/paru) | `linux/pacman-aur.txt` |
| Ubuntu/Debian | apt | `linux/apt.txt` |

## 📝 使用方法

### 导出当前系统的包列表

```bash
# 导出当前系统已安装的包
./scripts/pkg-export
```

**macOS (Homebrew)**:
- 自动导出 taps、brews、casks 到 `macos/Brewfile`
- 包含备份功能，旧文件会被备份为 `.backup.时间戳`

**Linux (pacman)**:
- `pacman.txt` - 官方仓库的包
- `pacman-explicit.txt` - 所有显式安装的包
- `pacman-aur.txt` - AUR 包（需要 yay 或 paru）

### 在新机器上安装包

```bash
# 预览将要安装的包（推荐先执行）
./scripts/pkg-install --dry-run

# 实际安装包
./scripts/pkg-install
```

## 🔧 手动管理

### macOS (Homebrew)

```bash
# 手动导出
cd ~/dotfiles
brew bundle dump --force --file=macos/Brewfile

# 手动安装
brew bundle install --file=macos/Brewfile

# 清理未在 Brewfile 中的包
brew bundle cleanup --file=macos/Brewfile

# 检查 Brewfile 状态
brew bundle check --file=macos/Brewfile
```

### Linux (pacman)

```bash
# 手动导出官方包
pacman -Qqe | grep -vxFf <(pacman -Qqm) > linux/pacman.txt

# 手动导出 AUR 包
pacman -Qqm > linux/pacman-aur.txt

# 手动安装官方包
sudo pacman -S --needed - < linux/pacman.txt

# 手动安装 AUR 包 (使用 yay)
yay -S --needed - < linux/pacman-aur.txt
```

### Linux (apt)

```bash
# 手动导出
apt-mark showmanual > linux/apt.txt

# 手动安装
xargs sudo apt install -y < linux/apt.txt
```

## 📂 文件结构

```
dotfiles/
├── macos/
│   └── Brewfile              # Homebrew 包配置
├── linux/
│   ├── pacman.txt            # Pacman 官方包
│   ├── pacman-explicit.txt   # Pacman 显式安装的包
│   ├── pacman-aur.txt        # AUR 包
│   ├── apt.txt               # APT 包
│   └── dnf.txt               # DNF 包
└── scripts/
    ├── pkg-export            # 导出脚本
    └── pkg-install           # 安装脚本
```

## 💡 最佳实践

### 1. 定期导出包列表

建议在安装新软件后及时导出包列表：

```bash
# 安装新软件后
brew install neovim
./scripts/pkg-export

# 提交到 git
git add macos/Brewfile
git commit -m "brew: add neovim"
git push
```

### 2. 分离核心包和可选包

在 Brewfile 中使用注释区分必备和可选软件：

```ruby
# ========== 核心工具 ==========
brew "git"
brew "neovim"

# ========== 可选工具 ==========
# brew "docker"  # 注释掉不常用的包
```

### 3. 使用 .gitignore 排除备份文件

```bash
echo "*.backup.*" >> .gitignore
```

### 4. 新机器初始化流程

```bash
# 1. 克隆 dotfiles
git clone https://github.com/yourname/dotfiles.git ~/dotfiles
cd ~/dotfiles

# 2. 预览将要安装的包
./scripts/pkg-install --dry-run

# 3. 安装包
./scripts/pkg-install

# 4. 安装 dotfiles
./install.sh
```

## 🔗 相关链接

- [Homebrew Bundle 文档](https://github.com/Homebrew/homebrew-bundle)
- [Arch Wiki - Pacman Tips](https://wiki.archlinux.org/title/Pacman/Tips_and_tricks)
- [APT Documentation](https://wiki.debian.org/Apt)

## 📊 统计信息

查看当前包数量：

```bash
# macOS
grep -c '^brew ' macos/Brewfile
grep -c '^cask ' macos/Brewfile

# Linux
wc -l linux/pacman.txt
wc -l linux/pacman-aur.txt
```
