# Terminal API

```python
import pi_terminal as terminal
```

所有方法使用 `terminal.start(...)['id']`（或 `list()`）返回的字符串会话 id。会话不存在、已关闭或 PTY 操作失败时抛 `RuntimeError`，参数不合法时抛 `ValueError`/`TypeError`。注册了 lifecycle hook 后，`start`、`close`、`close_all` 只能在注册 hook 的线程调用；跨线程调用会在产生副作用前抛 `RuntimeError`。未注册 hook 时不附加线程限制，其他方法没有此限制。

## 类型

下面是所有返回 dict 的完整 Python 结构。除特别标注外，字段始终存在

```python
from typing import Literal, Mapping, NotRequired, Sequence, TypedDict

Rect = terminal.Rect


class RunningStatus(TypedDict):
    kind: Literal['running']


class ExitedStatus(TypedDict):
    kind: Literal['exited']
    code: int
    signal: str | None


ProcessStatus = RunningStatus | ExitedStatus


class SessionInfo(TypedDict):
    id: str
    pid: int
    rows: int
    cols: int
    status: ProcessStatus
    generation: int
    reader_done: bool
    closed: bool
    error: str | None


class WaitOutcome(TypedDict):
    matched: bool
    timed_out: bool
    source: Literal['screen', 'raw']
    pattern_supplied: bool


class Size(TypedDict):
    rows: int
    cols: int


class RectValues(TypedDict):
    x: int
    y: int
    width: int
    height: int


class RectResult(TypedDict):
    requested: RectValues
    effective: RectValues


class CursorInfo(TypedDict):
    x: int
    y: int
    visible: bool
    inside_rect: bool
    relative_x: int | None
    relative_y: int | None


class DefaultColor(TypedDict):
    kind: Literal['default']


class IndexedColor(TypedDict):
    kind: Literal['indexed']
    index: int


class RgbColor(TypedDict):
    kind: Literal['rgb']
    red: int
    green: int
    blue: int


CellColor = DefaultColor | IndexedColor | RgbColor


class CellInfo(TypedDict):
    text: str
    wide: bool
    wide_continuation: bool
    foreground: CellColor
    background: CellColor
    bold: bool
    dim: bool
    italic: bool
    underline: bool
    inverse: bool


class ScreenSnapshot(TypedDict):
    session_id: str
    full_size: Size
    rect: RectResult
    lines: list[str]
    text: str
    cursor: CursorInfo
    alternate_screen: bool
    application_cursor: bool
    bracketed_paste: bool
    cells: list[list[CellInfo]] | None
    status: ProcessStatus
    generation: int
    raw_dropped_bytes: int
    reader_done: bool
    error: str | None
    wait: WaitOutcome


class RawOutput(TypedDict):
    session_id: str
    data: bytes
    bytes: int
    start: int
    end: int
    lost_bytes: int
    truncated: bool
    dropped_bytes: int
    status: ProcessStatus
    reader_done: bool
    error: str | None


class DrainOutcome(TypedDict):
    session_id: str
    status: ProcessStatus
    exited: bool
    drained: bool
    timed_out: bool
    reader_done: bool
    error: str | None


class RectMapping(TypedDict):
    x: int
    y: int
    width: int
    height: int


RectInput = Rect | RectMapping | tuple[int, int, int, int] | list[int]


class ContainsWait(TypedDict):
    contains: str
    source: NotRequired[Literal['screen', 'raw']]


class RegexWait(TypedDict):
    regex: str
    source: NotRequired[Literal['screen', 'raw']]


WaitFor = ContainsWait | RegexWait


class KeyEvent(TypedDict):
    kind: Literal['key']
    key: str


class TextEvent(TypedDict):
    kind: Literal['text']
    text: str


class PasteEvent(TypedDict):
    kind: Literal['paste']
    text: str


class RawEvent(TypedDict):
    kind: Literal['raw']
    data: bytes


TerminalEvent = KeyEvent | TextEvent | PasteEvent | RawEvent
```

`WaitFor` 要求且只要求 `contains` 或 `regex` 其中一个字段；`source` 默认是 `'screen'`。`cells` 未请求时仍有这个字段，但值为 `None`。

## start

```python
terminal.start(
    argv: Sequence[str],
    *,
    cwd: str | None = None,
    env: Mapping[str, str] | None = None,
    cols: int = 80,
    rows: int = 24,
) -> SessionInfo
```

