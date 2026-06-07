# packages/ — 装机清单与工具链

`dots bootstrap` 的数据源。改清单 = 改文本，不改代码。

## 清单角色

| 文件 | 后端 | 装什么 |
| --- | --- | --- |
| `pacman.txt` | Arch pacman | 系统底座 + 平台独占（fzf/yazi/zellij/kitty/桌面栈/字体） |
| `aur.txt` | paru（自动自举） | AUR 包（浏览器/输入法皮肤等） |
| `brew.txt` | macOS Homebrew（自动自举） | 同上 macOS 版；tap 包写全限定名自动 tap |
| `apt.txt` | Debian/Ubuntu apt | 只拿版本不敏感的底座（build-essential/git/zsh…） |
| `toolchains.toml` | shell 命令逐条幂等执行 | 跨平台工具链，见下 |

**单一渠道不变量**：被 `toolchains.toml` 用 cargo 管理的工具（uv/starship/zoxide/dust/rg/fd/bat/eza/delta），
**不再**出现在 pacman/brew 清单里——一个工具只有一个安装渠道，版本三平台一致。
apt 的现代 CLI 是化石（且二进制名被改成 batcat/fdfind），永远不从 apt 装它们。

## toolchains.toml 分组

`[节头]` 给条目分组（core/dev/ai/js），首个节头前的裸条目归 core。
机器范围在 `dots.lua` 的 host 块声明：

```lua
hosts({
    ["my-server"] = function()
        toolchains({ only = { "core" } })   -- 服务器：只要 shell 基线
        -- 或 toolchains({ skip = { "ai", "js" } })
    end,
})
```

不声明 = 全装；未知组名 bootstrap 时警告。详见 [LUA_API.md](../docs/LUA_API.md) 的 `toolchains(spec)`。

条目语义：键 = `command -v` 探测的二进制名（已存在则跳过）；`"name!"` 引号键 = 跳过探测
每次执行（rustup 组件类，靠命令自身幂等）。**条目按文件序执行**——`rustup-stable!` 必须
排在 cargo 工具之前（cargo install 按 MSRV 挑版本，陈旧 stable 会装到旧版甚至失败）。

## cargo 渠道的日常管理

cargo 自带账本（`~/.cargo/.crates.toml`），全生命周期：

```bash
cargo install --list             # 账本：装了什么、什么版本
cargo install --locked <crate>   # 安装/更新（有新版替换，已最新 no-op）
cargo install <crate>@1.2.3      # 钉版本
cargo uninstall <crate>          # 删除（二进制 + 账本记录）
```

批量管理走 `[dev]` 组的两件管理面工具：

```bash
cargo install-update -l          # 列出 已装 → 最新 对照（只看不动）
cargo install-update -a          # 全部更新
cargo cache                      # ~/.cargo 缓存占用透视
cargo cache --autoclean          # GC：删可再生缓存（解压源码/git checkout），常回收一半以上
```

为什么统一 `--locked`：cargo install 默认无视上游 Cargo.lock、安装时重新解析依赖——
得到的是无人测试过的依赖组合（编译失败或行为漂移）。`--locked` 复现上游 CI 验证过的
确定构建。升级必须是显式动作（`install-update`），不能是安装的副作用。

## 镜像

cargo 走 rsproxy（`tree/home/.cargo/config.toml`，bootstrap.sh 首编前会先落一份真实文件
解鸡生蛋）；rustup 下载走 `RUSTUP_DIST_SERVER=https://rsproxy.cn`。github raw 渠道
（nvm 等）国内裸连基本必挂，需要时 `proxy on` 再跑。

## fzf（唯一的手动项）

Go 写的、不在 crates.io，无国内稳定渠道；不进 toolchains——bootstrap 跑的时候代理
往往还没接上，清单条目必须不依赖代理。Arch/macOS 由 pacman/brew 提供；apt 机器
（apt 的 fzf <0.48 不支持 `--zsh`，不能用）等代理就绪后手动跑：

```bash
proxy on    # 代理地址不同就先 export DOTS_PROXY_URL=http://…
rm -rf /tmp/fzf-src && git clone --depth 1 https://github.com/junegunn/fzf.git /tmp/fzf-src
/tmp/fzf-src/install --bin          # 官方脚本：探测平台 → 下载 release 二进制，不碰 shellrc
install -Dm755 /tmp/fzf-src/bin/fzf ~/.local/bin/fzf
```

装完开新 shell 即可——`25-fzf-tab.zsh` 的存在性守卫会自动启用整套 fzf 功能
（缺 fzf 时则整模块跳过，Tab 回落原生补全，配置不报错）。
