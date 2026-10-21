# Terminal Use API

```python
import terminal_use as terminal
```

仅支持 macOS 与 Linux 图形环境，使用 PATH 中的官方 `kitty` 和 `kitten`，截图要求 0.49.2+。安装见包内 [README](../../../README.md)。

## 实例

```python
terminal.list_instances() -> list[dict]
terminal.connect(*, socket: str | None = None, timeout: float = 10.0) -> KittyInstance
terminal.launch(
    *, cwd: str | None = None,
    cols: int = 120, rows: int = 40,
    hidden: bool = False, timeout: float = 20.0,
) -> KittyInstance
```

- 发现目录：`~/.local/state/kitty/control-*.sock`。每项包含 `socket`、`id`、Kitty `pid` 和 `windows`。连接失败会报错，不自动切换实例。
- `connect()` 未给 socket 时读取 `KITTY_LISTEN_ON`；缺少该变量时显式传地址或使用 `launch()`。
- `launch()` 使用 PATH 中的 Kitty 创建独立进程、唯一 socket 和初始 shell；返回前等待 socket 可用。启动超时不关闭已经启动的应用。
- `hidden=True` 使用 Kitty 的 `--start-as=hidden` 隐藏初始桌面窗口；默认显示。通过 `list_windows()` 和 `attach()` 操作初始 shell。`focus()` 不负责取消隐藏，`open_window()` 仍会新建可见桌面窗口。
- 实例属性：`socket`、`id`、`pid`。终端窗口内的子进程 PID 与实例 PID 不同。
- transport timeout 限定每次 IPC 调用；操作失败抛 `RuntimeError`，进程调用超时抛 `subprocess.TimeoutExpired`。

```python
kitty.list_windows() -> list[dict]
kitty.attach(window_id: int) -> Session
kitty.open_window(argv=None, *, cwd=None, env=None, title=None, cols=None, rows=None) -> Session
kitty.open_tab(argv=None, *, cwd=None, env=None, title=None, cols=None, rows=None) -> Session
kitty.open_split(argv=None, *, cwd=None, env=None, title=None, cols=None, rows=None) -> Session
```

`argv` 是非空程序参数序列；省略时使用 Kitty 默认 shell。`env` 在 Kitty 的子进程环境上覆盖变量，不复制 Python worker 的完整环境。`cwd` 相对 SDK 当前目录解析。窗口列表包含窗口/标签/桌面窗口 ID、标题、行列、工作目录、子进程与前台进程信息，不返回环境变量。

`open_window` 新建桌面窗口；`open_tab` 与 `open_split` 在实例的当前活动位置创建。`cols/rows` 必须同时提供，是近似尺寸请求。

## Session 身份与元数据

```python
session.instance       # KittyInstance
session.window_id      # Kitty window ID
session.id             # "<instance-id>:<window-id>"
session.pid            # 窗口最初启动的子进程 PID
session.closed         # 是否已记录窗口关闭
session.inspect() -> dict
```

`inspect()` 包含 Kitty 窗口元数据，并增加：

| 字段                      | 含义                                          |
| ------------------------- | --------------------------------------------- |
| `session_id`              | 完整终端标识                                  |
| `os_window_id` / `tab_id` | 所属桌面窗口与标签                            |
| `rows` / `cols`           | 实际行列                                      |
| `cell_size`               | 实际像素宽高 `{"width": int, "height": int}`  |
| `font_size`               | 桌面窗口共享字号，单位 pt                     |
| `closed`                  | 是否已记录关闭                                |
| `status`                  | 窗口运行、已关闭或已知退出状态，见下文        |
| `command`                 | shell integration 命令状态，未记录时为 `None` |

命令状态：

```python
{"sequence": 1, "running": False, "command": "make", "exit_code": 0}
```

`sequence` 在每次收到命令开始事件时递增，仅对当前窗口有效。连接到正在执行的已有命令时，可能缺少此前的开始事件。

## 读取

```python
session.read(
    *, rect=None, wait_for: dict[str, str] | None = None,
    timeout: float = 0.0, trim_trailing_spaces: bool = True,
    cells: bool = False,
) -> dict
```

