# Neovim Use

`nvim-use-sdk` 在 Pi 的持久 Python worker 内连接已有 Neovim。Python 导入名为 `nvim_use`；编辑器 API 使用原生 `pynvim`，LSP 复用 Neovim 已运行的 clients。

## 使用

Neovim 加载 dotfiles 的 `pi_bridge` 模块后登记实例。在 Pi 执行 `/nvim`，选择实例并连接；结果追加到会话消息，不触发回复。命令与 `python_repl` 共享同一个 Python worker：

```python
import nvim_use as nvim

editor = nvim.current()
print(editor.run(lambda nv: nv.current.buffer.name))
print(editor.lsp.clients())
editor.disconnect()
```

一个 Neovim 接受一个 Pi 会话。重复连接同一实例复用当前 handle；其他 Pi 和子 agent 会被拒绝。Neovim 的 `:PiConnection` 显示连接详情，`:PiDisconnect` 主动释放连接。lualine 显示模型名称、会话短 ID 和当前操作。`:PiConnection` 同时展示模型 ID 与 provider。

## 接口

| 入口 | 行为 |
| --- | --- |
| `discover()` | 读取活实例的登记摘要，不占用连接 |
| `connect(instance_id=..., timeout=10)` | 按完整 ID 显式连接 |
| `select(instance_id=..., timeout=10)` | 连接并设为当前选择，成功后释放上一个选择 |
| `current()` | 取回 `/nvim` 或 `select()` 选定的活连接 |
| `editor.run(callback, timeout=30)` | 同步 callback 接收原生 `pynvim.Nvim` |
| `editor.lua(code, *args, timeout=30)` | 原生 `nvim_exec_lua` |
| `editor.command(command, timeout=30)` | 原生 Ex command |
| `editor.on_notification(handler)` | 注册原生通知回调；`None` 清除 |
| `editor.lsp.clients()` | client ID、workspace、编码、能力、buffers |
| `editor.lsp.request(method=..., client_id=..., params=..., timeout=30)` | 任意标准／扩展 LSP 请求，返回原生结果 |
| `editor.lsp.request(method=..., buffer=..., editor_position=..., params=...)` | 从编辑器位置构造请求，`params` 补充方法字段 |
| `editor.lsp.position_params(method=..., buffer=..., editor_position=..., client_id=...)` | 单独返回 `{client_id, params}` |
| `editor.lsp.notify(client_id=..., method=..., params=...)` | 原生 LSP notification |
| `editor.lsp.open_document(client_id=..., uri=...)` | 后台加载并附着指定 client，返回 buffer ID |
| `editor.lsp.execute_command(client_id=..., command=..., buffer=..., timeout=30)` | 执行客户端或服务端 LSP Command |
| `editor.showcast.start(title=..., stops=[...])` | 当前代码窗口中发布带高亮与 Markdown 浮窗说明的 tour，不创建 tab 或分屏 |
| `editor.showcast.toggle_comment()` | 显示／隐藏当前站的说明 |
| `editor.showcast.toggle_diff()` | 显示／隐藏当前站的 diff，开关跨站保留 |
| `editor.showcast.stop()` | 结束 tour，清除高亮、diff 并恢复按键、原 buffer、视图和 winbar |
| `editor.disconnect()` | 断开连接，保留 Neovim、buffers 和 LSP |

超时单位是秒。LSP 参数的 `line` / `character` 使用原生 0-based 坐标和 client 编码。`editor_position` 使用 pynvim 光标的 `(1-based row, 0-based UTF-8 byte column)`。

`run()` 的原生对象留在该连接的 RPC 线程使用。callback 可以返回普通数据，或保留 handle 供后续 callback 使用。`editor` 和 `editor.lsp` 方法在创建连接的 Python 线程调用。

LSP 错误抛出 `LspError`，原始 ResponseError 保存在 `.response`。LSP 超时或中断取消请求并保留连接。原生 callback 超时或中断会关闭连接；已经执行的操作保留。执行任意阻塞 Python callback 时，Pi worker 的外层超时负责终止无法退出的线程。