- `argv`：程序与参数列表，`argv[0]` 按 `PATH` 查找；空列表、空程序名或非字符串参数报错。
- `cwd`：已存在的目录，默认继承 worker 当前目录。指定后，SDK 会把子进程的 `PWD` 同步为这个值；如果 `env` 显式提供 `PWD`，则以显式值为准。这是因为部分 TUI 会优先读取 `PWD` 决定初始目录，而不是调用 `getcwd()`。
- `env`：在 worker 环境之上添加或覆盖变量，不是隔离环境；未提供的变量（如 `PATH`、`HOME`）继续透传。当前没有清空继承环境或删除单个继承变量的参数。未在 `env` 中提供 `TERM` 时，SDK 会设置为 `xterm-256color`。
- 返回完整的 `SessionInfo`。

## list / inspect

```python
terminal.list() -> list[SessionInfo]
terminal.inspect(session_id: str) -> SessionInfo
```

`SessionInfo` 字段：

| 字段 | 类型 | 含义 |
| --- | --- | --- |
| `id` | `str` | 稳定会话 id，worker 生命周期内不变 |
| `pid` | `int` | 子进程 pid |
| `rows` / `cols` | `int` | 当前屏幕尺寸 |
| `status` | `ProcessStatus` | `{'kind': 'running'}` 或退出状态 |
| `generation` | `int` | 屏幕状态版本号；每次 PTY reader 成功读取一个输出 chunk、resize 或 reader 完成时递增。不表示输出事件数量、字节数或渲染帧数 |
| `reader_done` | `bool` | PTY 读取线程已结束，输出流不再变化 |
| `closed` | `bool` | 会话是否已经完成 close 清理 |
| `error` | `str \| None` | 会话生命周期中的错误；正常时为 `None` |

退出状态的完整结构是：

```python
{'kind': 'exited', 'code': int, 'signal': str | None}
```

## read

```python
terminal.read(
    session_id: str,
    *,
    rect: RectInput | None = None,
    wait_for: WaitFor | None = None,
    timeout: float = 0.0,
    trim_trailing_spaces: bool = True,
    cells: bool = False,
) -> ScreenSnapshot
```

无 `wait_for` 时立即快照，`timeout` 必须为 0；有 `wait_for` 时最多等待 `timeout` 秒，`timeout` 必须是有限值且大于 0。超时只结束等待、不杀进程，仍返回当前快照。

- `rect`：`Rect(x, y, width, height)`、`{'x': int, 'y': int, 'width': int, 'height': int}`、四元组或四元素列表。坐标是 0 基单元格，`width`/`height` 大于 0；超出屏幕的部分自动裁剪。rect 同时限定快照范围和 screen 匹配范围。
- `wait_for`：恰好包含 `contains` 或 `regex` 之一，可选 `source`：

```python
{'contains': 'ready> '}                       # screen 子串（默认）
{'regex': r'Done in \d+\.\d+s'}               # screen 正则
{'contains': 'Traceback', 'source': 'raw'}    # 匹配保留窗口内的原始输出
```

  空 pattern 报错。`wait` 始终存在，结构为 `WaitOutcome`。无等待条件时 `pattern_supplied` 为 `False`、`matched` 为 `True`、`timed_out` 为 `False`。raw 模式判断 bounded raw ring 当前保留窗口是否包含 pattern，不提供“本次调用之后的新出现”语义；二进制内容用 `read_raw` 精确判断。需要增量判断时，先记录 `read_raw()` 的 `end`，再用 `read_raw(since=end)` 自己维护偏移。
- `trim_trailing_spaces=False`：保留行尾空格；screen 来源的 `contains`/`regex` 匹配也使用这个设置。默认为 `True` 时，匹配和返回值都会去掉每行末尾的空格。
- `cells=True`：额外请求原生的逐单元格样式数据；否则 `cells` 为 `None`。

`ScreenSnapshot` 字段：

| 字段 | 类型 | 含义 |
| --- | --- | --- |
| `session_id` | `str` | 会话 id |
| `full_size` | `Size` | 完整终端尺寸 |
| `rect` | `RectResult` | 请求区域与裁剪后的有效区域 |
| `lines` | `list[str]` | rect 每行的逻辑文本，自顶向下，不足的行补空串；宽字符 continuation cell 不重复插入占位空格 |
| `text` | `str` | `\n`.join(`lines`) |
| `cursor` | `CursorInfo` | 光标在完整屏幕及 rect 中的位置 |
| `alternate_screen` | `bool` | 是否处于备用屏幕 |
| `application_cursor` | `bool` | 是否启用 application cursor mode |
| `bracketed_paste` | `bool` | 是否启用 bracketed paste mode |
| `cells` | `list[list[CellInfo]] \| None` | 请求样式数据时的逐单元格信息 |
| `status` | `ProcessStatus` | 读取时的进程状态 |
| `generation` | `int` | 快照对应的屏幕版本号；按 PTY reader chunk、resize 或 reader 完成递增，不是帧号或输出事件计数 |
| `raw_dropped_bytes` | `int` | 原始环形缓冲累计丢弃的字节数 |
| `reader_done` | `bool` | 读取线程是否结束 |
| `error` | `str \| None` | 屏幕读取错误；正常时为 `None` |
| `wait` | `WaitOutcome` | 本次等待的匹配结果 |