`rect` 接受 `terminal.Rect(x, y, width, height)`、同字段 dict 或四元素 tuple/list。坐标非负、宽高为正，超出实际网格的部分裁剪。

`wait_for` 恰好包含非空 `contains` 或 `regex`，匹配 rect 内的文字；必须提供正 timeout。无 wait_for 时 timeout 必须为 0。SDK 轮询快照，匹配已有文字也会立即返回；瞬时输出可能在两次快照之间被覆盖。

等待超时时返回最后取得的快照；若连首次快照都未能在时限内取得，则抛 `subprocess.TimeoutExpired`。

返回字段：

| 字段                       | 含义                                                                    |
| -------------------------- | ----------------------------------------------------------------------- |
| `session_id` / `window_id` | 目标终端                                                                |
| `full_size`                | `{"cols": int, "rows": int}`                                            |
| `cell_size`                | 实际像素宽高                                                            |
| `rect`                     | `requested` 与 `effective` 两个 rectangle                               |
| `lines` / `text`           | 行数组及用换行连接的文字                                                |
| `cursor`                   | 完整网格中的 `x/y/visible`；另有 `inside_rect`、`relative_x/relative_y` |
| `alternate_screen`         | 是否处于备用屏幕                                                        |
| `application_cursor`       | 应用光标键模式                                                          |
| `bracketed_paste`          | 应用粘贴模式                                                            |
| `cells`                    | 逐格数据；未请求为 `None`                                               |
| `status`                   | 当前或已保存的退出状态                                                  |
| `wait`                     | `matched`、`timed_out`、`pattern_supplied`                              |

`lines/text` 按终端当前活动屏幕读取，不跟随用户向上滚动历史的位置。宽字符 continuation 不重复出现在文本中。每个 cell 包含：

```python
{
    "text": str, "wide": bool, "wide_continuation": bool,
    "foreground": CellColor, "background": CellColor,
    "bold": bool, "dim": bool, "italic": bool,
    "underline": bool, "inverse": bool,
}
```

颜色是 `{"kind":"default"}`、`{"kind":"indexed","index":int}` 或 `{"kind":"rgb","red":int,"green":int,"blue":int}`。颜色属性未应用 `inverse`。

### 截图

```python
session.screenshot() -> bytes

png = session.screenshot()
display_image(png)
```

调用官方 `kitten @ screenshot --match id:<window-id>`，返回该分屏实际渲染的 PNG。目标必须仍存在、位于活动 tab，且未被布局隐藏；不满足时 Kitty 返回错误。SDK 不自动切换焦点或布局，不自动保存文件。返回的 bytes 可用 `Path(...).write_bytes(png)` 保存。

截图包含文字、图片与实际排版，不受 `read(rect=...)` 影响。

### 历史与命令输出

```python
session.history(*, ansi: bool = False) -> str
session.command_output(*, ansi: bool = False) -> str
```

`history` 返回当前屏幕与 Kitty 保留的 scrollback。`command_output` 返回 shell integration 识别的最近命令输出。两者要求窗口仍存在；关闭后可读的是保存的最终屏幕。

## 输入

```python
session.send_text(text: str) -> None
session.send_key(*keys: str) -> bool
session.paste(text: str) -> None
session.write(data: bytes) -> None
session.input(events: Sequence[dict], *, delay: float = 0.0) -> list[bool]
```

按键使用 US 基础键位，支持 Kitty 的命名键和修饰键 `ctrl/shift/alt/super`。键名忽略大小写，`"A"` 与 `"a"` 相同；大写用 `("shift","a")`。导航键支持修饰键，例如 `("ctrl","left")`、`("shift","tab")`。Kitty 按当前应用键盘模式编码，未产生输入时 `send_key()` 返回 `False`。

`paste` 在应用开启 bracketed paste 时包裹并清理嵌套终止序列，否则发送普通文字。`write` 发送原始输入字节，终端 line discipline 仍可能处理这些字节。

