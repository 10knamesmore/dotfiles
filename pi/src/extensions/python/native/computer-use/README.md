# Computer Use

供 Pi `python_repl` 使用的 Linux Hyprland SDK。Rust/PyO3 导出 `computer_use`；每个显式桌面对象拥有固定目标和独立执行线程。支持当前桌面接管、后台桌面、原始像素捕获、虚拟键鼠与 Hyprland IPC。Python 3.12+；macOS 跳过此包。

```python
import computer_use as computer

with computer.create_background(size=(1920, 1080)) as desktop:
    desktop.launch(["kitty", "bash"])
    print(desktop.hyprland.query("clients"))
    shot = desktop.capture(rect=(100, 100, 20, 10))
    rows = shot.buffer.rgb()
    display_image(shot)
```

已有窗口使用 `computer.connect_host()`。所有捕获、输入和 compositor 操作都从对象进入，不存在隐式模块级桌面。Agent 入口见 [computer-use](skills/computer-use/SKILL.md)，完整签名见 [API](skills/computer-use/references/api.md)，示例见 [patterns](skills/computer-use/references/patterns.md)。

## 运行条件

- 当前桌面：`XDG_RUNTIME_DIR`、`WAYLAND_DISPLAY`、`HYPRLAND_INSTANCE_SIGNATURE` 指向同一 Hyprland；Quickshell 配置加载仓库的 `computer-control` 服务。
- 后台桌面：本机 PATH 上已有 `Hyprland`、`kwin_wayland`，以及有效的 `XDG_RUNTIME_DIR`。SDK 不安装系统依赖。后台禁用 XWayland，应用须支持 Wayland。
- 两种模式均需 libxkbcommon、xkeyboard-config，以及 compositor 提供 `wl_output` v4、`wl_shm`、`zwlr_screencopy_manager_v1` v3、`zwlr_virtual_pointer_manager_v1` v2、`zwp_virtual_keyboard_manager_v1` v1。

导入模块不连接桌面。创建对象时建立 Wayland 连接，虚拟输入设备按首次使用创建；截图使用独立的短期连接。SDK 不调用 grim、hyprctl 或剪贴板工具，也不使用提权输入设备。

## 当前桌面接管

按 Hyprland 实例管理两个运行时文件：

```text
$XDG_RUNTIME_DIR/hypr/$HYPRLAND_INSTANCE_SIGNATURE/
  computer-use.lock    # flock 独占锁；内容为持有 worker PID，仅供诊断
  computer-use.sock    # Quickshell 接管提示连接
```

SDK 独立打开 lockfile 并以 `LOCK_EX | LOCK_NB` 加锁，成功后写 PID；同进程或跨进程的第二个连接都立即失败。锁 FD 不传给启动的应用，文件不 unlink。锁由内核维护，不以文件存在或 PID 存活判断占用。

加锁后 SDK 发送 hello，等 Quickshell 在所有屏幕显示提示后回复 ready，才允许操作。当前桌面显示淡紫边框、顶部断开按钮、Pi 虚拟光标和键鼠状态。移动动画与实际输入独立；不监听用户物理键鼠，文本只展示字符数。覆盖层仅断开按钮可交互，不抢焦点、不占布局。

用户断开、提示服务退出或重载，会使对象失效。原生执行线程在 Python 空闲和长操作期间持续检查提示连接，释放自有输入并断开；后续捕获、输入、查询和应用启动均报错，不自动重连。`status` 为 `active`、`closed`、`revoked` 或 `disconnected`；关闭保留撤销或断连原因。

`close()` 先释放输入、销毁虚拟设备，再断开提示并释放锁。host 模式不会退出用户的 Hyprland 或应用。确认完成或等待用户手动接手前立即关闭，不等待整个任务结束。

## 后台桌面

后台由无窗口的 `kwin_wayland --virtual` 承载独立 Hyprland；Hyprland 只连接这个虚拟父 compositor，不申请用户 seat、DRM master 或物理输入。KWin 使用临时 XDG 配置与缓存；Hyprland 和应用复用用户 HOME、开发环境和文件权限。它是桌面焦点分离，不是安全沙箱。

