# 操作模式

先选择桌面，再查结构化状态并截图。下面的输入例子应在目标已确认后执行。

## 后台运行应用

```python
import computer_use as computer

desktop = computer.create_background(size=(1920, 1080))
pid = desktop.launch(["kitty", "--class", "pi-work", "bash"], cwd="/home/wanger/dotfiles")
clients = desktop.hyprland.query("clients")
print(clients)
display_image(desktop.capture(max_size=1600))
```

应用启动返回 PID 不代表窗口已呈现；查询并确认目标窗口后再输入。后台不共享前台焦点，但共享用户文件和权限。应用若默认复用已有进程，应使用其独立实例参数或 profile，避免把新窗口开到前台。完成后 `desktop.close()` 会退出专用桌面及自有应用。

## 连接当前桌面、聚焦并截图

```python
import computer_use as computer

desktop = computer.connect_host()
hyprland = desktop.hyprland
workspaces = hyprland.query("workspaces")
clients = hyprland.query("clients")
# 确认 workspace 3 是目标后再执行。
hyprland.dispatch("hl.dsp.focus({ workspace = 3 })")
active = hyprland.query("activewindow")
shot = desktop.capture(max_size=1600)
display_image(shot)
```

busy 表示已有控制者，不重试抢占。用户点击断开或提示服务断开后，对象失效；停止发送操作，等待用户确认。新一轮操作必须显式连接并重新截图。

## 窗口选择器

`hl.dsp.focus` 与 `hl.dsp.window.*`（close、float、center、resize 等）的 `window` 字段共用同一套选择器；字段可以写选择器字符串，也可以传 `hl.get_window(...)`、`hl.get_active_window()` 返回的窗口对象。

- 精确匹配：`address:0x...`、`stableid:<hex>`、`pid:<数字>`
- 全匹配正则：`class:`、`initialclass:`、`title:`、`initialtitle:`、`tag:`
- 特殊值：`active`（当前焦点窗口）、`floating` / `tiled`（焦点工作区内第一个浮动／平铺窗口）

不带前缀的字符串不会命中任何窗口，`hyprctl dispatch focuswindow <class>` 式的裸 class 写法在这里无效。多个窗口共享 class/title 时先查 `clients` 取 `address`：

```python
clients = desktop.hyprland.query("clients")
target = next(c for c in clients if c["title"] == "nvim")
desktop.hyprland.dispatch(f'hl.dsp.focus({{ window = "address:{target["address"]}" }})')
desktop.hyprland.dispatch('hl.dsp.window.close({ window = "class:mpv" })')
desktop.hyprland.dispatch('hl.dsp.window.float({ window = hl.get_active_window() })')
```

## 裁剪后点击

```python
# rect 始终针对 DP-3 完整截图的原始像素，即使之前看过缩小图。
region = desktop.capture(monitor="DP-3", rect=(600, 300, 1200, 800), max_size=900)
display_image(region)
# 观察图像后，使用返回图像内的像素坐标。
desktop.click(220, 180, relative_to=region)
display_image(desktop.capture(monitor="DP-3", max_size=1600))
```

不要自行除以显示器 scale 或重复加裁剪偏移。`relative_to` 已包含转换信息，且只接受当前桌面对象的截图。显示器布局变化或目标窗口移动后重新观察。

## 读取区域 RGB 与原始字节

```python
region = desktop.capture(rect=(100, 100, 20, 10), max_size=10)
buffer = region.buffer
print(buffer.size, buffer.format, buffer.channel_bits)  # (20, 10)，原生格式与位深
rows = buffer.rgb()
print(rows[0][0])  # (R, G, B)，8-bit 为 0–255，10-bit 为 0–1023
raw = buffer.data  # 未编码像素字节；不是 PNG，也不是每像素三字节的 RGB
print(len(raw), buffer.stride)  # 800, 80

display_image(region)  # 同一帧的 10×5 图像；buffer 仍是 20×10
```

`rows[y][x]` 使用裁剪区域内、缩放前的像素坐标。需要按像素分析后点击时，省略 `max_size`，让 buffer 与图像坐标一致。大区域可用 `np.asarray(rows)` 在 Python 内计算统计结果，不必把所有像素打印给模型。读取 buffer 不会生成或解码 PNG。

## 嵌套组合键

```python
with desktop.hold("ctrl"):
    desktop.press("a")
    with desktop.hold("ctrl", "shift"):
        desktop.press("Left")
    # 内层只释放它新获取的 Shift；Control 仍属于外层。
    desktop.press("c")
```

`hold()` 构造时不按键，只有进入 `with` 才获取。需要跨调用持有时使用 `key_down()`；最后使用 `key_up()` 或 `release_keys()`。

## 输入 Unicode

```python
desktop.click(220, 180, relative_to=region)
desktop.press("ctrl", "a")
desktop.type_text("你好，Hyprland！", interval=0.03)
desktop.press("enter")
```

在局部组合键作用域外调用 `type_text()`，避免被当前对象的修饰键改变语义。它发送 Unicode 键事件，不替换剪贴板，也不绕过应用输入法。

## 滚动与拖拽

```python
shot = desktop.capture(max_size=1600)
display_image(shot)
desktop.move_pointer(700, 450, relative_to=shot)
desktop.scroll(3)
shot = desktop.capture(max_size=1600)
display_image(shot)
desktop.drag((300, 350), (800, 350), duration=0.4, relative_to=shot)
```

`scroll(-2)` 向上两格；`scroll(2, horizontal=True)` 向右两格。拖拽中断会释放按钮；重新截图确认应用已经接受了哪些动作。

## 交还当前桌面

按 [交还、撤销与错误](../SKILL.md#交还撤销与错误) 的要求，在确认操作完成或需要用户接手时关闭：

```python
desktop.close()
print(desktop.status)
```

host 关闭不退出用户 Hyprland 或应用。关闭后告知用户已交还，不再使用这个对象输入；后台模式的关闭则会退出自有桌面和应用。`close()` 不会自动重连，cell 失败或取消也会关闭当前 Python 环境中的所有活动桌面。
