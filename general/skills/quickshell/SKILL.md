---
name: quickshell
description: 协助编写和调试 QuickShell (QML) shell 配置，包括 layer shell 面板、Hyprland 集成、系统服务绑定等。
---

# QuickShell 配置助手

## 目标

协助用户编写和调试 QuickShell 配置，包括：PanelWindow 布局、Hyprland IPC 集成、系统服务（音量/电量/通知/MPRIS）、进程交互、组件拆分等。

## 核心原则

**先查 qmltypes，再写代码。** 不要凭记忆描述 API，直接读本机安装的类型定义。所有模块的 qmltypes 在 `/usr/lib/qt6/qml/Quickshell/`。

## 环境信息

- QuickShell 版本：`quickshell --version`
- 用户 shell 配置：`~/.config/quickshell/`（通常为 `~/dotfiles/linux/.config/quickshell/` 的符号链接）
- 入口文件：`shell.qml`（固定，不可更改）
- 日志：`/run/user/1000/quickshell/by-id/*/log.qslog`

## 调查流程

### 1) 明确问题

- 报错信息在哪一行？（错误格式：`@file.qml[line:col]: ...`）
- 是加载失败、运行时错误、还是行为不符预期？

### 2) 读 qmltypes 确认 API

遇到任何不确定的属性或类型，先读对应模块的 qmltypes：

```
/usr/lib/qt6/qml/Quickshell/quickshell-core.qmltypes        # 核心：ShellRoot, PanelWindow, Process...
/usr/lib/qt6/qml/Quickshell/Io/quickshell-io.qmltypes        # IO: Process, Socket, FileView, SplitParser...
/usr/lib/qt6/qml/Quickshell/Wayland/quickshell-wayland.qmltypes
/usr/lib/qt6/qml/Quickshell/Hyprland/quickshell-hyprland.qmltypes
/usr/lib/qt6/qml/Quickshell/Hyprland/_GlobalShortcuts/quickshell-hyprland-global-shortcuts.qmltypes
/usr/lib/qt6/qml/Quickshell/Hyprland/_Ipc/quickshell-hyprland-ipc.qmltypes
/usr/lib/qt6/qml/Quickshell/Services/Mpris/quickshell-service-mpris.qmltypes
/usr/lib/qt6/qml/Quickshell/Services/Notifications/quickshell-service-notifications.qmltypes
/usr/lib/qt6/qml/Quickshell/Services/Pipewire/quickshell-service-pipewire.qmltypes
/usr/lib/qt6/qml/Quickshell/Services/UPower/quickshell-service-upower.qmltypes
/usr/lib/qt6/qml/Quickshell/Services/SystemTray/quickshell-service-statusnotifier.qmltypes
/usr/lib/qt6/qml/Quickshell/Widgets/quickshell-widgets.qmltypes
```

也可查 `references/quickshell-api.md` 中的速查表（注意：以本机 qmltypes 为准）。

### 3) 读用户现有配置

先读 `~/.config/quickshell/` 下的所有 `.qml` 文件，理解现有结构再提建议。

### 4) 提修复方案

优先最小改动。涉及 QML 语言特性（binding loop、属性覆盖、信号连接）时，说明原因。

## 已知陷阱（必须记住）

### PanelWindow

- **没有 `active` 属性**，`onActiveChanged` 会导致加载失败。`active` 是普通 `Window` 的属性，PanelWindow 不继承它。
- **点击外部关闭**的正确做法：将 PanelWindow 铺满全屏（四边 anchors 全 true），背景透明，外层 `MouseArea` 捕获点击关闭，面板内部再加一个 `MouseArea` 阻止事件穿透。
- **`color: "transparent"`** 需要显式设置，否则默认白色背景会覆盖整个屏幕。
- `implicitWidth/implicitHeight` 只在窗口不铺满时控制大小；铺满模式下面板位置用 `anchors` 定位。

### 路径与环境变量

- **`StandardPaths` 不可用**，用 `Quickshell.env("HOME")` 替代。
- 其他有用的路径：`Quickshell.cacheDir`、`Quickshell.dataDir`、`Quickshell.configDir`（均为 XDG 目录）。

### Process

- `running = true` 触发执行；进程结束后 `running` 自动变 `false`。
- 想重复执行同一命令：先确保 `running = false`，再设 `running = true`（或直接再次设为 true，QuickShell 会重启进程）。
- stdout 用 `SplitParser`（按行分割）或 `StdioCollector`（收集全部输出）。
- `SplitParser` 的 `onRead` 每次只触发一行，不含换行符。

### 自定义 Slider（QtQuick.Controls）

- handle 的 `Rectangle` **必须设置 `implicitWidth` 和 `implicitHeight`**，否则拖拽命中区域为零，只能点击不能拖动。
- background 的 `Rectangle` 同样需要 `implicitWidth` 和 `implicitHeight`，否则 Slider 宽度计算异常。

### 子目录组件

- 同目录的 `.qml` 文件自动作为组件类型可用。
- 子目录需要在使用方显式 `import "./subdir"`，子目录内部组件互相引用不需要 import。

### GlobalShortcut（Hyprland）

- import 路径：`import Quickshell.Hyprland._GlobalShortcuts`（带下划线）
- 外部工具触发：`hyprctl dispatch global quickshell:<name>`
- `appid` + `name` 共同构成唯一标识

### Hyprland IPC

- `import Quickshell.Hyprland._Ipc` 获得 `Hyprland` singleton
- 发送命令：`Hyprland.dispatch("keyword ...")`
- 监听原始事件：`Hyprland.rawEvent` 信号，事件对象有 `name` 和 `data` 属性

### FileView + JsonAdapter

- 用于读写 JSON 状态文件，比 `Process { command: ["cat", ...] }` 更干净
- `watchChanges: true` 可在文件被外部修改时自动重新加载

## 常用模式

### 执行脚本并读取输出

```qml
Process {
    id: proc
    stdout: SplitParser {
        onRead: data => { /* 每行一次 */ }
    }
}
// 触发：
proc.command = ["bash", "-c", "some-command"]
proc.running = true
```

### 可关闭的浮动面板

```qml
PanelWindow {
    anchors.top: true; anchors.bottom: true
    anchors.left: true; anchors.right: true
    color: "transparent"
    focusable: false
    exclusionMode: ExclusionMode.Ignore

    MouseArea {
        anchors.fill: parent
        onClicked: parent.visible = false
    }

    Rectangle {
        id: panel
        anchors.top: parent.top; anchors.topMargin: 54
        anchors.right: parent.right; anchors.rightMargin: 10
        width: 320; height: contentCol.implicitHeight + 32

        MouseArea { anchors.fill: parent; onClicked: mouse => mouse.accepted = true }
        // ... 面板内容
    }
}
```

### GlobalShortcut 触发面板

```qml
// shell.qml
import Quickshell.Hyprland._GlobalShortcuts
import "./my-panel"

ShellRoot {
    GlobalShortcut {
        appid: "quickshell"
        name: "myPanel"
        onPressed: panel.togglePanel()
    }
    MyPanel { id: panel }
}
```

### 读取系统服务

```qml
// 电量
import Quickshell.Services.UPower
Text { text: Math.round(UPower.displayDevice.percentage) + "%" }

// 默认音频音量
import Quickshell.Services.Pipewire
Text { text: Math.round(Pipewire.defaultAudioSink.audio.volume * 100) + "%" }

// 当前播放
import Quickshell.Services.Mpris
Repeater {
    model: Mpris.players
    Text { text: modelData.trackTitle + " — " + modelData.trackArtist }
}
```
