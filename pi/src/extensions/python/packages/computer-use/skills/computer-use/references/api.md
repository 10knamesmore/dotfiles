# API

通过 `import computer_use as computer` 创建显式 `Desktop`。下面的捕获、键鼠和应用方法均属于桌面对象，不是模块级函数。每个桌面由独立原生线程执行操作，固定绑定一组 Wayland 与 Hyprland socket，不修改 Python 进程环境。

## 桌面与生命周期

| 调用 | 约定 |
| --- | --- |
| `computer.connect_host()` | 连接启动环境中的当前 Hyprland；独占加锁并等 Quickshell 接管提示就绪后返回。已占用或提示服务不可用时失败。 |
| `computer.create_background(*, size=(1920, 1080))` | 创建独立后台桌面，宽高为正整数像素，scale=1。本机须有 `Hyprland` 和 `kwin_wayland`；输出就绪后返回。 |
| `desktop.status` | 只读：`active`、`closed`、`revoked`、`disconnected`。 |
| `desktop.launch(argv, *, cwd=None)` | argv 是可执行文件及参数的字符串列表，不经过 shell；返回 PID。指定 cwd 时用该目录，否则 host 继承当前工作目录、background 继承创建桌面时的工作目录。应用使用该桌面的连接环境。 |
| `desktop.close()` | 幂等关闭并等待清理；先释放虚拟输入，再撤掉提示并释放锁。host 保留用户桌面和应用；background 退出自有桌面及应用。 |
| `with desktop:` | 进入时检查对象有效，退出时关闭。也可以直接 `with computer.create_background() as desktop:`。 |

当前桌面独占锁位于 `$XDG_RUNTIME_DIR/hypr/$HYPRLAND_INSTANCE_SIGNATURE/computer-use.lock`，使用非阻塞 `flock`；PID 仅作诊断。锁文件不删除，进程退出后内核释放锁。第二个连接即使来自同一 Python 进程也立即报 busy，不排队、不抢占。

用户断开或提示通道失效后，即使 Python 空闲也会清理并失效对象。`revoked`/`disconnected` 在后续 `close()` 后仍保留原因。失效后除状态查询和关闭外，所有桌面操作报 `RuntimeError`，已保存的 `desktop.hyprland` 也不能重连。需要继续时显式创建新对象；用户主动断开后须先取得继续操作的确认。

后台模式复用用户 HOME、PATH 和应用 XDG 环境，不是安全沙箱。KWin 自身使用临时配置目录；专用 Hyprland 禁用 XWayland，因此后台应用必须支持 Wayland。应用的单实例机制仍可能复用用户已启动的进程：需要独立窗口时使用应用自身的新实例参数或独立 profile，再用 `clients` 确认窗口实际属于此桌面。

worker 在 cell 失败/取消和正常退出时调用内部 `_close_all()`。该函数关闭所有活动桌面；`_release_inputs()` 仅释放各桌面的虚拟输入而保留连接，供内部清理使用。普通操作使用对象的 `close()`，不要自行替代 worker 生命周期处理。没有暂停或恢复后台桌面的接口。

## 捕获与指针

| 调用 | 约定 |
| --- | --- |
| `capture(*, monitor=None, rect=None, max_size=None)` | 默认捕获焦点显示器，返回同一帧的原始像素缓冲区与按需生成的图像。`max_size` 只影响图像。 |
| `move_pointer(x, y, *, relative_to=None)` | 默认桌面逻辑坐标；指定截图后使用其返回 PNG 的像素坐标。 |
| `click(x, y, *, button="left", count=1, relative_to=None)` | 移动后点击；`count` 为正整数，每次重复完整按下/松开。 |
| `drag(start, end, *, button="left", duration=0.3, relative_to=None)` | `start/end` 为 `(x,y)`，`duration` 为非负秒数；直线移动，失败也释放按钮。路径须经过有效显示器区域。 |
| `scroll(amount, *, horizontal=False)` | 在当前指针处滚动离散格数；正数向下或向右，0 不操作。 |

`button` 为 `left`、`right`、`middle`、`back` 或 `forward`。

`rect=(x,y,width,height)` 使用指定显示器内、方向正确的原始输出像素，不是桌面逻辑坐标。x/y 非负，宽高为正整数，整个矩形须在输出内；越界不裁掉溢出部分。`max_size` 为正整数，只限制图像最长边，不放大小图，也不缩放 buffer。裁剪偏移和图像缩放供 `relative_to` 转换；它接受图像像素坐标，不是 buffer 像素坐标，并拒绝其他桌面对象产生的截图。改变显示器布局后重新捕获。

`Capture` 由 `desktop.capture()` 返回，不能直接构造。桌面关闭后，已有图像和像素缓冲区仍可读取：

| 只读成员 | 含义 |
| --- | --- |
| `monitor: str` | 捕获的 Hyprland 显示器名称。 |
| `size: tuple[int,int]` | 图像的像素宽高，受 `max_size` 影响。 |
| `bounds: tuple[float,float,float,float]` | 捕获区域的桌面逻辑 `(x,y,width,height)`，含裁剪偏移，可有负原点。 |
| `buffer: PixelBuffer` | 同一区域的未缩放像素缓冲区，保留 compositor 的像素格式和位深。 |
| `_repr_png_() -> bytes` | 按需生成 8-bit RGB PNG，用于 `display_image(shot)` 或写文件；不重新捕获、不修改 buffer。10-bit 通道映射到 0–255，缩小时使用 Lanczos3。 |

