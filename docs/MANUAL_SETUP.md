# 需要手动安装的配置项

<!--toc:start-->
- [需要手动安装的配置项](#需要手动安装的配置项)
  - [1. archlinuxcn 软件源（可选）](#1-archlinuxcn-软件源可选)
  - [2. fcitx5-macos 输入法（仅 macOS）](#2-fcitx5-macos-输入法仅-macos)
<!--toc:end-->

`dots sync` 只管理 `$HOME` 侧的符号链接，以下 root 级 / 需 GUI 交互的配置需要手动安装。

---

## 1. archlinuxcn 软件源（可选）

`packages/aur.txt` 中注释掉的 `archlinuxcn-keyring` 依赖此源。需要时手动配置：

```bash
# 1. 追加源到 /etc/pacman.conf
sudo tee -a /etc/pacman.conf << 'CONF'

[archlinuxcn]
Server = https://mirrors.tuna.tsinghua.edu.cn/archlinuxcn/$arch
CONF

# 2. 装 keyring（首次需信任密钥）
sudo pacman -Sy archlinuxcn-keyring
```

配置后可把 `packages/aur.txt` 里的 `archlinuxcn-keyring` 取消注释，部分 AUR 包（如 quickshell 生态）也可改走该源的二进制包。

---

## 2. fcitx5-macos 输入法（仅 macOS）

不在 brew 生态（认证成本原因走自有 installer 发布），且安装需 GUI 交互
（解锁未认证程序 + 管理员密码 + 注销重登），故不进 `packages/brew.txt`
和 toolchains 自动化。上游：[fcitx5-macos-installer](https://github.com/fcitx-contrib/fcitx5-macos-installer/blob/master/README.zh-CN.md)。

```bash
# 中州韵版（与 Linux 侧 fcitx5-rime 对应；rime 用户目录 ~/.local/share/fcitx5/rime）
cd /tmp && rm -rf Fcitx5Installer.app && curl -fsSLO https://github.com/fcitx-contrib/fcitx5-macos-installer/releases/download/latest/Fcitx5-Rime.zip && unzip Fcitx5-Rime.zip && open Fcitx5Installer.app
```

安装器弹出后：

1. 系统拦截未认证程序时：系统设置 → 隐私与安全性 → 仍要打开
2. 安装完成后**注销重登**（否则候选窗在全屏应用中不显示）
3. 若输入法菜单没出现小企鹅：键盘设置中删掉再于「简体中文」下手动重新添加
