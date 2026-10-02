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
shot = computer.screenshot(max_size=1600)
display_image(shot)
```

## 裁剪后点击

```python
# rect 始终针对 DP-3 完整截图的原始像素，即使之前看过缩小图。
region = computer.screenshot("DP-3", rect=(600, 300, 1200, 800), max_size=900)
display_image(region)
# 观察图像后，使用返回图像内的像素坐标。
computer.click(220, 180, relative_to=region)
display_image(computer.screenshot("DP-3", max_size=1600))
```

不要自行除以显示器 scale 或重复加裁剪偏移。`relative_to` 已包含转换信息。显示器布局变化或目标窗口移动后重新观察。

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
shot = computer.screenshot(max_size=1600)
display_image(shot)
computer.move_pointer(700, 450, relative_to=shot)
computer.scroll(3)
shot = computer.screenshot(max_size=1600)
display_image(shot)
computer.drag((300, 350), (800, 350), duration=0.4, relative_to=shot)
```

`scroll(-2)` 向上两格；`scroll(2, horizontal=True)` 向右两格。拖拽中断会释放按钮；重新截图确认应用已经接受了哪些动作。

## 主动结束

```python
owned_keys = computer.held_keys()
computer.release_keys()
computer.close()
```

未创建输入连接时这些调用不会连接桌面。`close()` 后可以继续使用 SDK，下一次输入会新建连接。worker 的 cell 错误和取消会触发其内部清理，不需要在每个 cell 中复制 `try/finally`。
