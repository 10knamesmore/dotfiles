---
name: terminal-session
description: 用 python_repl 的 pi_terminal 驱动交互式 Unix 终端程序：启动 PTY、等待提示、发送按键与文本、读取屏幕或原始输出。适用于安装器、REPL、TUI 等需要真实终端交互的任务；一次性命令用 bash。
---

# Terminal Session

需要与真实终端交互时使用本 skill：用 `python_repl` 运行 Python 并 `import pi_terminal as terminal`。不要改用 `python`/`python3`、bash 管道或新增工具。

会话绑定当前 Python worker：worker 重启或 reload 会关闭并丢失全部会话，没有守护进程或跨会话持久化。运行时由 SDK 负责清理：解释器退出时自动关闭所有会话；cell 被取消或超时不等于当前 cell 创建的会话已经关闭。完成一个工作流后显式 `terminal.close(session_id)`，取消后用 `terminal.list()` 检查仍存活的会话。

`python_repl` 工具本身的 `timeout` 是整次 Python 调用的硬上限。`terminal.read(..., timeout=...)`、`terminal.wait(..., timeout=...)` 等方法级 timeout 只能占用外层调用的剩余时间，不能延长它；应为后续读取、收尾和异常处理预留时间，不要把方法级 timeout 设得等于或大于外层 timeout。

仅支持 macOS 与 Linux。每个会话只有一条 PTY 流：子进程的 stdout 与 stderr 合并，没有独立通道；屏幕由 Alacritty terminal core 解析。当前接口不返回 graphics、hyperlink 或剪贴板对象，不要把它当作完整图形终端。

## 路由

| 场景 | 文件 |
| --- | --- |
| 方法签名、返回字段、wait_for、输入事件与按键名 | [references/api.md](references/api.md) |
| 从启动到收尾的完整流程与故障处理 | [references/patterns.md](references/patterns.md) |