`desktop.launch(argv, cwd=...)` 直接启动程序，不经过 shell，并设置该桌面的连接环境。后台应用由监督进程管理；关闭桌面会终止自有 compositor 和应用。监督进程通过 worker 管道 EOF 处理正常退出和 worker 被硬杀的情况，清理半创建实例、socket 和后代进程。host 启动的应用则独立于控制连接存活。

应用自身的单实例行为仍可能把请求转交给用户已打开的进程。需要专用窗口时使用应用的新实例参数或独立 profile，并通过目标桌面的 `clients` 确认。后台不点亮 host 边框；当前没有把后台桌面临时展示给用户或暂停/恢复的接口。

## 捕获与输入

`desktop.capture()` 同时返回原始缓冲区与图像。`Capture.buffer` 保留裁剪区域的原生像素字和 8-bit/10-bit 位深，只修正方向、裁剪并移除行填充。`_repr_png_()` 按需生成同一帧的 8-bit RGB PNG；`max_size` 只限制图像，不改变 buffer。`display_image` 由 Python worker 提供，SDK 不依赖图像展示包。

`relative_to=shot` 使用图像像素坐标，自动处理裁剪和缩放；截图绑定原桌面对象，不能在另一个对象上用于输入。桌面关闭后，已有截图仍可作为不可变图像或缓冲区读取。

键名采用 US 基础键位对应的 XKB 名称。`hold()` 只释放当前作用域新获取的键；嵌套作用域与重新获取的同名键用独立标识区分。`type_text()` 通过专用 Unicode 键图输入，不改剪贴板，也不绕过应用输入法。物理键盘状态不属于 SDK，不能通过释放 API 清除。

每个桌面的原生线程独占 XKB/Wayland 状态。Python 等待期间释放 GIL 并检查取消；取消会等待桌面清理后传播原异常。先前有键盘输入时，切换到文字键图或执行首个指针操作前等待 20 ms，让客户端和输入法处理已发送的按键；文本逐字符保留投递时间，额外间隔由 `interval` 指定。操作后仍需观察应用结果，协议确认不等于业务完成。

## worker 清理与日志

worker 在 cell 失败或取消时调用 `_close_all()`，关闭所有活动桌面；模块也注册该函数到 atexit。普通 Python 变量保留不意味着旧桌面仍有效。`_release_inputs()` 是只释放虚拟输入、不关闭桌面的内部能力，不代替交还操作。

单次输入失败会回滚本次新获取的键和按钮，已送达应用的动作不可回滚。清理错误写日志；compositor 已断开时无法保证其应用收到松键。worker 被强制终止时 host 依赖 Wayland 断开和 compositor 的虚拟键盘释放行为，不能承诺 Python 清理一定执行。

日志写入 `COMPUTER_USE_LOG`，默认 `/tmp/computer-use-sdk.log`，到 1 MiB 后清空重写。记录操作名、PID、生命周期与成功/失败，不记录截图、坐标、键名、输入文字、窗口内容或 Lua 表达式。Quickshell 只记录连接、事件类型与固定关闭原因；其展示文案不直接使用后端错误文本。

## 构建与维护

```sh
cargo fmt --manifest-path pi/src/extensions/python/native/computer-use/Cargo.toml --check
cargo check --locked --manifest-path pi/src/extensions/python/native/computer-use/Cargo.toml
uv build --wheel --out-dir pi/src/extensions/python/native/computer-use/target/dist pi/src/extensions/python/native/computer-use
qmllint -I /usr/lib/qt6/qml tree/home.linux/.config/quickshell/computer-control/*.qml
```

保留 `Cargo.lock`。maturin 构建 PyO3 0.29.2 的 `abi3-py312` 扩展；后台监督脚本嵌入扩展，不另装 Python 依赖。仓库不新增测试，使用编译、类型检查和真实运行验证；键鼠验证放在专用后台桌面，避免影响用户应用。

模块按职责组织：`src/desktop/` 管对象、独占锁、接管协议和后台进程；`src/input/` 管虚拟设备和按键所有权；`src/capture/` 管像素、编码和坐标；`src/wayland.rs` 管协议事件；`src/hyprland.rs` 管固定目标 IPC；`src/wait.rs` 管取消与等待；`src/logging.rs` 管诊断。