事件：

```python
{"kind": "key", "keys": ["ctrl", "c"]}
{"kind": "text", "text": "hello"}
{"kind": "paste", "text": "multi\nline"}
{"kind": "raw", "data": b"\x03"}
{"kind": "mouse", "action": "press", "col": 12, "row": 4, "button": "left"}
```

输入先解析事件，再提交 Kitty。`delay` 单位为毫秒，在相邻事件间等待；首尾不等待。该操作不提供整批 PTY write 原子性。

### 鼠标

```python
session.click(*, col, row, button="left", modifiers=()) -> list[bool]
session.mouse_down(*, col, row, button="left", modifiers=()) -> bool
session.mouse_up(*, col, row, button="left", modifiers=()) -> bool
session.mouse_move(*, col, row, button=None, modifiers=()) -> bool
session.scroll(*, col, row, direction, steps=1, modifiers=()) -> bool
```

坐标为实际完整网格的 0 基列/行，必须在屏幕范围内。按钮为 `left/middle/right`，滚轮方向为 `up/down/left/right`；拖动时每次 move 显式带 button。Kitty 按应用有效鼠标模式编码，未启用相应事件时返回 `False`。`click` 返回按下和松开的两个结果。

混合事件的 mouse `action` 为 `press/release/move/scroll`。所有事件带 `col/row`；press/release 带 button，move 可带 button，scroll 带 direction 与可选 steps。可用 modifiers 为 `ctrl/shift/alt/super`。

## 尺寸与焦点

```python
session.focus() -> None
session.resize(*, cols, rows, allow_font_resize=False) -> dict
session.fit_grid(*, min_cols, min_rows, min_font_size=8.0) -> dict
```

`resize` 请求近似尺寸：单终端 tab 调整桌面窗口，多终端 tab 调整分屏布局。返回 `{"requested":{"cols":...,"rows":...},"actual":{...}}`。窗口管理器可限制尺寸，尺寸不一致不算调用失败。

`allow_font_resize=True` 允许在尺寸不足时缩小共享字号。`fit_grid` 保持窗口大小，最多三次缩小字号，不低于 min_font_size；返回实际网格、字号与 `fits`。两者的字号变化影响整个 OS window。

## 信号、等待、关闭

```python
session.signal(signal_name, *, target="foreground") -> None
session.wait(*, timeout=5.0) -> dict
session.wait_command(*, after: int, timeout=5.0) -> dict
session.close(*, timeout=5.0) -> dict
session.detach() -> None
```

信号名称大小写不敏感，可带 `SIG` 前缀。target：

- `foreground`：Kitty 选定的前台进程。
- `process`：窗口最初启动的子进程。
- `process_group`：该子进程所属进程组。

退出状态：

```python
{"kind": "running"}  # 窗口仍存在
{"kind": "closed"}   # 窗口关闭，但 Kitty 未报告退出码
{"kind": "exited", "exit_code": int | None, "signal": int | None}
```

`wait` 等待关闭记录，返回 `{"closed":bool,"timed_out":bool,"status":...}`。超时不杀进程。只有 Kitty 明确报告进程退出时才返回 `exited` 及退出码/信号，`closed` 不表示成功退出。极短命进程可能在 watcher 观察之前退出；需要 shell 命令退出码时使用 `wait_command`。没有 raw reader/drain 状态。

`wait_command` 等待 sequence 大于 after 的 shell 命令结束，返回 `completed/timed_out/command`。调用前记录序号，再发送命令。要求 shell integration。

`close` 显式请求 Kitty 关闭窗口，再等待关闭事件；重复关闭已记录窗口不会重复发送请求。关闭后的 `inspect/read` 使用最终记录。记录位于 `~/.local/state/terminal-use/<instance-id>/<window-id>.json`，包含终端内容；异常终止 Kitty 可能丢失记录。

`detach` 停用当前 Python 句柄，不关闭窗口；其他句柄不受影响。重新使用时 `kitty.attach(window_id=...)`。没有解释器退出清理，也不通过 context manager 自动关闭。
