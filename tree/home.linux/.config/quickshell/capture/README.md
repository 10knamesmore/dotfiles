# 屏幕捕获（Hyprland）

Quickshell 内完成截图、原位标注、取色和录屏来源选择，不启动外部图片编辑器。

## 依赖与接入

- Quickshell、Qt 6.11+、Python 3.11+、Pillow、`grim`、`wl-clipboard`、`xdg-user-dirs`。
- 录屏另需 `gpu-screen-recorder`、`xdg-desktop-portal`、`xdg-desktop-portal-hyprland`、PipeWire，以及用于识别 Portal 调用者的 `busctl`。音频通过 PulseAudio 接口读取默认输出与默认输入，使用 PipeWire 时需 `pipewire-pulse`。
- `dots sync` 分发配置与 `dots-capture-picker`，不安装这些系统依赖。安装依赖后重新加载 Quickshell。
- `~/.config/hypr/xdph.conf` 指定自定义 picker；Portal 服务的 `PATH` 须包含 `$DOTS_SCRIPTS`。更新 Portal 后重启对应用户服务；重启会中断现有屏幕共享。

窗口录制要求 Portal 正确处理窗口尺寸变化。已验证 GSR 6.1.3 与 [XDPH e87ae78](https://github.com/hyprwm/xdg-desktop-portal-hyprland/commit/e87ae7823e7bf0601385220c69b5a3b245123fc5)；XDPH 发行包 `1.4.1-2` 在窗口缩放时会花屏，需要更新，不能用矩形录制代替真窗口录制。

## 使用

| 快捷键 | 操作 |
| --- | --- |
| Super+P | 区域截图 |
| Super+Shift+P | 窗口截图 |
| Super+Alt+P | 取色 |
| Super+Ctrl+P | 录屏设置与来源选择 |
| Super+Ctrl+Shift+P | 停止录屏 |

也可点击状态栏捕获图标。录制时图标变成计时按钮，点击整个按钮即可停止；收到首帧后才开始计时。

截图浮层支持区域、窗口与显示器选择，以及箭头、矩形、画笔、文字、马赛克和实色遮挡。选区内可移动，边缘可调整大小；选区不跨显示器。每个显示器保留原始像素，不把分数缩放后的桌面逻辑尺寸当作导出尺寸。

- Enter 或“复制”：只复制 PNG，不保存永久图片。
- Ctrl+S 或“保存”：只保存到 XDG 图片目录下的 `Screenshots/`。
- Esc：取消，不覆盖剪贴板；文字输入中先取消当前文字。
- Ctrl+Z：撤销；Ctrl+Shift+Z 或 Ctrl+Y：重做。
- 文字输入中 Enter 确认，Shift+Enter 换行。
- 取色可选择 HEX 或 RGB，点击原图像素复制；标注工具的取色只改变画笔颜色。

马赛克用于视觉模糊；隐藏敏感内容时使用实色遮挡。窗口**截图**裁剪冻结桌面，不能恢复被遮挡部分；窗口**录屏**通过 Portal/PipeWire 捕获独立窗口，不把其他窗口录进来。

录屏可同时勾选系统声音与麦克风，混合到 AAC 音轨；视频为 H.264、60 FPS、MP4，结束后保存到 XDG 视频目录下的 `Recordings/`。未收到首帧的取消不保留文件。关闭被录窗口或重载 Quickshell 会停止并保存当前录制，不跨重载继续录制。

## 生命周期与屏幕共享

原图与标注中间文件位于当前 Hyprland 会话的私有运行时目录，完成、取消或后端退出时清理。复制后的剪贴板由 `wl-copy` 持有，不依赖浮层继续存在。

自定义 picker 同样接管浏览器等应用的 Portal 屏幕共享请求，因此使用这些请求时 Quickshell 必须运行。每次请求都需要明确选择，不使用记忆授权或预先批准的来源。正在编辑截图时，新共享请求会被拒绝，完成当前操作后重试。

后端只停止自己启动的录屏进程，不按名称结束其他录像。Quickshell 的 Process 会在重载时强制结束它跟踪的进程，因此它跟踪 `backend/launcher.py`；真正的后端在输入管道关闭后完成录制保存与临时文件清理。

诊断写入 Quickshell 日志，前缀为 `[capture]` 和 `[capture-backend]`。界面只展示结构化状态对应的提示，不直接显示后端内部错误。
