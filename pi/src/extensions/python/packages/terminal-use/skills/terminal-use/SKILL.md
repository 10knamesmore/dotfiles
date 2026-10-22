---
name: terminal-use
description: 用 python_repl 的 terminal_use 操作真实 Kitty 终端：发现或启动实例、连接已有程序、读取文字与截图、发送键盘鼠标输入并等待结果。用于交互式 CLI、REPL、TUI 和人工接管
---

# Terminal Use

通过 `python_repl` 使用 `import terminal_use as terminal`。使用 Session 对象操作终端，不自己实现 socket 协议、VT 解析或键盘编码。

## 选择目标

```python
import terminal_use as terminal

instances = terminal.list_instances()       # socket、实例 ID、Kitty PID、窗口概况
kitty = terminal.connect(socket=instances[0]["socket"])
windows = kitty.list_windows()
session = kitty.attach(window_id=windows[0]["id"])
```

`terminal.connect()` 不带参数时读取 `KITTY_LISTEN_ON`。未从 Kitty 中启动 Pi 或需要独立工作区时：

```python
session = terminal.launch(
    ["some-tui"], cwd="/path/to/project", cols=120, rows=40
)
```

`launch()` 创建新的 Kitty 实例，在唯一的初始桌面窗口运行指定程序，并返回该终端的 `Session`；省略程序参数时运行默认 shell。通过 `kitty = session.instance` 获取实例。只有需要额外桌面窗口时才调用 `kitty.open_window()`；`open_tab()` / `open_split()` 使用实例的当前活动位置。窗口 ID 仅在一个实例内有意义，不跨实例复用。

不显示初始桌面窗口时使用 `session = terminal.launch(hidden=True)`，直接通过返回的 `Session` 操作初始 shell。

## 交互

```python
screen = session.read(wait_for={"contains": "ready>"}, timeout=5)
session.send_text("hello")
session.send_key("enter")
screen = session.read(cells=True)
print(screen["text"])
# 需要查看图片或排版时再截图
display_image(session.screenshot())
```

- 键名使用 US 基础键位；修饰键分开传，例如 `send_key("ctrl", "c")`。`"A"` 不隐含 Shift；字面文字用 `send_text()`，大写按键用 `send_key("shift", "a")`。
- `paste()` 根据程序当前模式使用 bracketed paste。`write(bytes)` 是原始输入，不是输出读取。
- 鼠标坐标使用实际终端网格的 0 基列/行：`session.click(col=12, row=4)`。拖动每次 `mouse_move` 显式带 `button`；滚轮用 `scroll(direction="down", steps=3, col=12, row=4)`。程序没有启用相应鼠标报告时返回 `False`。
- 输入返回只表示编码/提交结果。后续应等待程序的独特提示或状态变化，不使用任意固定 sleep 代替业务条件。
- `read(rect=...)` 按终端 cell 裁剪文字与 cells；Python 字符索引不等于列坐标。
- `screenshot()` 返回目标终端分屏的 PNG bytes，使用 `display_image(session.screenshot())` 查看。要求官方 Kitty 0.49.2+，目标在活动 tab 中且未被布局隐藏。SDK 不会自动聚焦；需要切换时显式 `focus()`。不提供源图片导出。
- 长输出用 `history()`；shell 最近命令输出用 `command_output()`。不提供原始 PTY 输出。

## 生命周期

终端属于 Kitty，不属于 Python worker。worker 重启、cell 取消、变量丢弃均不关闭终端。之后可重新发现并 attach。

```python
session.focus()              # 展示给用户接管
session.detach()             # 停用句柄，终端继续运行
session = session.instance.attach(window_id=session.window_id)
session.close()              # 明确关闭终端
```

不要关闭用户原有终端来清理 Python 状态。`wait()` 等待窗口关闭，不是等待 shell 当前命令；后者使用 `wait_command(after=...)`，详见 API。Kitty 未报告退出码时 status 为 `closed`，不要当作成功退出。关闭后仍可读取最终文字与 cells，但不能再截图；异常终止 Kitty 可能无法保存关闭记录。

`cols/rows` 是期望尺寸，实际尺寸看 `read()["full_size"]` 或 `inspect()`。默认调整窗口/布局，不更改字号；`fit_grid()` 或 `resize(allow_font_resize=True)` 会调整同一桌面窗口所有终端共享的字号，需要明确接受这一影响。

方法级 timeout 必须小于当前 `python_repl` 调用剩余时间，给读取与收尾留余量；超时不会关闭终端。

## 参考

- [API](references/api.md)：方法签名、返回字段、输入事件、尺寸与退出状态。
- [交互流程](references/patterns.md)：TUI、shell 命令、截图和跨 worker 接管。
