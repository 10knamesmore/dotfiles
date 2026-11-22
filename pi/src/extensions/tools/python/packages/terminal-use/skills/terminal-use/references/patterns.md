# 交互流程

## 独立 TUI 与人工接管

```python
import terminal_use as terminal

session = terminal.launch(["nvim", "."], cwd="/path/to/project")
print(session.read()["text"])
session.send_key("escape")
session.send_text(":set number")
session.send_key("enter")
session.focus()
```

用户在同一个 Kitty 终端接管。返回 Agent 后重新读取屏幕，不根据旧快照继续发送坐标输入。只在确实要结束该程序时调用 `session.close()`。

## shell 命令与退出状态

```python
session = terminal.launch(cwd="/path/to/project")
command = session.inspect()["command"]
sequence = command["sequence"] if command is not None else 0
session.send_text("make")
session.send_key("enter")
outcome = session.wait_command(after=sequence, timeout=30)
print(outcome)
print(session.command_output())
```

`wait_command` 使用 shell integration；它不会把命令回显或已经存在的提示符当成完成。`session.wait()` 用于等待整个终端关闭。

直接在新的独立 Kitty 窗口运行程序时：

```python
job = terminal.launch(["some-interactive-program"])
job.send_text("answer")
job.send_key("enter")
print(job.wait(timeout=10))
print(job.read()["text"])  # 关闭前保存的最终画面
```

## 截图调试

```python
screen = session.read(cells=True)
print(screen["text"])
png = session.screenshot()
display_image(png)
```

截图包含该终端分屏实际渲染的文字和图片。目标必须处于活动 tab 且未被布局隐藏；若需要切换，先显式调用 `session.focus()`。截图返回后不随后续重绘变化。保存到文件使用 `Path("/tmp/terminal.png").write_bytes(png)`。

## 跨 Python worker 重连

```python
instances = terminal.list_instances()
print([(item["pid"], item["socket"]) for item in instances])
kitty = terminal.connect(socket=instances[0]["socket"])
print(kitty.list_windows())
session = kitty.attach(window_id=42)
```

worker 重启不会关闭 Kitty。先确认实例与窗口，不把不同实例中的同一个数字 ID 当成同一终端。

## 尺寸与输入

```python
print(session.resize(cols=120, rows=40))
print(session.read()["full_size"])
session.click(col=12, row=4)
```

窗口管理器可能改变实际尺寸。坐标来自当前快照；缩放字号会影响整个桌面窗口，只有接受该影响时才使用 `fit_grid()` 或 `allow_font_resize=True`。
