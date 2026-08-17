# QML 文档注释规范（桌面 shell 实践版）

## 基本形式

* QML 生态没有强制的文档标准，规范务实优先，不追求工具链可生成。
* 日常项目**推荐普通注释**：单行用 `//`，多行用 `/* ... */`。
* qdoc 风格 `/*! ... */`（配合 `\qmltype`、`\qmlproperty`、`\brief` 等命令）**仅在确实要生成文档站点时使用**；自用 shell 配置不要引入。
* 文档注释紧贴被注释对象上方；简短的尾随说明可写在同一行右侧。
* QML 函数本质是 JavaScript，函数注释采用 JS 风格的 `@param` / `@returns`。

## 放置位置

* 组件文件级注释放在最外层根元素声明上方（`import` 之后、根类型之前）。
* `property`、`signal`、`function` 的注释放在各自声明上方；取值简短的可用尾随 `//` 注释。
* 复杂绑定表达式在绑定上方说明意图，或拆成具名 `function` / `readonly property` 再注释。
* 自明的 `width` / `height` / `anchors` / `color` 直连常量等不写注释。

## 内容建议

* 第一行直接说明“这个组件/属性/信号/函数做什么”，不要复述名字。
* 优先写调用者关心的：语义、单位、取值范围、默认值、发射时机、副作用、降级行为。
* 不要解释实现里无关的循环、临时变量、私有缓存结构。
* 示例与真实 API 一致，贴近 shell 实际用途（面板、服务单例、进程订阅）。

## 组件文件级注释

每个自定义组件文件顶部写一段说明组件职责。一句话概括 + 必要时补关键行为、约束。

```qml
import "../../theme"
import QtQuick
import QtQuick.Layouts

// 基础模块包装：圆角背景 + 左侧弧形指示器(hover 蔓延) + 弹性缩放 + 柔和阴影。
// bar 下所有 module 都套这一层，统一交互与视觉；contents 默认属性接收子内容。
Rectangle {
    id: root
    // ...
}
```

职责复杂、有历史包袱时，可多写两行交代为什么这样设计：

```qml
// 系统监控采集 — 每秒一次 fork，读 /proc/stat + /proc/meminfo + /proc/net/dev，
// 算好 CPU / 内存 / 网速写入 SystemStats 单例。
// 取代旧 Cpu/Memory/NetSpeed 三个 module 各自 1Hz 轮询（4 fork → 1 fork）。
Scope {
    id: root
}
```

## property 注释

注释关注**语义、单位、取值范围、默认值**。多数 property 一行尾随注释即可：

```qml
property real progress: -1          // 进度值 0~1，-1 表示不启用进度条
property real backgroundAlpha: 0.85 // 背景不透明度 0~1
property color accentColor: Colors.blue
```

值的含义不直观、或有约定时，单独起一行写在上方：

```qml
// 当前选中的接口名；空字符串表示尚未探测到物理网卡。
property string netIface: ""

// per-core 使用率数组，长度等于逻辑核心数；采集前为空数组。
property var cpuCorePcts: []
```

### required / readonly / alias

这三类要写清**契约**，因为它们对调用者有强约束。

`required`：必须由使用方传入，注释说明期望什么。

```qml
// 必填：本模块对应的屏幕，用于 layer shell 锚定与多屏分发。
required property var modelData
```

`readonly`：对外只读的派生/常量，注释说明它从哪来、何时变。

```qml
// 历史缓冲容量（环形），系统面板曲线按此长度滚动；编译期常量。
readonly property int histMax: 60
```

`alias`：暴露内部子项，注释说明它实际指向谁、调用者能改什么。

```qml
// 默认内容插槽：宿主把子元素塞进内部 inner 容器。
default property alias contents: inner.data

// 暴露内层文本，供外部直接改 text / color。
property alias label: labelItem
```

## signal 注释

写清**何时发射**与**每个参数的含义**。

```qml
// 左键点击模块时发射；mouse 为 MouseArea 的事件对象（含 x/y/button）。
signal clicked(var mouse)

// 滚轮滚动时发射；delta 为方向，+1 上滚 / -1 下滚（已归一，不带原始角度）。
signal scrolled(int delta)

// 拖动进度区时发射；value 为归一化进度 0~1（已 clamp）。
signal progressDragged(real value)
```

无参数信号若名字够直白可只写一行时机：

```qml
// 面板完成展开动画后发射。
signal opened()
```

## function 注释（JS 风格）

QML 函数是 JS，用 `@param` / `@returns`。第一行摘要，参数/返回不直观时补标签。

```qml
/**
 * 把字节速率格式化为带单位的短字符串。
 * @param {number} bytesPerSec 速率，单位 bytes/s
 * @returns {string} 形如 "1.2 MB/s" 的展示文本
 */
function formatSpeed(bytesPerSec) {
    // ...
}
```

简短私有辅助函数可只写一行摘要，省略标签：

```qml
// 把原始 /proc 文本拆成三段分别解析。
function _parse(raw) {
    // ...
}
```

涉及副作用的函数要点明（写哪个单例、起什么进程）：

```qml
/**
 * 触发一次采集：重启 reader 进程，结果异步写入 SystemStats。
 * @remarks 有副作用——会 fork 子进程并改写 SystemStats 多个属性。
 */
function refresh() {
    reader.running = false;
    reader.running = true;
}
```

## 复杂绑定表达式注释

长绑定、含魔法逻辑的绑定，在上方说明意图。能拆就拆成具名 function / readonly property 再注释。

