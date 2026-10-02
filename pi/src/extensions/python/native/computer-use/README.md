# Computer Use

供 Pi Agent 的 `python_repl` 使用的 Linux Hyprland 原生 SDK。Rust 通过 PyO3 直接导出 `computer_use` 模块，实现截图、虚拟输入与 Hyprland IPC。Python 3.12+，PyO3 0.29.2 的 `abi3-py312` 扩展由 maturin 构建。macOS 必须跳过此包。

```python
import computer_use as computer
from computer_use import hyprland

monitors = hyprland.query("monitors")
shot = computer.capture(rect=(100, 100, 20, 10))
rows = shot.buffer.rgb()  # rows[y][x] = (R, G, B)，保留原生位深
data = shot.buffer.data  # 未编码的像素字节
display_image(shot)      # 同一帧按需编码成 PNG
```

完整调用约定见 [API](skills/computer-use/references/api.md)，操作示例见 [patterns](skills/computer-use/references/patterns.md)，Agent 入口见 [computer-use](skills/computer-use/SKILL.md)。`display_image` 是 Python worker 提供的全局能力，SDK 通过 `_repr_png_()` 提供图像，不依赖图像展示包。

## 运行条件

运行环境提供 `XDG_RUNTIME_DIR`、`WAYLAND_DISPLAY`、`HYPRLAND_INSTANCE_SIGNATURE`、libxkbcommon 与 xkeyboard-config。Hyprland 必须提供 `wl_output` v4、`wl_shm`、`zwlr_screencopy_manager_v1` v3、`zwlr_virtual_pointer_manager_v1` v2、`zwp_virtual_keyboard_manager_v1` v1。SDK 不调用 grim、hyprctl、剪贴板工具或提权输入设备。

输入必须在导入模块的 Python 线程执行。导入、`held_keys()`、未使用时的释放和关闭均不连接桌面。截图使用独立的短期 Wayland 连接，不创建输入设备；输入连接和虚拟设备按需创建，并随 worker 存活。

## 捕获、原始缓冲区与图像

`capture()` 从 Wayland 共享内存取得完整输出，修正旋转、镜像和协议的 Y 反转，再按原始像素裁剪。`Capture.buffer` 保留裁剪区域的 4 字节像素字、格式与位深，只移除行填充，不经过 PNG 编解码或 10-bit 到 8-bit 转换。`buffer.data` 提供字节，`buffer.rgb()` 提供原生位深的二维 RGB 元组数组。格式、字节序和尺寸约定见 [API](skills/computer-use/references/api.md#捕获与指针)。

`display_image(shot)` 通过 `_repr_png_()` 按需生成同一帧的 8-bit RGB PNG，不重新捕获；10-bit 通道仅在此处量化。`max_size` 只限制图像最长边，绝不放大或修改 buffer；默认图像不缩放。`rect` 以完整、方向正确的输出像素为单位，越界报错。

`Capture.size` 是图像尺寸，`Capture.buffer.size` 是未缩放区域尺寸。`Capture.bounds` 是区域的桌面逻辑矩形，包含负原点与裁剪偏移；`relative_to=shot` 接受图像像素坐标并转换为桌面逻辑坐标。改变显示器布局后重新捕获。`Capture` 与 `PixelBuffer` 没有公开构造器，属性只读。

## 输入与清理

键名使用 US 基础键位对应的 XKB 名称；组合键单独传参，例如 `press("ctrl", "a")`。`hold()` 在进入时获取键，在退出时倒序释放本次新获取的键。外层已按住的键不属于内层作用域，`press()` 也保留这些键。每次获取有独立标识，旧作用域不会释放显式释放后重新获取的同名键。

`type_text()` 把 Unicode 字符映射到专用虚拟键盘，支持中文、换行和制表符。调用前须释放本 worker 已按住的键；快捷键使用 `press()`。应用和输入法仍按键事件处理输入，SDK 不直接提交应用文本，也不修改剪贴板。

`held_keys()`、`key_up()`、`release_keys()` 只涉及 worker 自己注入的虚拟键。用户实际按住的物理修饰键不会被这些 API 释放，并且仍可能影响应用接收输入的结果。

成功的 `key_down()` 可以跨 `python_repl` 调用保留。单次操作失败会释放本次新获取的键和鼠标按钮；worker 必须在 cell 失败或取消时调用 `_release_inputs()`，释放跨调用保留的全部虚拟输入。`close()` 幂等释放并关闭连接，已注册 `atexit`，之后可以重连。无法撤销已经送达应用的点击或文字。进程被强制终止时无法运行 Python 清理；当前 Hyprland 的 `input:virtualkeyboard:release_pressed_on_close` 默认关闭，因此客户端断开不保证应用收到松键事件。worker 应优先正常关闭，给原生清理留出执行时间。

键盘事件之后的首个指针操作会等待 20 ms，给输入法异步转发按键及修饰状态留出处理时间。协议确认只表示 compositor 收到了请求，操作后仍需观察应用结果。

Socket 等待采用 25 ms 轮询并释放 GIL，普通请求每次最多等待 5 秒；等待期间检查 Python 信号。拖拽、文本与重复按键在事件间检查信号。图像处理在独立线程执行，等待结果时检查信号；取消后已开始的纯图像计算可能继续完成。释放输入时忽略待处理 Python 信号以完成清理，但仍有 5 秒等待上限。

## 构建与维护

仓库根目录下运行：

```sh
cargo fmt --manifest-path pi/src/extensions/python/native/computer-use/Cargo.toml --check
cargo check --locked --manifest-path pi/src/extensions/python/native/computer-use/Cargo.toml
uv build --wheel --out-dir pi/src/extensions/python/native/computer-use/target/dist pi/src/extensions/python/native/computer-use
```

保留 `Cargo.lock`。构建主机需要现有 Rust 工具链、C 链接器和 libxkbcommon；本包不安装系统依赖。不要为此个人配置仓库添加测试。编译之外，验证截图和只读查询；键盘鼠标操作在隔离桌面中验证，避免影响当前应用。

操作日志写入 `COMPUTER_USE_LOG` 指定路径，默认为临时目录中的 `computer-use-sdk.log`，到 1 MiB 后清空重写。日志只记录操作名、进程、生命周期与成功/失败，不记录截图、坐标、键名、输入文字、窗口内容或 Lua 表达式。

模块分工：`src/capture/` 处理原始像素、图像编码与坐标映射，`src/input/` 管理键盘、指针和所有权，`src/wayland.rs` 管理协议对象与事件，`src/hyprland.rs` 处理 JSON/Lua IPC，`src/wait.rs` 处理可中断等待，`src/logging.rs` 记录诊断。

实现参考 [Wayland Rust bindings](https://github.com/Smithay/wayland-rs)、[grim 输出变换](https://github.com/emersion/grim/blob/master/render.c)、[wtype 的 Unicode 键图](https://github.com/atx/wtype/blob/master/main.c) 和 [libxkbcommon](https://xkbcommon.org/doc/current/)。Quickshell 专用 IPC 与模块集成不属于本 SDK。
