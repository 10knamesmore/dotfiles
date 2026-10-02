# API

所有函数通过 `import computer_use as computer` 使用。输入在导入模块的 worker 线程执行。

## 截图与指针

| 调用 | 约定 |
| --- | --- |
| `screenshot(monitor=None, *, rect=None, max_size=None)` | 默认捕获焦点显示器。完整输出先修正方向，再裁剪并缩小；默认保留原始像素。 |
| `move_pointer(x, y, *, relative_to=None)` | 默认桌面逻辑坐标；指定截图后使用其返回 PNG 的像素坐标。 |
| `click(x, y, *, button="left", count=1, relative_to=None)` | 移动后点击；`count` 为正整数，每次重复完整按下/松开。 |
| `drag(start, end, *, button="left", duration=0.3, relative_to=None)` | `start/end` 为 `(x,y)`，`duration` 为非负秒数；直线移动，失败也释放按钮。路径须经过有效显示器区域。 |
| `scroll(amount, *, horizontal=False)` | 在当前指针处滚动离散格数；正数向下或向右，0 不操作。 |

`button` 为 `left`、`right`、`middle`、`back` 或 `forward`。

`rect=(x,y,width,height)` 使用完整、方向正确的原始截图像素。x/y 非负，宽高为正整数，整个矩形须在图像内；越界不裁掉溢出部分。`max_size` 为正整数，限制返回图像最长边，不放大小图。裁剪偏移和缩放保存在截图中，供 `relative_to` 转换。改变显示器布局后重新截图。

`Screenshot` 由 `screenshot()` 返回，不能直接构造：

| 只读成员 | 含义 |
| --- | --- |
| `monitor: str` | 捕获的 Hyprland 显示器名称。 |
| `size: tuple[int,int]` | 返回 PNG 的像素宽高。 |
| `bounds: tuple[float,float,float,float]` | 捕获区域的桌面逻辑 `(x,y,width,height)`，含裁剪偏移，可有负原点。 |
| `_repr_png_() -> bytes` | 编码 PNG，用于 `display_image(shot)` 或写文件；不重新截图。 |

## 键盘

| 调用 | 约定 |
| --- | --- |
| `press(*keys, count=1, interval=0.05)` | keys 是一个组合键，重复整个组合；`count` 为正整数，`interval` 为组合之间的非负秒数。 |
| `key_down(*keys)` | 整组名称验证后才发送事件，已持有的键不会重复获取。成功调用后跨 cell 保留。 |
| `key_up(*keys)` | 整组验证后释放 worker 持有的对应键；未持有的键无效果。 |
| `hold(*keys)` | 返回 context manager；进入时获取，退出时只倒序释放本 scope 新获取的键。 |
| `type_text(text, /, *, interval=0.0)` | 通过 Unicode/XKB 键图输入，间隔为字符间非负秒数；支持换行、制表符，不使用剪贴板。 |
| `held_keys()` | 按获取顺序返回本 worker 持有键的规范 XKB 名称元组；不查询物理键盘。 |
| `release_keys()` | 释放本 worker 的全部键和虚拟修饰状态；未使用过输入时不连接。 |
| `close()` | 释放键与鼠标按钮并关闭连接，幂等；后续调用可以重连。 |

`press()` 和内层 `hold()` 保留进入前已经持有的键。若整个组合都已持有，`press()` 不重新按下它们。作用域保存每次获取的标识；显式释放后重新获取的同名键不再属于旧作用域。`type_text()` 要求 worker 没有持有键，其他控制字符请用 `press()`；实际应用和输入法仍可能处理 Unicode 按键，不能保证任意应用都会按文本提交。

常用别名与 XKB 名称（忽略大小写，`esc` 与 `escape` 等价）：

| 别名 | 规范名称 |
| --- | --- |
| `ctrl`, `shift`, `alt`, `super` | `Control_L`, `Shift_L`, `Alt_L`, `Super_L` |
| `enter`, `escape`, `tab`, `backspace` | `Return`, `Escape`, `Tab`, `BackSpace` |
| `delete`, `insert`, `home`, `end` | `Delete`, `Insert`, `Home`, `End` |
| `left`, `right`, `up`, `down` | `Left`, `Right`, `Up`, `Down` |
| `pageup`, `pagedown`, `space` | `Prior`, `Next`, `space` |

也接受 `Control_R`、`Shift_R`、`Alt_R`、`Super_R`、`F1` 等 US 键图基础层中存在的 XKB 名称。空格和 US 未加 Shift 的标点可直接作为键名：`` ` ``、`-`、`=`、`[`、`]`、`\`、`;`、`'`、`,`、`.`、`/`；也可使用对应 XKB 名称 `grave`、`minus`、`equal`、`bracketleft`、`bracketright`、`backslash`、`semicolon`、`apostrophe`、`comma`、`period`、`slash`。

字母 `a` 与 `A` 指同一个未加修饰的基础键位。大写用 `press("shift", "a")`；感叹号用 `press("shift", "1")`。字面文字使用 `type_text()`，例如 `type_text("A!")`。不要把组合键写成一个字符串。所有键名在第一条键事件前完成验证，组合键按下时先处理修饰键。

terminal-use 的共有键名、US 基础键位含义和分参数组合键写法相同，但支持范围不同：computer 发送按下/松开事件，可以持有键并与鼠标操作组合；terminal 只发送一次编码后的输入，不提供按住状态，也不支持所有桌面组合键。

`held_keys()` 和释放 API 只跟踪 worker 虚拟设备。用户物理键盘的修饰键不能被它们释放，仍可能影响应用。调用失败会回滚该次新获取的键和按钮；worker 在 cell 错误或取消时调用内部 `_release_inputs()` 清理跨 cell 保留的全部输入。普通使用不需要直接调用这个内部函数。

## Hyprland

```python
from computer_use import hyprland
hyprland.query("activewindow")
hyprland.dispatch("hl.dsp.focus({ workspace = 3 })")
```

`query(name, /)` 通过 Unix socket 请求 JSON，返回字典或列表。name 是单一查询名，例如 `clients`、`workspaces`、`monitors`、`activewindow`、`activeworkspace`、`cursorpos`、`layers`、`binds`、`globalshortcuts`；不接受命令批处理或带参数表达式。

`dispatch(expression, /)` 发送当前 Lua dispatcher 表达式，由 Hyprland 执行 `hl.dispatch(expression)`，成功返回 `None`。使用 `hl.dsp.*` 或当前配置已有的 dispatcher 表达式，不使用旧式字符串 dispatcher。SDK 不记录表达式内容。

参数语义错误通常抛出 `ValueError`；Python 类型或整数转换错误可能抛出 `TypeError` / `OverflowError`。协议、格式与 compositor 错误抛出 `RuntimeError`，等待超过 5 秒抛出 `TimeoutError`。取消信号按 Python 原有异常传播。已经送达应用的动作不可回滚。
