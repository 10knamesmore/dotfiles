# Terminal Use

Python SDK，用于发现、启动和控制真实 Kitty 终端。终端属于 Kitty，Python worker 重启不关闭窗口；用户与 Agent 操作同一个终端。

```python
import terminal_use as terminal

kitty = terminal.launch(cwd="/path/to/project")
session = kitty.open_window(["nvim", "."])
screen = session.read(cells=True)
display_image(session.screenshot())
session.send_key("ctrl", "c")
session.focus()
```

隐藏启动时使用 `terminal.launch(hidden=True)`，再通过 `kitty.list_windows()` 和 `kitty.attach(window_id=...)` 操作初始 shell。`open_window()` 仍会新建可见桌面窗口。

## 安装

使用官方 Kitty 0.49.2 或更新版本，`kitty` 与 `kitten` 必须在 PATH 中。macOS 通过 Homebrew 安装和升级：

```bash
brew install --cask kitty
brew upgrade --cask kitty
```

- Linux 的 Kitty 包由 `dots install` 安装。
- `dots sync` 的 Python hook 安装 SDK；`dots.lua` 的 directory resource 管理 socket 目录，持续将权限收敛到 `0700`，不管理其中的 socket。
- `dots.lua` 的 symlink resource 管理 `~/.local/share/terminal-use/bridge.py`，指向 SDK 的 Kitty watcher。
- 公共 Kitty 配置开启 socket-only IPC，地址为 `~/.local/state/kitty/control-{kitty_pid}.sock`。

从仓库根同步安装布局：

```bash
./dots.sh sync
```

SDK 不下载、编译或修改 Kitty。升级后，已经运行的旧实例需要重启才会使用新版本。

macOS 的官方 Kitty 0.49.2 在新建 OS window 中运行立即退出的程序时可能触发原生崩溃。一次性命令用 bash，`open_window()` 用于持续交互程序。

## 截图

`session.screenshot()` 调用官方 `kitten @ screenshot`，返回目标终端分屏的 PNG bytes。它包含实际渲染的文字、图片与排版，可直接交给 `display_image()`。目标必须位于活动 tab 且未被布局隐藏；SDK 不会自动切换焦点或布局。

`read()` 只返回文字、cells、光标与模式，不提供源图片或 placement 导出。截图必须在窗口关闭前显式获取，关闭记录中只保留文字与 cells。

## 代码结构

```text
src/terminal_use/
  instance.py          实例发现、连接、启动与窗口创建
  session.py           屏幕读取、输入、等待、尺寸与生命周期 API
  models.py            终端区域坐标
  transport.py         调用 Kitty 自带 kitten remote-control client
  bridge.py            在 Kitty 进程内读取屏幕、编码输入、记录关闭事件
```

SDK 不创建 PTY，不解析 VT 输出，不注册 worker 进程组 ownership，也没有退出时自动关闭终端的 hook。环境变量默认来自 Kitty，`env` 参数覆盖新终端的指定变量。

## 生命周期与状态

- 实例由 socket 定位，连接后获得 Kitty 进程内的唯一实例 ID；终端由实例 ID 与 window ID 定位。
- `detach()` 仅停用 Python 句柄；`close()` 明确关闭 Kitty window。
- 全局 watcher 为窗口保存最终文字与 cells。关闭记录位于 `~/.local/state/terminal-use/<instance-id>/<window-id>.json`；包含终端内容，应按用户数据管理。SDK 不自动删除这些记录。
- `wait()` 等待窗口关闭，超时不杀进程。Kitty 报告退出状态时返回 `exited` 和退出码/信号；没有报告时返回 `closed`，不推断退出成功。尤其不能保证极短命程序的退出码。进程可能已退出而后台程序仍持有终端，所以窗口关闭和主进程退出是两个事件。
- `wait_command()` 使用 shell integration 的命令序号与退出状态。只有启用了 shell integration 的 shell 命令有此信息。
- 连接需要正在运行的 Kitty。已有 Session 可以在 Kitty 退出后读取其已保存的最终快照；跨 worker 重连适用于仍运行的实例。
- 原始 PTY 输出、raw offset、reader drain、emulator generation 和虚拟 cell size 不属于此 SDK。
- SDK 启动的 Kitty 将诊断写入 `~/.local/state/terminal-use/control-<socket-id>.log`。

完整 API 与交互流程见 [skill](skills/terminal-use/SKILL.md)。
