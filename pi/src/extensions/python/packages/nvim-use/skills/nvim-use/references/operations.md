# 原生操作示例

这些示例复用已经连接的 `editor`。按任务选择要返回的数据，避免一次读取全部 buffer 正文。

## 读取与修改 buffer

```python
def read_buffer(nv):
    buf = nv.buffers[buffer_id]
    return {
        "lines": buf[:],
        "changedtick": nv.api.buf_get_changedtick(buf),
        "modified": buf.options["modified"],
    }

before = editor.run(read_buffer)
```

原生修改进入 Neovim undo 历史；保存是单独操作。把版本检查和修改放在同一次 Neovim 执行中，避免中间穿插用户输入：

```python
editor.lua('''
local buffer, expected, first, last, lines = ...
assert(vim.api.nvim_buf_get_changedtick(buffer) == expected, 'Buffer changed')
vim.api.nvim_buf_set_lines(buffer, first, last, true, lines)
''', buffer_id, before["changedtick"], start_line, end_line, replacement_lines)
```

保存指定 buffer 可使用 `vim.api.nvim_buf_call`，不要为保存切换用户窗口：

```python
editor.lua('''
local buffer = ...
vim.api.nvim_buf_call(buffer, function() vim.cmd.write() end)
''', buffer_id)
```

## Tabs、windows 与导航

```python
print(editor.run(lambda nv: [
    {"tab": tab.handle, "windows": [win.handle for win in tab.windows]}
    for tab in nv.tabpages
]))

# 明确要求导航时才改变焦点。
editor.run(lambda nv: nv.api.set_current_win(window_id))
editor.command("vsplit")
```

## Rename 与 WorkspaceEdit

请求返回原生 `WorkspaceEdit`，先检查涉及的 URI 和修改，再显式应用：

```python
workspace_edit = editor.lsp.request(
    client_id=client_id,
    method="textDocument/rename",
    params={"textDocument": {"uri": uri}, "position": position, "newName": "load_config"},
)
print(workspace_edit)

editor.lua('''
local client_id, edit = ...
local client = assert(vim.lsp.get_client_by_id(client_id))
vim.lsp.util.apply_workspace_edit(edit, client.offset_encoding)
''', client_id, workspace_edit)
```

应用使用 Neovim 的原生编辑规则和文档版本检查。请求到应用期间用户可能继续输入；对未携带文档版本的编辑，应用前核对相关 buffer 的 `changedtick`。多文件操作按 Neovim 的顺序应用，失败时检查已经修改的文件。

格式化返回 `TextEdit[]`，用 `vim.lsp.util.apply_text_edits(edits, buffer_id, client.offset_encoding)` 应用。保存仍由任务明确决定。

## Code action 与 server command

```python
actions = editor.lsp.request(
    client_id=client_id,
    method="textDocument/codeAction",
    params={
        "textDocument": {"uri": uri},
        "range": {"start": position, "end": position},
        "context": {"diagnostics": []},
    },
)
print(actions)
```

选择具体 action。若 client 支持 resolve，按需调用 `codeAction/resolve`。先应用 action 的 `edit`，再执行其 `command`：

```python
result = editor.lsp.execute_command(client_id=client_id, command=action["command"])
```

`execute_command()` 走 Neovim `Client:exec_cmd`，因此也支持 Neovim 注册的客户端 command。原始 `workspace/executeCommand` 则通过 `request()` 直接发给 server。

## 原生事件订阅

handler 在 RPC 线程执行，收到原生 `nv`、通知名和参数。用线程安全队列把所需数据带回 Python cell：

```python
from queue import SimpleQueue

events = SimpleQueue()
editor.on_notification(lambda nv, name, args: events.put((name, args)))
editor.run(lambda nv: nv.api.buf_attach(buffer_id, False, {}))

# 在后续 cell 中读取事件。
while not events.empty():
    print(events.get_nowait())

editor.run(lambda nv: nv.api.buf_detach(buffer_id))
editor.on_notification(None)
```

其他事件可在 Lua 中创建 autocmd，使用当前连接的 channel 调用 `vim.rpcnotify`。创建的 autocmd 由调用者删除；disconnect 会关闭 socket 并结束订阅接收。

## 参考

- [pynvim 原生接口](https://pynvim.readthedocs.io/en/latest/api/nvim.html)
- [Neovim RPC 与 API](https://neovim.io/doc/user/api/)
- [Neovim LSP client](https://neovim.io/doc/user/lsp/)
- [LSP 规范](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/)
