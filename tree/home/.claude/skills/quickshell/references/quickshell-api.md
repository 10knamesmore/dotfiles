# QuickShell API 速查

> 本文档从本机 `/usr/lib/qt6/qml/Quickshell/**/*.qmltypes` 提取，版本 0.2.1。
> 以本机 qmltypes 为权威来源，本文档仅供快速索引。

---

## import Quickshell（核心）

### ShellRoot
根容器，`shell.qml` 的顶层类型。
- `settings: QuickshellSettings* [readonly]`

### QuickshellGlobal（singleton `Quickshell`）
- `processId`, `screens`, `shellDir`, `configDir`, `workingDirectory`, `watchFiles`, `clipboardText`
- `dataDir`, `stateDir`, `cacheDir` — XDG 目录
- `env(QString) -> QVariant` — 读取环境变量
- `execDetached(String[])` — 异步执行命令
- `reload(bool hard)`
- `iconPath(QString) -> QString`
- Signals: `lastWindowClosed`, `reloadCompleted`, `reloadFailed(QString)`, `screensChanged`

### QuickshellScreenInfo（不可创建）
- `name`, `model`, `serialNumber` [const]
- `x`, `y`, `width`, `height` [readonly]
- `devicePixelRatio`, `physicalPixelDensity`, `logicalPixelDensity` [readonly]

### PanelWindow（layer-shell 窗口）
继承 ProxyWindowBase / WlrLayershell
- `layer: WlrLayer::Enum` — Background/Bottom/Top/Overlay
- `anchors` — top/bottom/left/right: bool
- `margins` — top/bottom/left/right: int
- `exclusionMode: ExclusionMode::Enum`
- `exclusiveZone: int`
- `keyboardFocus: WlrKeyboardFocus::Enum` — None/Exclusive/OnDemand
- `focusable: bool`
- `color: QColor` — 窗口背景色（`"transparent"` 常用）
- `visible: bool`
- **没有 `active` 属性**（这是普通 Window 的属性）

### LazyLoader
- `active: bool`, `activeAsync: bool`
- `item: QObject* [readonly]`
- `source: QString` / `component: QQmlComponent*`
- `loading: bool [readonly]`

### Variants
- `model: QVariant` — 数据模型
- `delegate: QQmlComponent*`
- `instances: QObject[] [readonly]`

### SystemClock
- `enabled: bool`
- `precision: SystemClock::Enum` — Hours/Minutes/Seconds
- `date: QDateTime [readonly]`, `hours`, `minutes`, `seconds` [readonly]

### BoundComponent
- `sourceComponent: QQmlComponent*` / `source: QString`
- `bindValues: bool`
- `item: QObject* [readonly]`
- Signal: `loaded`

### ColorQuantizer
- `source: QUrl`, `depth: double`, `rescaleSize: double`
- `colors: QColor[] [readonly]`

### DesktopEntries（singleton）
- `applications: UntypedObjectModel* [readonly]`
- `byId(QString) -> DesktopEntry*`
- `heuristicLookup(QString) -> DesktopEntry*`

### DesktopEntry（不可创建）
- `id`, `name`, `genericName`, `icon`, `execString`, `command: String[]`
- `categories: String[]`, `keywords: String[]`
- `actions: DesktopAction[]`
- `execute()`

---

## import Quickshell.Io

### Process
- `command: String[]` — 命令及参数
- `running: bool` — 设为 true 启动，结束后自动变 false
- `workingDirectory: QString`
- `environment: QHash<QString,QVariant>`
- `clearEnvironment: bool`
- `stdout: DataStreamParser*` — 通常用 SplitParser 或 StdioCollector
- `stderr: DataStreamParser*`
- `stdinEnabled: bool`
- `write(QString)` — 写 stdin
- `startDetached()` — 分离执行
- `signal(int)` — 发信号
- Signals: `started`, `exited(int exitCode, QProcess::ExitStatus)`

### SplitParser（继承 DataStreamParser）
- `splitMarker: QString` — 分隔符（默认换行）
- Signal: `read(QString data)` — 每个分段触发一次

### StdioCollector（继承 DataStreamParser）
- `text: QString [readonly]`
- `data: QByteArray [readonly]`
- `waitForEnd: bool` — 等 EOF 再触发
- Signal: `streamFinished`

### FileView
- `watchChanges: bool` — 文件变动时自动重载
- `atomicWrites: bool`
- `adapter: FileViewAdapter*` — 配合 JsonAdapter 使用
- `loaded: bool [readonly]`
- `reload()`, `writeAdapter()`, `setText(QString)`, `setData(QByteArray)`
- Signals: `loaded`, `loadFailed(FileViewError)`, `saved`, `fileChanged`

### JsonAdapter（配合 FileView）
将 JSON 文件映射为 QML 属性树。

### Socket
- `path: QString` — Unix socket 路径
- `connected: bool [readonly]`
- `write(QString)`, `flush()`
- Signal: `error(QLocalSocket::LocalSocketError)`

### IpcHandler
- `enabled: bool`, `target: QString`

---

## import Quickshell.Hyprland._GlobalShortcuts

### GlobalShortcut
- `appid: QString`, `name: QString`, `description: QString`
- `pressed: bool [readonly]`
- Signals: `pressed`, `released`
- 外部触发：`hyprctl dispatch global <appid>:<name>`

---

## import Quickshell.Hyprland._Ipc

