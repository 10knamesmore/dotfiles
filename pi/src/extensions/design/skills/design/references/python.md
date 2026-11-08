# Python SDK

在现有 `python_repl` 中 `import pi_design`。查询结果是 Python dict/list。
项目文件使用现有文件工具读写，SDK 用于读取设计现场和项目操作。

## 模块方法

| 方法 | 结果与行为 |
| --- | --- |
| `list_projects()` | 返回全局项目列表；每项有 `id`、`name`、`path` |
| `open(path, *, name=None, start_preview=True)` | 注册并选中目录，返回 `Project`；默认启动项目的 Vite 预览 |
| `current()` | 返回当前会话选中的 `Project`；未选择时报错 |

`open` 接受绝对路径或 `Path`。缺失时创建 `design.json`、
`.design/workspace.json`，不生成 React 源码、不安装依赖。
同一目录重复打开复用项目身份。修改源码后无需重新 open。

目录还未准备好运行时可先注册：

```python
project = pi_design.open("/absolute/project/path", name="Player", start_preview=False)
```

安装依赖、写好 React 后，再调用默认的 `open` 启动预览。

## Project 属性与方法

| 接口 | 结果 |
| --- | --- |
| `.id` | 全局项目 ID |
| `.path` | 项目绝对目录 |
| `.url` | 浏览器工作区地址 |
| `.read_state()` | 完整设计状态 |
| `.read_changes(since=0)` | 指定版本之后的实际文件变化 |
| `.read_element(element_id)` | 当前元素的 `text`、`style` |
| `.read_artboard(artboard_id)` | 画板的名称、路由、位置和尺寸 |
| `.resolve_comment(comment_id)` | 标记评论已处理，返回当前状态 |
| `.undo()` / `.redo()` | 撤销、重做设计数据编辑，返回当前状态 |

选择另一个项目后，旧 `Project` 句柄不再代表当前作品；重新使用
`pi_design.current()` 获取当前句柄。

### read_state()

```python
state = project.read_state()
print(state["selection"])
print(state["design"]["parameters"])
print([c for c in state["workspace"]["comments"] if not c["resolved"]])
```

| 字段 | 含义 |
| --- | --- |
| `project` | `id`、`name`、`path` |
| `revision` | 当前服务中的设计版本 |
| `design` | `design.json` 当前内容 |
| `workspace` | `.design/workspace.json` 当前内容 |
| `selection` | 当前 `artboard` 和 `element`，未选择时为 `None` |
| `preview_url` | Vite 预览地址；未启动时为 `None` |
| `can_undo` / `can_redo` | 当前可用的历史操作 |
| `history` | 当前设计数据快照 ID 与历史条目摘要，条目有 `id`、`summary` |

历史快照包含 `design.json` 与 `.design/workspace.json`，不包含 React 源码。
在同一个项目保持打开期间可撤销、重做；切换项目或重启服务后重新记录。

### read_changes()

```python
changes = project.read_changes(since=state["revision"])
print(changes)
```

返回 `{ "revision": ..., "changes": [...] }`。每条变化包含
`revision`、`file` 和 RFC 6902 `patch`。包含用户编辑及外部文件修改，
读取不会提交、清除或确认这些变化。版本属于当前服务生命周期，重新打开
会话后先读取当前状态。

### 评论定位

元素评论：

```json
{ "artboard": "player", "element": "play-button" }
```

坐标评论：

```json
{ "artboard": "player", "x": 420, "y": 180 }
```

坐标单位是画板预览视口中的 CSS 像素，不是缩放后的工作区屏幕像素。
截图和 DOM 检查使用 browser-use；SDK 不提供截图方法。
