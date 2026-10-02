# 交互流程与故障处理

## 标准流程

```python
import terminal_use as terminal

info = terminal.start(["python3", "-i"])
session_id = info["id"]
terminal.read(session_id, wait_for={"contains": ">>> "}, timeout=10)

terminal.send_text(session_id, "print(40 + 2)\n")
terminal.read(session_id, wait_for={"contains": "42"}, timeout=10)

terminal.send_text(session_id, "exit()\n")
terminal.wait(session_id, timeout=5)
terminal.close(session_id)
```

启动后先等一个稳定锚点（提示符、菜单、完成标记），再发送输入；不要用 read 轮询循环或 sleep 猜时间。`read(wait_for=...)` 会先检查调用前已经存在的 screen/raw 内容，短 pattern 可能立即命中旧状态；优先使用带上下文的唯一标题、路径或完成标记。正则用 `{"regex": ...}`，匹配对象是 rect 范围内多行拼接的屏幕文本。多键菜单先等待菜单出现，必要时用 `input(..., delay=...)` 发出序列，再等待唯一的确认内容或最终状态；不要把菜单快捷键说明中的通用词当作确认框锚点。

需要完整输出时等待结束标记或 `wait` 到进程退出，再 `read` 收尾；命令完成前可能出现看似完成的中间输出。`python_repl` 的外层 timeout 包住整段代码，方法级 `read(..., timeout=...)` / `wait(..., timeout=...)` 不能超过外层调用剩余时间；应保留时间给最后的 `read`、`close` 和错误处理。

## 发送输入

```python
terminal.input(
    session_id,
    [
        {"kind": "text", "text": "answer"},
        {"kind": "key", "keys": ["enter"]},
    ],
    delay=50,  # 毫秒；按事件逐个发送
)
terminal.send_key(session_id, "ctrl", "c")  # 中断前台进程
terminal.send_key(session_id, "up")         # 历史或菜单导航
```

- 多行文本或需要应用按 bracketed paste 接收的内容才用 `paste`；发送前确认 `terminal.read(session_id)["bracketed_paste"] is True`。该状态受应用和 TERM 配置影响，可能动态改变；为当前状态为 False 的应用使用 `send_text`，应用不支持 bracketed paste 时也用 `send_text` 或 `{"kind": "raw", "data": b"..."}`。`paste` 始终发送开始/结束标记，模式关闭时应用可能把标记当按键处理。
- `input(..., delay=毫秒数)` 用于需要模拟逐个事件输入的 TUI 序列；它只控制 SDK 发出事件的时间间隔，不保证 PTY 读端如何分块，也不保证应用一定接受序列。方向键和 Home/End 会依据最近解析到的 application cursor mode 编码；需要发精确控制序列时使用 `write()`。
- 整批非法时 `input` 不发送任何内容；只有自己能接受部分生效时才拆成多次调用。
- 预期输出是字节时不猜编码：`read()` 的文本来自终端 emulator 的屏幕状态，ANSI/VT 控制码可能已被消费，需要原始字节用 `read_raw`。
- `write()` 成功只表示字节写入了 PTY master，不表示应用已经读取或接受全部内容。canonical 模式仍受系统行规程和 `MAX_CANON` 限制；大块单行输入可能被内核截断或丢弃。先等待应用进入 raw mode 或稳定 ready 状态，SDK 不自动分块改变输入语义。

## TUI 与局部读取