`RectResult` 的两个字段都是同样形状的 rectangle mapping；`requested` 是调用方请求的区域，`effective` 是裁剪到当前屏幕后实际使用的区域。`CursorInfo.relative_x` / `relative_y` 只有光标位于有效 rect 内时才有整数值，否则为 `None`。`lines` / `text` 面向自然文本和匹配，不保证 Python 字符索引或字符串长度等于终端列坐标；宽字符的 continuation cell 不重复输出，组合字符附加在所属 base cell 文本后。需要逐列布局时使用 `cells=True`；如果 rect 从宽字符的第二列开始，文本投影可能不包含该字符。

`CellInfo.foreground` 和 `background` 的形状分别是：

```python
{'kind': 'default'}
{'kind': 'indexed', 'index': int}
{'kind': 'rgb', 'red': int, 'green': int, 'blue': int}
```

颜色查询和像素尺寸查询：

- OSC 4/10/11/12 查询会收到 SDK 提供的 xterm-256color 默认 palette；OSC 设置过的动态颜色优先于默认值。
- CSI `18t` 返回当前字符行列；CSI `14t` 返回 `0×0` 像素尺寸，因为 SDK 没有 GUI surface 或真实 cell 像素大小。
- `foreground` / `background` 是 cell 的原始颜色属性，不是应用 inverse 后的最终显示颜色。`inverse=True` 时，渲染器自行交换或重算可见前景和背景。`indexed` 只表示调色板索引，不代表已经转换为 RGB。
- clipboard、graphics、hyperlink 和其他 GUI 事件仍不提供对象；clipboard 查询不会得到内容。

## read_raw

```python
terminal.read_raw(
    session_id: str,
    *,
    max_bytes: int = 65536,
    since: int | None = None,
) -> RawOutput
```

读取 bounded raw ring 中保留的原始输出，不做终端 emulator 解析；默认从当前保留窗口的最旧字节开始返回，而不是直接返回最新尾部：

| 字段 | 类型 | 含义 |
| --- | --- | --- |
| `session_id` | `str` | 会话 id |
| `data` | `bytes` | 精确原始字节 |
| `bytes` | `int` | 本次返回的字节数，等于 `len(data)` |
| `start` / `end` | `int` | 数据在流中的绝对偏移，`end` 为开区间端点 |
| `lost_bytes` | `int` | 因请求起点早于保留窗口而无法返回的字节数 |
| `truncated` | `bool` | 本次返回是否受 `max_bytes` 限制而截断 |
| `dropped_bytes` | `int` | 因环形缓冲容量被丢弃的累计早期字节数 |
| `status` | `ProcessStatus` | 读取时的进程状态 |
| `reader_done` | `bool` | 读取线程是否结束 |
| `error` | `str \| None` | 原始读取错误；正常时为 `None` |

`since` 指定起始绝对偏移，默认为最旧保留字节；`max_bytes` 默认只返回其中前 64 KiB。保留上限约 1 MiB，丢弃内容无法恢复。`read` 和等待都不消耗 raw 数据。需要持续读取时保存上一次返回的 `end`，再传给下一次 `since`。

## input / send_text / send_key / paste / write

```python
terminal.input(
    session_id: str,
    events: Sequence[TerminalEvent],
    *,
    delay: float = 0.0,
) -> None
terminal.send_text(session_id: str, text: str) -> None
terminal.send_key(session_id: str, key: str) -> None
terminal.paste(session_id: str, text: str) -> None
terminal.write(session_id: str, data: bytes) -> None
```

`input` 先验证整批事件，再写入 PTY；任一事件非法则整批不发送任何字节。默认 `delay=0` 时仍合并为一次写入。`Up`、`Down`、`Left`、`Right`、`Home`、`End` 会根据会话最近解析到的 application cursor mode 选择普通或 SS3 序列；需要严格控制字节时使用 `write()`。设置 `delay` 后，按事件逐个写入，并在相邻事件的成功写入之间等待指定毫秒数；第一个事件前和最后一个事件后都不等待。它用于模拟逐步的人类输入，不保证 PTY 读取端的 chunk 边界，也不替代等待应用状态。Playwright 的 `pressSequentially(..., {delay})` 使用同样的毫秒单位语义。事件结构见 `TerminalEvent`：