```qml
// hover 时背景提亮一档；flat 模式走半透明分支。可读性优先，别再加分支。
color: root.flat
    ? Qt.rgba(Colors.surface1.r, Colors.surface1.g, Colors.surface1.b, root.hovered ? 0.85 : 0.5)
    : (hovered ? Qt.lighter(root.backgroundColor, 1.1) : root.backgroundColor)
```

绑定里有不显然的“为什么”，务必记下来，否则后人会误改：

```qml
// 必须整体新数组赋值才触发绑定刷新（原地 push 不会通知）。
SystemStats.cpuHist = SystemStats.cpuHist.concat([SystemStats.cpuUsage]).slice(-histMax);
```

## pragma Singleton 服务单例

`pragma Singleton` 的服务/状态单例是全局契约，文件级注释**必须**写明：

1. 这个单例负责什么；
2. 对外暴露哪些属性 / 信号（消费方读什么）；
3. 副作用——起进程、订阅 DBus、定时轮询、写文件等。

```qml
pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io

/*
 * 音频服务单例。
 *
 * 对外契约：
 *   - property volume   当前主输出音量 0~1
 *   - property muted    是否静音
 *   - signal volumeChangedByUser  用户经本服务改音量后发射（用于 OSD）
 *
 * 副作用：
 *   - 持有 PipeWire 节点订阅，进程存活期间常驻；
 *   - setVolume() 会 spawn wpctl 子进程。
 */
Singleton {
    id: root

    property real volume: 0
    property bool muted: false
    signal volumeChangedByUser()
}
```

纯数据单例（无副作用）也要标明“谁写、谁读”：

```qml
pragma Singleton
import QtQuick

// 系统监控数据 — 由 SystemStatsService 每秒写入；bar 的 Cpu/Memory/NetSpeed 模块纯读取。
// 本单例自身无副作用，只是共享状态容器。
QtObject {
    property int cpuUsage: 0             // 总体 CPU 占用百分比 0~100
    property int memUsagePct: 0          // 内存占用百分比 0~100
    property real netUpSpeed: 0          // 上行速率 bytes/s
    property real netDownSpeed: 0        // 下行速率 bytes/s
}
```

## 分层注释约定

仓库按 theme / state / services 三层划分，各层注释侧重点不同。

### theme（样式常量）

样式常量层（`Colors`、`Tokens`、`Fonts`、`Anim` 等）。重点是**含义、单位、设计意图**，不是数值本身。

```qml
pragma Singleton
import QtQuick

// 设计 token：圆角、间距、动画时长、透明度的全局常量，所有组件统一引用。
QtObject {
    readonly property real radiusL: 14        // 大圆角，模块/面板用
    readonly property int animFast: 120       // 快速过渡时长，单位 ms（hover 反馈）
    readonly property int animElaborate: 420  // 强调动画时长，单位 ms（展开/状态切换）
    readonly property real panelAlpha: 0.85   // 面板默认背景不透明度 0~1
}
```

对显然的色值/数值不必逐个解释，给分组小标题即可：

```qml
// ── 强调色 ──
readonly property color blue: "#89b4fa"
readonly property color red: "#f38ba8"
```

### state（状态单例）

共享可变状态。重点是**谁写、谁读、取值范围**，以及刷新约定（如必须整体赋值才触发绑定）。参见上面 `SystemStats` 示例。

```qml
pragma Singleton
import QtQuick

// 面板开合状态 — 控制中心各 panel 共享；任意组件可写 activePanel 切换当前面板。
QtObject {
    // 当前激活面板的标识；空字符串表示全部关闭。
    property string activePanel: ""
    // 是否正在播放展开/收起动画，期间屏蔽重复触发。
    property bool animating: false
}
```

### services（服务）

承载副作用的执行层（进程、DBus、Socket、定时器）。重点是**副作用清单 + 对外契约**，写法见上面 pragma Singleton 一节。服务层注释最该详细，因为副作用最难从代码一眼看清。

## 反例（不要这样写）

自明属性加噪声注释，纯属浪费：

```qml
// 反例：这些都是自明的，不要写
Rectangle {
    width: 100              // 宽度 100   ← 删
    height: 36              // 高度 36     ← 删
    anchors.fill: parent    // 填满父级   ← 删
    color: "red"            // 红色        ← 删
    visible: true           // 可见        ← 删
}
```

注释复述名字、不增信息：

```qml
// 反例：等于没说
property int cpuUsage: 0    // CPU usage（复述名字，没讲单位/范围）

// 正解
property int cpuUsage: 0    // 总体 CPU 占用百分比 0~100
```

给 Behavior / 动画块逐个写“这是动画”：

```qml
// 反例
Behavior on opacity {
    // 透明度动画   ← 删，类型已说明一切
    NumberAnimation { duration: Tokens.animFast }
}
```

## 风格与边界

* 中文为主，技术术语（property / signal / binding / Singleton / DBus / layer shell 等）保留英文。
* 摘要回答“它做什么”，不是“它是什么”。
* 同一仓库统一风格：是否用 `//` vs `/* */`、是否给信号都写参数、分层小标题是否用 `── 标题 ──`。
* 公共组件、对外契约、有副作用的服务**必须**注释；纯内部细节按需。
* property 优先尾随单行注释（语义+单位+范围+默认值）；含义复杂再起独立行。
* 复杂绑定记“为什么”，尤其是反直觉的赋值约定（如整体新数组才触发绑定）。
* 不引入 qdoc 命令，除非项目明确要生成文档站点。