- 全屏程序切到备用屏幕时 `read()['alternate_screen']` 为 True，退出后回到主屏。
- 局部读取：`terminal.read(session_id, rect=(0, 0, 80, 24))`；`cursor` 报告光标位置、可见性及相对 rect 的坐标。
- 布局依赖窗口尺寸：换行或截断异常时先 `terminal.resize(session_id, cols=..., rows=...)` 再读。
- `start(cwd=...)` 会同步子进程的 `PWD`，除非 `env` 显式覆盖 `PWD`。其余环境变量默认从 worker 透传，`env` 是覆盖层而不是隔离环境。
- 需要颜色、样式或逐列布局判断时传 `cells=True`；普通文本和 `contains`/`regex` 匹配使用 `lines`/`text`。宽字符在 `lines`/`text` 中只出现一次，continuation cell 仍保留在 `cells` 中；组合字符附加在 base cell 文本后。Python 字符索引不等于终端列坐标。
- `foreground` / `background` 是原始 cell 属性；`inverse=True` 时，渲染器需要自行交换可见前景和背景。SDK 会回答 OSC 颜色查询和尺寸查询；像素尺寸由 `start(cell_size=(8, 16))` 的虚拟单元格几何计算，不是实际字体尺寸。

## 观察 Kitty 图片

等应用准备好后，再请求图片快照：

```python
screen = terminal.read(session_id, images=True)
print(screen["text"])
for image in screen["images"]:
    print(image.image_id, image.size, image.placements)
    display_image(image)
```

- 图片对象包含源图和放置元数据；`display_image()` 展示源图，不绘制终端文字，也不模拟裁剪或遮挡。
- 同一次读取中的文字与图片状态来自同一次快照；返回后 TUI 继续运行不改变已有图片。需要新画面时重新 `read(images=True)`。
- `images=[]` 表示当前活动屏幕没有可返回的、被 placement 引用的图片；不等于应用从未上传图片。只有上传而没有 placement 的图片不返回，主屏图片也不会出现在备用屏快照中。
- 虚拟 placement 表示 Unicode 占位符模式；它不提供具体占位字符的屏幕位置，不能用来断言最终图文布局。普通文字 `rect` 不过滤图片列表。
- 图片问题先检查虚拟 `cell_size` 和终端行列，再检查源图、placement 与原始协议输出。只有需要核对最终合成画面时，才改用真实 Kitty 窗口截图。

## 原始输出增量读取

```python
chunk = terminal.read_raw(session_id)
chunk["data"], chunk["start"], chunk["end"]
next_chunk = terminal.read_raw(session_id, since=chunk["end"])
```

`dropped_bytes > 0` 表示更早的内容已被环形缓冲丢弃，只能从返回的 `start` 继续。等待 raw 模式只判断当前保留窗口，不会自动等待 pattern 的下一次出现；需要增量语义时用 `read_raw(since=...)` 自己维护偏移。screen emulator 不保留可读 scrollback；长输出需要依赖 raw ring 或让子进程自行写文件。

## 进程结束、超时与终止

- `wait` 返回后检查 `status`：`{'kind': 'exited', ...}` 才算真实退出；超时不会杀进程。
- 卡住或需要中断：`send_key(session_id, "ctrl", "c")`，必要时 `signal(session_id, "TERM")`，最后 `close`。
- `close` 会尝试终止进程并回收 PTY；若进程组或 reader 未在最终等待窗口内结束，返回状态可能仍是 running，且该 id 之后不可用。需要保留最终屏幕时，必须在 `close` 前读取。
- cell 被取消或超时不等于当前 cell 创建的 terminal session 已关闭；先用 `list()` 检查，再对仍在使用的 id 显式 `close()`。
- 每个会话只有一条 PTY 流，stdout 与 stderr 合并；需要分开时在启动命令里重定向到文件或用 shell 包装。

## 失败处理

- 会话丢失或已关闭时报 `RuntimeError`：用 `list()` 确认当前会话，不要复用旧 id；worker 重启或 reload 后所有 id 都失效。
- 等待超时先读当前屏幕找原因（登录提示、权限、语法错误、分页器），再决定继续发送还是终止。
- 不要 kill 系统进程做清理：SDK 在解释器退出时 `close_all`；正常结束显式 `close` 即可。若 cell 中途取消，仍应按上一条检查并关闭自己创建的 session。