### Hyprland（singleton）
- `focusedMonitor: HyprlandMonitor* [readonly]`
- `focusedWorkspace: HyprlandWorkspace* [readonly]`
- `activeToplevel: HyprlandToplevel* [readonly]`
- `monitors`, `workspaces`, `toplevels: UntypedObjectModel* [readonly]`
- `dispatch(QString)` — 发送 hyprctl dispatch
- `rawEvent(HyprlandIpcEvent*)` signal — 监听所有事件
- `monitorFor(QuickshellScreenInfo*) -> HyprlandMonitor*`
- `refreshMonitors()`, `refreshWorkspaces()`, `refreshToplevels()`

### HyprlandMonitor（不可创建）
- `id`, `name`, `description`, `x`, `y`, `width`, `height`, `scale`
- `activeWorkspace: HyprlandWorkspace*`
- `focused: bool`
- `lastIpcObject: QVariantMap`

### HyprlandWorkspace（不可创建）
- `id`, `name`, `active`, `focused`, `urgent`, `hasFullscreen`
- `monitor: HyprlandMonitor*`
- `toplevels: UntypedObjectModel*`
- `activate()`

### HyprlandToplevel（不可创建）
- `address`, `title`, `activated`, `urgent`
- `workspace: HyprlandWorkspace*`, `monitor: HyprlandMonitor*`
- `wayland: Toplevel*`

### HyprlandIpcEvent（不可创建）
- `name: QString [const]`, `data: QString [const]`
- `parse(int argumentCount) -> QString[]`

---

## import Quickshell.Hyprland._FocusGrab

### HyprlandFocusGrab
- `active: bool`
- `windows: QObjectList`
- Signal: `cleared`

---

## import Quickshell.Wayland._ToplevelManagement

### ToplevelManager（singleton）
- `toplevels: UntypedObjectModel* [readonly]`
- `activeToplevel: Toplevel* [readonly]`

### Toplevel（不可创建）
- `appId`, `title`, `activated`, `screens`, `maximized`, `minimized`, `fullscreen`
- `activate()`, `close()`, `fullscreenOn(QuickshellScreenInfo*)`

---

## import Quickshell.Services.UPower

### UPower（singleton）
- `displayDevice: UPowerDevice* [readonly]`
- `devices: UntypedObjectModel* [readonly]`
- `onBattery: bool [readonly]`

### UPowerDevice（不可创建）
- `percentage: double [readonly]`
- `state: UPowerDeviceState::Enum` — Charging/Discharging/FullyCharged/...
- `timeToEmpty`, `timeToFull: double [readonly]`
- `isLaptopBattery: bool [readonly]`
- `healthPercentage: double [readonly]`
- `iconName: QString [readonly]`

### PowerProfiles（singleton）
- `profile: PowerProfile::Enum` — PowerSaver/Balanced/Performance

---

## import Quickshell.Services.Pipewire

### Pipewire（singleton）
- `defaultAudioSink: PwNode* [readonly]`
- `defaultAudioSource: PwNode* [readonly]`
- `nodes`, `links`, `linkGroups: UntypedObjectModel* [readonly]`
- `ready: bool [readonly]`

### PwNode（不可创建）
- `name`, `description`, `nickname` [const]
- `isSink`, `isStream: bool [const]`
- `type: PwNodeType::Flags`
- `audio: PwNodeAudio* [readonly]`

### PwNodeAudio（不可创建）
- `muted: bool`
- `volume: float` — 所有声道平均值 [0.0-1.0+]
- `volumes: float[]` — 逐声道音量
- `channels: PwAudioChannel::Enum[]`

---

## import Quickshell.Services.Mpris

### Mpris（singleton）
- `players: UntypedObjectModel* [readonly]`

### MprisPlayer（不可创建）
- `identity: QString [readonly]` — 播放器名
- `trackTitle`, `trackArtist`, `trackAlbum`, `trackArtUrl` [readonly]
- `playbackState: MprisPlaybackState::Enum` — Stopped/Playing/Paused
- `isPlaying: bool`
- `position: double`, `length: double`
- `volume: double` [0.0-1.0+]
- `shuffle: bool`, `loopState: MprisLoopState::Enum`
- `play()`, `pause()`, `togglePlaying()`, `next()`, `previous()`, `seek(double)`

---

## import Quickshell.Services.Notifications

### NotificationServer
- `keepOnReload: bool`
- `trackedNotifications: UntypedObjectModel* [readonly]`
- `bodyMarkupSupported`, `actionsSupported`, `imageSupported`, `inlineReplySupported: bool`
- Signal: `notification(Notification*)`

### Notification（不可创建）
- `id: uint [const]`
- `appName`, `appIcon`, `summary`, `body` [readonly]
- `urgency: NotificationUrgency::Enum` — Low/Normal/Critical
- `actions: NotificationAction*[]`
- `image: QString [readonly]`
- `transient`, `resident: bool`
- `expire()`, `dismiss()`, `sendInlineReply(QString)`
- Signal: `closed(NotificationCloseReason::Enum)`

---

## import Quickshell.Services.SystemTray

### SystemTray（singleton）
- `items: UntypedObjectModel* [readonly]`

### SystemTrayItem（不可创建）
- `id`, `title`, `icon`, `tooltipTitle`, `tooltipDescription` [readonly]
- `status: Status::Enum` — Passive/Active/NeedsAttention
- `hasMenu: bool`, `menu: DBusMenuHandle*`
- `activate()`, `secondaryActivate()`
- `display(QObject* parentWindow, int x, int y)`

---

## import Quickshell.Widgets

### WrapperItem / WrapperRectangle / WrapperMouseArea
将子 Item 包裹在容器中，继承子 Item 的 implicitSize。

### ClippingRectangle
带圆角裁剪的 Rectangle（解决 Qt 圆角不裁剪子元素的问题）。

### IconImage
- 从主题或路径加载图标，支持 `source: QString`（图标名或路径）