| `kind` | 必填字段 | 行为 |
| --- | --- | --- |
| `key` | `key: str` | 按键序列 |
| `text` | `text: str` | UTF-8 字面输入，不自动追加回车 |
| `paste` | `text: str` | bracketed paste 包裹 |
| `raw` | `data: bytes` | 原样字节 |

`key` 接受：

- 单个字符按字面发送（含 Unicode）；
- 命名键（大小写不敏感）：`Enter`/`Return`、`Tab`、`Escape`/`Esc`、`Backspace`/`BSpace`、`Delete`/`DC`、`Insert`/`IC`、`Up`、`Down`、`Left`、`Right`、`Home`、`End`、`PageUp`/`PgUp`/`PPage`、`PageDown`/`PgDn`/`NPage`、`Space`、`F1`–`F12`；
- 控制键：`C-a`–`C-z`、`C-@`、`C-[`、`C-\\`、`C-]`、`C-^`、`C-_`、`C-Space`、`C-?`；
- Alt 组合：`M-<单个字符>`，如 `M-x`。

`paste` 始终发送 `ESC[200~`、文本和 `ESC[201~`，不会检查应用当前是否启用 bracketed paste。调用前应读取当前快照并确认 `bracketed_paste is True`；该字段只表示 emulator 最近解析到的应用模式，应用和 TERM 配置仍可能动态改变它。若为 `False`，使用 `send_text`；需要完全自定义字节时使用 `write`/`raw`。模式不匹配时，应用可能把标记当作普通输入或按键序列处理，导致内容丢失或误操作。

`write` 把 bytes 写入 PTY master；kernel line discipline 和子进程 termios 仍可能转换、回显、缓冲或把它解释为信号，并不保证应用最终收到相同字节。大块输入在应用不读取时可能阻塞；先等待应用 ready/raw 状态，不要向未读取的 canonical/raw 程序写入无限大块数据。

对 Which/多键菜单等序列交互，使用小的 `delay` 可以避免把所有键压在同一时刻发出，但仍应等待稳定的前置提示，并用确认框标题、目标路径或最终状态等唯一锚点确认结果。短的通用 pattern 可能在调用前已经存在的屏幕内容或路径中立即命中；不要只等待 `/`、`42` 等短字符串，也不要只等待菜单说明中的通用词（例如 `Trash`、`Delete`）。

## resize / wait / signal

```python
terminal.resize(session_id: str, *, cols: int, rows: int) -> SessionInfo
terminal.wait(session_id: str, *, timeout: float = 5.0) -> DrainOutcome
terminal.signal(session_id: str, signal_name: str) -> None
```

- `resize` 更新 PTY 窗口和屏幕尺寸，返回更新后的 `SessionInfo`；全屏程序会收到窗口变化。
- `wait` 等待子进程退出并等待 PTY reader drain，返回完整的 `DrainOutcome`；超时后 `status` 仍可能是 running。`read` 的 `timeout` 不会杀进程。
- `signal` 把信号发给会话进程组，支持 `HUP`、`INT`、`TERM`、`KILL`、`QUIT`（可带 `SIG` 前缀，大小写不敏感）；未知信号名属于参数错误，抛 `ValueError`。

## close / close_all

```python
terminal.close(session_id: str, *, grace_ms: int = 500) -> SessionInfo
terminal.close_all(*, grace_ms: int = 500) -> None
```

- `close` 先发 `SIGTERM`，等待 `grace_ms` 毫秒后升级为 `SIGKILL`；`grace_ms` 最大按 30 秒处理。它随后尝试回收子进程并释放 PTY，但如果进程组或 reader 未能在最终等待窗口内结束，返回的 `SessionInfo.status` 仍可能是 `running`，而 id 之后不可再使用。需要保留最终屏幕时必须在 `close` 前调用 `read`。线程限制会在关闭前检查，因此跨线程失败不会销毁 session。
- `close_all` 关闭 worker 的全部会话，逐个关闭；某个会话失败也继续，最后抛出第一个错误。解释器退出时 SDK 已自动调用，正常流程只需显式关闭正在使用的会话。
- `close_all` 关闭 worker 的全部会话，逐个关闭；某个会话失败也继续，最后抛出第一个错误。解释器退出时 SDK 已自动调用，正常流程只需显式关闭正在使用的会话。
