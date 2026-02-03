# 🏠 Dotfiles

Personal dotfiles managed with symbolic links and package version management.

## 📂 目录结构

```plaintext
dotfiles/
├── macos/                  # macOS 专属配置
│   ├── .config/
│   │   ├── borders/       # Borders 窗口边框配置
│   │   ├── skhd/          # SKHD 快捷键配置
│   │   └── yabai/         # Yabai 窗口管理器配置
│   ├── .zshrc_macos       # macOS 专属 zsh 配置
│   └── Brewfile           # Homebrew 包列表
├── linux/                  # Linux 专属配置
│   ├── .config/
│   │   ├── i3/            # i3 窗口管理器配置
│   │   └── kitty/         # Kitty 终端配置
│   ├── .zshrc_linux       # Linux 专属 zsh 配置
│   ├── pacman.txt         # Pacman 官方包列表
│   ├── pacman-aur.txt     # AUR 包列表
│   └── scripts/           # Linux 专属脚本
├── general/                # 通用配置（跨平台）
│   ├── .config/
│   │   ├── nvim/          # Neovim 配置
│   │   ├── kitty/         # Kitty 终端配置
│   │   ├── yazi/          # Yazi 文件管理器配置
│   │   └── starship.toml  # Starship 提示符配置
│   ├── .alias             # Shell 别名
│   ├── .p10k.zsh          # Powerlevel10k 配置
│   └── .zshrc.template    # Zsh 配置模板
├── static/                 # 静态资源（不创建软链接）
│   └── omz_custom/        # Oh My Zsh 自定义插件和主题
│       ├── plugins/
│       └── themes/
├── generated/              # 动态生成的配置
│   ├── .zshrc             # 渲染后的 zsh 配置
│   └── scripts/           # 符号链接的脚本集合
├── scripts/                # 管理脚本
│   ├── pkg-export         # 导出包列表脚本
│   └── pkg-install        # 安装包脚本
├── backup/                 # 配置备份目录
│   └── 2024-11-22T16:00:00/  # 时间戳备份
├── install.sh              # 主安装脚本
├── PKG_MANAGEMENT.md       # 包管理详细文档
└── README.md               # 本文档
```

### 目录说明

| 目录 | 说明 |
|------|------|
| `macos/` `linux/` `general/` | 以 `~/` 为根目录的配置结构，会创建符号链接 |
| `static/` | 不创建软链接的配置文件（如 Oh My Zsh 插件） |
| `generated/` | 存放模板渲染后的配置，符号链接最终指向此目录 |
| `scripts/` | 管理脚本（包管理、更新等） |
| `backup/` | 安装时自动备份的旧配置 |

## 🚀 快速开始

### 新机器完整安装流程

```bash
# 1. 克隆仓库
git clone https://github.com/yourname/dotfiles.git ~/dotfiles
cd ~/dotfiles

# 2. （可选）预览将要安装的包
./scripts/pkg-install --dry-run

# 3. 安装系统包
./scripts/pkg-install

# 4. 安装 dotfiles 配置
./install.sh

# 5. 重新加载 shell
source ~/.zshrc
```

### 仅安装 dotfiles（不安装包）

```bash
git clone https://github.com/yourname/dotfiles.git ~/dotfiles
cd ~/dotfiles
./install.sh
```

## 📦 包管理

### 导出当前系统的包列表

```bash
# 导出包列表（会自动备份旧文件）
./scripts/pkg-export
```

### 在新机器上安装包

```bash
# 预览（推荐先执行）
./scripts/pkg-install --dry

# 实际安装
./scripts/pkg-install
```

更多详细说明请查看 [包管理文档](./PKG_MANAGEMENT.md)

## 🛠️ 自定义命令

安装后可用的自定义命令：

| 命令 | 功能 |
|------|------|
| `dot` | 快速 cd 到 dotfiles 项目目录 |
| `skill` | 快速 cd 到 skills 目录 |

## 🔄 日常工作流程

### 修改配置

```bash
# 1. 编辑配置文件
vim ~/.zshrc          # 或直接编辑 ~/dotfiles/general/.zshrc.template

# 2. 如果修改了模板文件，重新运行安装脚本
cd ~/dotfiles
./install.sh

# 3. 重新加载配置
source ~/.zshrc
```

### 安装新软件后同步

```bash
# 1. 安装软件
brew install neovim    # macOS
# 或
sudo pacman -S neovim  # Arch Linux

# 2. 导出包列表
cd ~/dotfiles
./scripts/pkg-export

# 3. 提交更改
git add .
git commit -m "brew: add neovim"
git push
```

### 同步到新机器

```bash
# 在新机器上
cd ~/dotfiles
git pull
./scripts/pkg-install    # 安装新增的包
./install.sh             # 更新配置
source ~/.zshrc
```

## 📝 模板系统

### 支持的模板变量

在 `.template` 文件中可以使用以下变量：

| 变量 | 说明 | 示例值 |
|------|------|--------|
| `ZSH_CUSTOM_TEMPLATE` | Oh My Zsh 自定义目录 | `/Users/wanger/dotfiles/static/omz_custom` |
| `DOT_TEMPLATE` | cd 到 dotfiles 的命令 | `cd /Users/wanger/dotfiles` |
| `SCRIPTS_DIR_TEMPLATE` | scripts 目录路径 | `/Users/wanger/dotfiles/generated/scripts` |

### 模板示例

**模板文件** (`general/.zshrc.template`):
```bash
export ZSH_CUSTOM=ZSH_CUSTOM_TEMPLATE
export PATH=SCRIPTS_DIR_TEMPLATE:$PATH
alias dot="DOT_TEMPLATE"
```

**渲染后** (`generated/.zshrc`):
```bash
export ZSH_CUSTOM=/Users/wanger/dotfiles/static/omz_custom
export PATH=/Users/wanger/dotfiles/generated/scripts:$PATH
alias dot="cd /Users/wanger/dotfiles"
```

## 🛠️ 故障排除

### 符号链接冲突

如果已存在配置文件，`install.sh` 会自动备份到 `backup/时间戳/` 目录。

```bash
# 查看备份
ls -la ~/dotfiles/backup/

# 手动恢复某个文件
cp ~/dotfiles/backup/2024-11-22T16:00:00/.zshrc ~/.zshrc
```

### Homebrew 未安装（macOS）

```bash
# 安装 Homebrew
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

# 或者运行 pkg-install 脚本会自动安装
./scripts/pkg-install
```

### 模板渲染问题

如果修改了模板但没有生效：

```bash
# 删除旧的渲染文件
rm -rf ~/dotfiles/generated/

# 重新运行安装脚本
./install.sh
```

### Oh My Zsh 未安装

```bash
# 安装 Oh My Zsh
sh -c "$(curl -fsSL https://raw.githubusercontent.com/ohmyzsh/ohmyzsh/master/tools/install.sh)"

# 然后运行 dotfiles 安装
./install.sh
```

### Starship 提示符不显示

```bash
# 检查 Starship 是否安装
starship --version

# 安装 Starship
brew install starship    # macOS
pacman -S starship       # Arch Linux

# 或运行包安装脚本
./scripts/pkg-install
```
