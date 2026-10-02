---
name: terminal-use
description: 用 python_repl 的 terminal_use 驱动交互式 Unix 终端程序：启动 PTY、等待提示、发送按键与文本、读取屏幕或原始输出。适用于安装器、REPL、TUI 等需要真实终端交互的任务；一次性命令用 bash。
---

# Terminal Use

需要与真实终端交互时使用本 skill：用 `python_repl` 运行 Python 并 `import terminal_use as terminal`。不要改用 `python`/`python3`、bash 管道或新增工具。

键名与 computer-use 采用相同的 US 基础键位表达：忽略大小写，组合键分开传参，如 `terminal.send_key(session_id, "ctrl", "c")`。`"A"` 不隐含 Shift；大写用 `send_key(session_id, "shift", "a")`，字面文字用 `send_text()`。批量按键事件使用 `{"kind": "key", "keys": ["ctrl", "c"]}`。PTY 只接收本次输入，不保留按住状态；不支持的组合会报错。完整支持范围见 [API](references/api.md)。

会话绑定当前 Python worker：worker 重启或 reload 会关闭并丢失全部会话，没有守护进程或跨会话持久化。运行时由 SDK 负责清理：解释器退出时自动关闭所有会话；cell 被取消或超时不等于当前 cell 创建的会话已经关闭。完成一个工作流后显式 `terminal.close(session_id)`，取消后用 `terminal.list()` 检查仍存活的会话。

`python_repl` 工具本身的 `timeout` 是整次 Python 调用的硬上限。`terminal.read(..., timeout=...)`、`terminal.wait(..., timeout=...)` 等方法级 timeout 只能占用外层调用的剩余时间，不能延长它；应为后续读取、收尾和异常处理预留时间，不要把方法级 timeout 设得等于或大于外层 timeout。

仅支持 macOS 与 Linux。每个会话只有一条 PTY 流：子进程的 stdout 与 stderr 合并，没有独立通道；文字与 Kitty 图片协议由 Ghostty 核心解析，不启动 GUI。用 `terminal.read(session_id, images=True)` 取得源图快照，再显式 `display_image(image)`；普通文本读取不会自动展示图片。图片不是整屏图文截图，placement 也不保证当前可见，尤其不能用虚拟 placement 推断 Unicode 占位符的屏幕位置。完整字段见 [API](references/api.md#图片快照)。

## 路由

| 场景 | 文件 |
| --- | --- |
| 方法签名、返回字段、wait_for、输入事件与按键名 | [references/api.md](references/api.md) |
| 从启动到收尾的完整流程与故障处理 | [references/patterns.md](references/patterns.md) |