Showcast 由 Neovim 管理站点切换，不需要 Python 持续运行。`n/N` 切换站点，`Tab` 切换说明显示，`<leader>up` 切换 diff，`Enter` 聚焦说明，`Esc` 返回代码，`q` 结束。它使用真实 buffer，保留未保存内容；站点坐标及临时映射的作用范围见 [skill](skills/nvim-use/SKILL.md#showcast带讲解的代码-tour)。

## 站点 diff 预览

```python
editor.showcast.start(
    title="配置修改",
    stops=[{
        "buffer": "/absolute/project/config.py",
        "position": (1, 0),
        "title": "启用配置",
        "comment": "绿色虚拟行是拟修改内容，原文未变。",
        "proposed_text": "enabled = True\n",
    }],
)
editor.showcast.toggle_diff()
editor.showcast.stop()
```

站点可选字段 `proposed_text` 是以 LF 分行的完整拟修改文本，不是 patch；空字符串表示删除全部内容。基准是发布 tour 时该 buffer 的内容，包括未保存修改。组装原文时读取 `endofline` 保留末尾换行状态。同一 buffer 的多个站点都与此基准比较，不累积前一站的拟议修改。

Neovim 的 `vim.text.diff` 计算行差异，extmarks 负责显示：删除行用 `DiffDelete` 高亮并标 `-`，新增内容以 `DiffAdd` 虚拟行显示并标 `+`。虚拟行不折行；`nowrap` 下可水平滚动。仅末尾换行不同也会标出末行，但不显示专门的换行符。

进入站点显示该站的 diff，离站清除。`<leader>up` 在代码和 comment buffer 都可切换显示；开关跨站保留，无 diff 的站点不改变开关。`Tab` 独立控制 comment，浮窗定位会避开可见 diff，包括新增虚拟行。同一源码 buffer 的其他窗口也会显示这些装饰。

结束或替换 tour、关闭代码窗口、断开连接时一并清理 diff。源码编辑或 buffer 卸载会使该 buffer 所有站点的 diff 失效，返回站点也不会恢复旧差异；重新读取源码并发布 tour 后才能继续预览。

Diff 没有独立会话，不额外创建 tab、window 或 buffer。原文、undo 历史和磁盘保持不变，原 buffer 仍可编辑。预览没有 apply/accept 操作，实际写入仍由 `apply_patch` 或其他明确写入工具执行；执行前重新读取真实 buffer，保留用户的未保存修改。

## 运行时数据与生命周期

- Linux：`$XDG_RUNTIME_DIR/pi-nvim/<instance-id>.json`；此机器的 runtime 目录为 tmpfs。
- macOS：`$TMPDIR/pi-nvim/<instance-id>.json`。
- 登记目录权限为 0700，文件为 0600。只存连接地址、实例身份、cwd、当前文件、workspace roots 和 Pi 连接摘要。
- Neovim 按编辑器事件合并更新登记；正文、光标移动和诊断不写入登记文件。
- RPC 握手核实实例身份并获取独占连接。socket 关闭后 Neovim 释放 ownership、取消该连接的 LSP 请求并清理 showcast 和 diff 预览。
- Python worker 收到 Pi session ID、cwd 和所选模型，在运行用户代码前初始化连接信息。Pi 切换模型时向 worker 发送状态更新：空闲时立即处理，正在执行的 cell 结束后处理。模型变化同步到所有已连接 Neovim，无需再次调用工具。
- Python 正常退出通过 atexit 断开；被强杀时 socket 关闭触发 Neovim 清理。
- Python 环境重置后重新执行 `/nvim`；会话中的旧连接消息不会恢复连接。Neovim 重启后重新发现新实例 ID。

Python 日志：`NVIM_USE_LOG`，默认系统临时目录的 `nvim-use-<worker-pid>.log`。Neovim 日志：`stdpath('state')/pi-nvim.log`。日志记录连接、操作结果、耗时和错误类别，不记录正文或 LSP payload。

## 安装与验证

本包作为 editable 本地依赖安装到 Python worker 的 uv 环境。仓库根运行：

```sh
uv sync --project pi/src/extensions/python --locked
uvx ty check --python pi/src/extensions/python/.venv/bin/python pi/src/extensions/python/packages/nvim-use/src/nvim_use
pnpm --dir pi run typecheck
stylua --check tree/home/.config/nvim/lua/pi_bridge
```

Pi `/reload` 后新 worker 加载包与 skill。Neovim 重启加载 bridge 和 lualine 组件。

使用流程与原生操作示例见 [skill](skills/nvim-use/SKILL.md) 和 [操作示例](skills/nvim-use/references/operations.md)。
