# 操作模式

先查结构化状态确认目标，再观察屏幕。下面的输入例子应在目标已确认后执行。

## 聚焦工作区并截图

```python
from computer_use import hyprland
import computer_use as computer

workspaces = hyprland.query("workspaces")
clients = hyprland.query("clients")
# 确认 workspace 3 是目标后再执行。
hyprland.dispatch("hl.dsp.focus({ workspace = 3 })")
active = hyprland.query("activewindow")
shot = computer.capture(max_size=1600)
display_image(shot)
```

## 窗口选择器

`hl.dsp.focus` 与 `hl.dsp.window.*`（close、float、center、resize 等）的 `window` 字段共用同一套选择器；字段可以写选择器字符串，也可以传 `hl.get_window(...)`、`hl.get_active_window()` 返回的窗口对象。

- 精确匹配：`address:0x...`、`stableid:<hex>`、`pid:<数字>`
- 全匹配正则：`class:`、`initialclass:`、`title:`、`initialtitle:`、`tag:`
- 特殊值：`active`（当前焦点窗口）、`floating` / `tiled`（焦点工作区内第一个浮动／平铺窗口）

不带前缀的字符串不会命中任何窗口，`hyprctl dispatch focuswindow <class>` 式的裸 class 写法在这里无效。多个窗口共享 class/title 时先查 `clients` 取 `address`：

```python
clients = hyprland.query("clients")
target = next(c for c in clients if c["title"] == "nvim")
hyprland.dispatch(f'hl.dsp.focus({{ window = "address:{target["address"]}" }})')
hyprland.dispatch('hl.dsp.window.close({ window = "class:mpv" })')
hyprland.dispatch('hl.dsp.window.float({ window = hl.get_active_window() })')
```

## 裁剪后点击

```python
# rect 始终针对 DP-3 完整截图的原始像素，即使之前看过缩小图。
region = computer.capture("DP-3", rect=(600, 300, 1200, 800), max_size=900)
display_image(region)
# 观察图像后，使用返回图像内的像素坐标。
computer.click(220, 180, relative_to=region)
display_image(computer.capture("DP-3", max_size=1600))
```

不要自行除以显示器 scale 或重复加裁剪偏移。`relative_to` 已包含转换信息。显示器布局变化或目标窗口移动后重新观察。

## 读取区域 RGB 与原始字节

```python
region = computer.capture(rect=(100, 100, 20, 10), max_size=10)
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
with computer.hold("ctrl"):
    computer.press("a")
    with computer.hold("ctrl", "shift"):
        computer.press("Left")
    # 内层只释放它新获取的 Shift；Control 仍属于外层。
    computer.press("c")
```

`hold()` 构造时不按键，只有进入 `with` 才获取。需要跨调用持有时使用 `key_down()`；最后使用 `key_up()` 或 `release_keys()`。

## 输入 Unicode

```python
computer.click(220, 180, relative_to=region)
computer.press("ctrl", "a")
computer.type_text("你好，Hyprland！", interval=0.03)
computer.press("enter")
```

在局部组合键作用域外调用 `type_text()`，避免被本 worker 的修饰键改变语义。它发送 Unicode 键事件，不替换剪贴板，也不绕过应用输入法。

## 滚动与拖拽

```python
shot = computer.capture(max_size=1600)
display_image(shot)
computer.move_pointer(700, 450, relative_to=shot)
computer.scroll(3)
shot = computer.capture(max_size=1600)
display_image(shot)
computer.drag((300, 350), (800, 350), duration=0.4, relative_to=shot)
```

`scroll(-2)` 向上两格；`scroll(2, horizontal=True)` 向右两格。拖拽中断会释放按钮；重新截图确认应用已经接受了哪些动作。

## 交还当前桌面

按 [交还当前桌面](../SKILL.md#交还当前桌面) 的要求，在确认操作完成或需要用户接手时关闭：

```python
computer.close()
```

未创建输入连接时 `close()` 不会连接桌面。关闭后告知用户桌面已交还；它不会退出 Hyprland 或关闭用户应用。当前 SDK 在后续输入调用时会新建连接，因此交还后不要继续发送输入，除非开始新一轮桌面操作。worker 的 cell 错误和取消只触发虚拟输入清理，不代表整个桌面任务已经结束。
