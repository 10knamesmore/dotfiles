---
name: nvim-use
description: 连接已有 Neovim，读取编辑现场、调用 LSP 并操作编辑器。用于用户指出问题时了解其上下文、Pi 修改代码后在编辑器中展示改动, 需要使用lsp操作, 辅助用户调试nvim等场景。
---

# Neovim Use

通过 `python_repl` 导入 `nvim_use`。先列举实例，再按项目和用户意图显式选择；每个 Neovim 只接受一个 Pi 会话。连接在 Python cells 之间保持。

```python
import nvim_use as nvim

print(nvim.discover())
editor = nvim.connect(instance_id="从 discover 选择的完整 ID")
```

`discover()` 返回登记摘要，包括 `instance_id`、`pid`、`socket`、`cwd`、`current_file`、`workspace_roots`、`connection`。状态栏显示当前模型、会话短 ID 和操作状态，模型切换后自动同步。用户可在 Neovim 执行 `:PiConnection` 查看连接及模型详情、`:PiDisconnect` 断开。占用中的实例需要原连接释放后才能连接。

## 原生编辑器 API

`editor.run(callback)` 向同步 callback 提供原生 `pynvim.Nvim`。所有原生对象操作都在 callback 内完成；可以保留对象供后续 callback 使用。返回需要打印的数据，再在 cell 主线程展示。

```python
def inspect(nv):
    return {
        "tabs": [tab.handle for tab in nv.tabpages],
        "windows": [
            {"id": win.handle, "buffer": win.buffer.handle, "cursor": win.cursor}
            for win in nv.windows
        ],
        "buffers": [{"id": buf.handle, "name": buf.name} for buf in nv.buffers],
        "current": {
            "tab": nv.current.tabpage.handle,
            "window": nv.current.window.handle,
            "buffer": nv.current.buffer.handle,
        },
    }

print(editor.run(inspect))
```

`nv.current.buffer` 是当前窗口显示的 buffer。buffer 可以出现在多个窗口；光标属于 window。原生 `window.cursor` 是 `(1-based row, 0-based UTF-8 byte column)`。原生 buffer 切片按 0-based 行号、右端不包含。

```python
text = editor.run(lambda nv: nv.buffers[buffer_id][10:30])
value = editor.lua("return vim.fn.getcwd()")
editor.command("copen")
```

读取与查询时保留用户的焦点、窗口和未保存修改。移动、修改、保存使用明确的原生操作。编辑已经打开的文件前读取 buffer，避免拿磁盘旧内容覆盖未保存编辑。

## LSP：按 URI 自行探索

先检查 client 的 workspace、能力和 `offset_encoding`，选择覆盖目标文件的 client。原生请求无需 buffer；参数和结果沿用该 client 的 LSP 编码。

```python
print(editor.lsp.clients())

result = editor.lsp.request(
    client_id=client_id,
    method="textDocument/definition",
    params={
        "textDocument": {"uri": "file:///project/src/config.rs"},
        "position": {"line": 41, "character": 8},
    },
)
print(result)
```

LSP 的 `line` 和 `character` 都从 0 开始。`character` 的单位按 client 的 `offset_encoding`，通常为 UTF-16。构造本机文件 URI 可用 `Path(path).resolve().as_uri()`。

`request()` 支持全部标准方法和 server 扩展，例如 references、hover、symbols、completion/resolve、rename、codeAction/resolve、formatting、callHierarchy、typeHierarchy。方法参数按 LSP 规范填写；请求结果由调用者决定如何展示或应用。

如果 server 要求文档已同步，显式加载到后台 buffer 并附着指定 client：

```python
buffer_id = editor.lsp.open_document(client_id=client_id, uri=uri)
```

已有 buffer 的未保存内容会保留；用户窗口和焦点保持不变。后续文本变更、保存和卸载用 Neovim 原生 API，由 Neovim 管理文档版本与同步。

## LSP：使用编辑现场

```python
buffer_id, cursor = editor.run(
    lambda nv: (nv.current.buffer.handle, nv.current.window.cursor)
)

result = editor.lsp.request(
    method="textDocument/references",
    buffer=buffer_id,
    editor_position=cursor,
    params={"context": {"includeDeclaration": True}},
)
```

`buffer + editor_position` 自动生成 URI、转换坐标。只有一个附着 client 支持方法时自动选它；多个候选时显式传 `client_id`。`params` 可补充方法参数，例如 `context`、`newName`。也可用 `editor.lsp.position_params(...)` 单独获取 `{client_id, params}`。

诊断来自 Neovim 当前缓存：

```python
print(editor.lua("return vim.diagnostic.get(...) ", buffer_id))
```

## 生命周期与执行

- `run()`、`lua()`、`command()` 默认 30 秒，`connect()` 默认 10 秒。时间单位均为秒。
- LSP `request()` / `execute_command()` 默认 30 秒；超时或 Python 中断会取消请求，连接保留。
- 原生 callback 超时或中断会关闭连接，阻止后续排队 callback。已经执行的修改保留。
- Neovim 退出、用户主动断开、worker 重置后，旧 handle 失效。重新 discover/connect；实例重启后 ID 改变。
- `editor.disconnect()` 只释放连接。Neovim、用户文件和 LSP 继续运行。
- 在 callback 内只用原生 `nv`，外层 `editor` / `editor.lsp` 在 cell 主线程调用。

更多原生编辑、事件订阅、rename、code action 示例见 [操作示例](references/operations.md)。包接口与日志位置见 [README](../../README.md)。