`PixelBuffer` 不能直接构造，成员只读：

| 成员 | 含义 |
| --- | --- |
| `data: bytes` | 未编码像素字节；每个像素占 4 字节，保留原始通道、alpha 与未使用位。每次访问复制成 Python bytes。 |
| `size: tuple[int,int]` | 裁剪区域的原始像素宽高，不受 `max_size` 影响。 |
| `stride: int` | 相邻行的字节距离，等于 `size[0] * 4`；无行填充。 |
| `format: str` | compositor 提供的 `wl_shm` 格式：`argb8888`、`xrgb8888`、`abgr8888`、`xbgr8888`、`argb2101010`、`xrgb2101010`、`abgr2101010` 或 `xbgr2101010`。 |
| `channel_bits: int` | RGB 每个通道的位数，8 或 10。 |
| `rgb() -> list[list[tuple[int,int,int]]]` | `rows[y][x] = (R,G,B)`，保留原生整数值：8-bit 为 0–255，10-bit 为 0–1023；忽略 alpha，不缩放、不经过 PNG。 |

buffer 来自 Wayland 共享内存捕获。SDK 只重排方向、裁剪和去除行填充，不改写保留像素的 4 字节；它不是包含原始行填充和方向的整块共享内存副本。格式名称描述本机字节序的 32-bit 整数位布局，不是内存字节顺序。这里的原始值指 compositor 导出的值，不保证等同于应用源颜色或显示器最终颜色。当前捕获包含鼠标指针。

小区域颜色调试直接读取 `shot.buffer.rgb()`；大区域优先在 Python 内分析，避免把整个数组打印给模型。示例见 [读取区域 RGB 与原始字节](patterns.md#读取区域-rgb-与原始字节)。

## 键盘

| 调用 | 约定 |
| --- | --- |
| `press(*keys, count=1, interval=0.05)` | keys 是一个组合键，重复整个组合；`count` 为正整数，`interval` 为组合之间的非负秒数。 |
| `key_down(*keys)` | 整组名称验证后才发送事件，已持有的键不会重复获取。成功调用后跨 cell 保留。 |
| `key_up(*keys)` | 整组验证后释放当前桌面对象持有的对应键；未持有的键无效果。 |
| `hold(*keys)` | 返回 context manager；进入时获取，退出时只倒序释放本 scope 新获取的键。 |
| `type_text(text, /, *, interval=0.0)` | 通过 Unicode/XKB 键图输入，`interval` 为字符间额外等待的非负秒数；支持换行、制表符，不使用剪贴板。 |
| `held_keys()` | 按获取顺序返回当前桌面对象持有键的规范 XKB 名称元组；不查询物理键盘。 |
| `release_keys()` | 释放当前对象的全部键和虚拟修饰状态，不关闭桌面。 |

文本输入每字符发送后保留 4 ms 投递时间，避免 XWayland 应用尚未处理旧按键时就收到下一块文字的键图；`interval` 在此基础上增加间隔。切换到文字键图前还会等待先前的键盘输入，例如 Ctrl+A。协议返回不代表目标应用已完成处理，慢应用可增加 `interval`，操作后仍须观察结果。

`press()` 和内层 `hold()` 保留进入前已经持有的键。若整个组合都已持有，`press()` 不重新按下它们。作用域保存每次获取的标识；显式释放后重新获取的同名键不再属于旧作用域。`type_text()` 要求当前桌面对象没有持有键，其他控制字符请用 `press()`；实际应用和输入法仍可能处理 Unicode 按键，不能保证任意应用都会按文本提交。

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

`held_keys()` 和释放 API 只跟踪当前对象的虚拟设备。用户物理键盘的修饰键不能被它们释放，仍可能影响 host 应用。单次调用失败会回滚本次新获取的键和按钮；异常逃出 cell 时，worker 进一步关闭全部活动桌面。普通调用错误若被当前 cell 捕获且连接仍有效，不会关闭外层已持有的键。

## Hyprland

```python
hyprland = desktop.hyprland
hyprland.query("activewindow")
hyprland.dispatch("hl.dsp.focus({ workspace = 3 })")
```

`query(name)` 通过 Unix socket 请求 JSON，返回字典或列表。name 是单一查询名，例如 `clients`、`workspaces`、`monitors`、`activewindow`、`activeworkspace`、`cursorpos`、`layers`、`binds`、`globalshortcuts`；不接受命令批处理或带参数表达式。

`dispatch(expression)` 发送当前 Lua dispatcher 表达式，由 Hyprland 执行 `hl.dispatch(expression)`，成功返回 `None`。使用 `hl.dsp.*` 或当前配置已有的 dispatcher 表达式，不使用旧式字符串 dispatcher。SDK 不记录表达式内容。

参数语义错误通常抛出 `ValueError`；Python 类型或整数转换错误可能抛出 `TypeError` / `OverflowError`。协议、格式、占用与对象失效错误抛出 `RuntimeError`；socket 等待通常限制为 5 秒，超时以 `TimeoutError` 或 `RuntimeError` 报告。后台启动另有 compositor 启动与输出就绪等待，外层 `python_repl` timeout 应留出启动和清理时间。Python 取消保留原异常并等待桌面关闭。已经送达应用的动作不可回滚。
