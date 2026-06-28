# 显示器管理面板重做 · 设计稿

日期：2026-06-28
范围：`tree/home.linux/.config/quickshell/`（QuickShell 配置）+ `tree/home.linux/.config/hypr/`

## 背景与目标

现有显示器管理（`quickshell/display/` 下的 `DisplayPanel.qml` + `MonitorCard.qml` + `MonitorPreview.qml`）功能能用，但有几处硬伤：

1. 业务逻辑全堆在 `DisplayPanel.qml`，没收口成 service 单例，违反仓库的「theme/state/services 三层」约定。
2. 用 10 秒 `Timer` 轮询热插拔，违反「CPU 轮询反模式」（应事件驱动）。
3. 配置不持久化——`hyprctl keyword monitor` 重启即失效。
4. 交互简陋：不能像现代 OS 那样拖拽摆位；无主显示器、无回滚保护。

目标：重做成一个**现代 OS 桌面风格的可视化显示器管理面板**，直接绑定 Hyprland，遵循仓库分层约定，事件驱动，可持久化，并**按显示器组合自动记忆/恢复布局**。

非目标（YAGNI）：VRR / 10bit / HDR 等高级色彩项本期不做；镜像模式本期不做。

## 已决定的关键选型

| 维度 | 决定 |
| --- | --- |
| 整体布局 | B 方案：左侧可拖拽画布 + 右侧选中屏参数控件（类 macOS） |
| 持久化落点 | 机器本地 `~/.local/state/hypr/monitors.local.lua`（开机恢复）+ `~/.local/state/quickshell/monitor-profiles.json`（按组合存档）。**均落 XDG state，不入 git**（`~/.config/hypr` 是整目录软链进仓库，故开机文件不能落那里） |
| 热插拔 | 按「显示器组合签名」记忆布局、插上自动恢复 |
| apply 安全 | apply 后 15 秒回滚确认（防错误配置黑屏） |
| 未知新组合 | 不自动应用，保持 Hyprland 默认 + 发通知「新显示器组合，点此配置」 |
| 入口 | 侧边栏（`QuickSettings.qml`）内一个「显示器」按钮打开；**不加快捷键** |
| 自动恢复宿主 | 全收口进常驻的 QuickShell `MonitorService` 单例（方案 A），不引入额外进程 |

## 架构

QuickShell 的 `shell.qml` 进程常驻（即 bar/shell 守护进程），因此 `MonitorService` 单例可一直挂着监听 Hyprland 事件，即使面板 UI 未打开也能自动恢复布局。UI 面板只是该 service 的消费者。

```
┌─ services/MonitorService.qml (常驻单例·大脑) ─────────────┐
│  · 订阅 Hyprland 显示器事件（socket2）→ 事件驱动，无轮询    │
│  · 查询当前显示器                                           │
│  · 计算组合签名                                             │
│  · apply（hyprctl keyword monitor）+ 15s 回滚定时器        │
│  · 读写 monitor-profiles.json / monitors.local.lua         │
│  · 开机对账、热插拔自动应用                                  │
└───────────┬───────────────────────────────┬───────────────┘
            │ 写状态                          │ 调用纯逻辑
            ▼                                ▼
   state/MonitorState.qml          display/lib/monitorModel.js
   (响应式状态单例)                  (纯函数·可 node 测试)
            │
            ▼ 绑定
   display/DisplayPanel.qml (B 布局容器)
     ├─ MonitorCanvas.qml   (左·拖拽吸附画布)
     ├─ MonitorControls.qml (右·参数控件)
     └─ RevertConfirm.qml   (回滚倒计时条)
```

## 文件结构

新增/重写（`tree/home.linux/.config/quickshell/`）：

```
services/MonitorService.qml   新·常驻单例：事件订阅 + 查询 + 签名 + apply + 存储 + 回滚 + 开机对账
state/MonitorState.qml        新·响应式状态：monitors[]、当前签名、applying、pending 布局、回滚倒计时
display/lib/monitorModel.js   新·纯函数：签名计算 / 布局↔monitor 串序列化 / 存储读写与迁移
display/DisplayPanel.qml      重写·B 布局容器
display/MonitorCanvas.qml     新·可拖拽吸附画布（替代旧 MonitorPreview）
display/MonitorControls.qml   新·右侧参数控件（替代旧 MonitorCard）
display/RevertConfirm.qml     新·回滚倒计时条
```

删除：旧 `display/MonitorPreview.qml`、`display/MonitorCard.qml`，旧 `DisplayPanel.qml` 内的业务逻辑。

修改：
- `settings/QuickSettings.qml`：替换 458–484 行的三个 monitor_profile 按钮为单个「显示器」入口。
- `tree/home.linux/.config/hypr/hyprland.lua`：显示器段改为 `dofile(monitors.local.lua)` + 内置默认；删除对 `monitor_profile.sh` 的引用（约 371 行）。
- 删除脚本 `scripts/linux/hypr/monitor_profile.sh`。

## 数据获取（事件驱动）

已验证 `Quickshell.Hyprland` 的 `Hyprland` 单例 API（`/usr/lib/qt6/qml/Quickshell/Hyprland/_Ipc/quickshell-hyprland-ipc.qmltypes`）：

- `Hyprland` 单例：`monitors`、`focusedMonitor`、`rawEvent(event)` 信号、`refreshMonitors()`、`dispatch(request)`、`eventSocketPath`。
- `HyprlandMonitor` 对象：`id`/`name`/`description`/`x`/`y`/`width`/`height`/`scale`/`focused`，外加 **`lastIpcObject`**——即 `hyprctl monitors -j` 那条 JSON 原对象，**全字段都在**（`refreshRate`/`transform`/`availableModes`/`make`/`model`/`serial`/`disabled` 等）。

方案：
- 事件检测：`Connections { target: Hyprland; function onRawEvent(e) {...} }`，对 monitor 相关事件去抖后调用 `Hyprland.refreshMonitors()`。**事件驱动，无定时轮询**，不自己 spawn `socat`。
- 取数：读 `Hyprland.monitors[*].lastIpcObject` 拿全字段。
- 应用：`hyprctl eval '<hl.monitor(...) lua chunk>'`。**注意**：Hyprland lua 模式（non-legacy parser）下运行时 `hyprctl keyword` 已被禁用（报 `keyword can't work with non-legacy parsers. Use eval.`），必须用 `hyprctl eval` 跑 `hl.monitor({...})`。多屏拼成一个 lua chunk 一次 eval；chunk 作为单个 argv 直传，免 shell 转义。判错看 stdout（成功 `ok`、失败 `error: ...`，退出码均 0，不可靠）。

`lastIpcObject` 关键字段：`name`、`description`、`make`、`model`、`serial`、`width`、`height`、`refreshRate`、`x`、`y`、`scale`、`transform`、`disabled`、`focused`、`availableModes`。

## 组合签名

为「记住每种显示器组合的布局」，需要一个对同一套物理显示器稳定、对名字变化不敏感的键。

- 每块在场显示器取**稳定标识**：优先 `description`（内含 EDID 厂商/型号/序列），否则 `make|model|serial`，最后兜底 `name`。
- 对在场显示器的稳定标识集合排序后拼接 → 组合签名。
- 不使用 `name`（`DP-3` 等会随接口变化），稳定标识来自 EDID 不变。

## 存储 schema

`~/.local/state/quickshell/monitor-profiles.json`（机器本地，XDG_STATE_HOME，不入 git）：

```json
{
  "version": 1,
  "profiles": {
    "<组合签名>": {
      "primary": "<稳定id>",
      "monitors": {
        "<稳定id>": {
          "name": "eDP-1",
          "enabled": true,
          "mode": "2560x1600@60",
          "position": "640x2160",
          "scale": 1.0,
          "transform": 0,
          "mirror": null
        }
      }
    }
  }
}
```

- 以**稳定 id** 为 key（非 name）；apply 时再映射回当前 name。
- `version` 预留迁移；损坏时备份原文件并重置为空（见错误处理）。

## 开机衔接（无闪烁）

Hyprland 的 lua 入口「一旦存在即完全接管、忽略所有 .conf」，故持久化只能走 lua。`hyprland.lua` 显示器段：

```lua
local mlocal = HOME .. "/.local/state/hypr/monitors.local.lua"
local chunk = loadfile(mlocal)            -- 文件缺失返回 nil
local ok = chunk and pcall(chunk)         -- 解析/运行错误被兜住
if not ok then
    hl.monitor({ output = "eDP-1", mode = "preferred", position = "auto", scale = 1 })  -- 安全默认
end
```

- `monitors.local.lua` 由 `MonitorService` 每次成功 apply 后回写，内容是当前布局对应的 `hl.monitor({...})` 行。
- 常见情形（开机时物理组合 = 上次关机时组合）下，开机直接由该文件恢复、无闪烁。
- quickshell 启动后 `MonitorService` 再做一次**对账**：查询实际显示器 → 算签名 → 若存档与当前不一致（例如上次双屏、本次仅笔记本），按签名存档重新 apply。

## apply / 回滚机制

1. 用户在面板编辑 → 内存中累积 pending 布局。
2. 点「应用」→ `MonitorService.applyLayout(pending)`：
   1. 快照当前生效布局（供回滚）。
   2. 逐屏 `hyprctl keyword monitor <串>` 应用。
   3. 启动 15 秒回滚定时器；UI 显示 `RevertConfirm` 倒计时条。
3. 「保留」→ 提交：写 `profiles.json[签名]` + 回写 `monitors.local.lua` + 取消定时器。
4. 「恢复」或 15 秒超时 → 重放快照、丢弃 pending。

应用走 `hyprctl eval`（见上）；`monitorModel.serializeMonitorString`（`NAME,RES@RATE,XxY,SCALE[,transform,N]` / `NAME,disable`）现仅用于 `_differsFromCurrent` 的状态比对，不再用于实际应用。实际应用用 `monitorModel.monitorLuaLine` 生成的 `hl.monitor({...})`。

## 热插拔自动应用

- socket2 显示器增删事件 → **去抖**（插拔常连发多个事件，合并到稳定态再处理）。
- 重算签名：
  - **命中**已存档 → 直接应用该存档布局（已知良配，**不走回滚条**）+ 回写 `monitors.local.lua`。
  - **未命中**（全新组合）→ 保持 Hyprland 默认（preferred/auto），发通知「新显示器组合，点此配置」，不自动改动。

## 错误处理

- `hyprctl` 应用失败 → 面板内红字提示，不写存储、不回写 local.lua。
- `profiles.json` 解析失败/损坏 → 备份为 `monitor-profiles.json.bak`，重置为空 schema，warn。
- 签名无存档 → 用默认，不崩。
- 禁用「最后一块启用的显示器」→ 拦截并警告（防止全黑无法操作）。
- 位置重叠/空隙：画布吸附尽量避免；Hyprland 容忍空隙，不做硬校验。

## 侧边栏入口

`QuickSettings.qml`（左侧滑出面板）现有 458–484 行的「显示器切换」三按钮（双屏/外接/笔记本，调用 monitor_profile.sh）整体替换为单个「显示器」入口：

```qml
// 点击：关侧边栏 + 打开新显示器面板，沿用 WiFi/蓝牙 onRightClicked 范式
onClicked: {
    PanelState.settingsOpen = false;
    PanelState.toggleDisplay();
}
```

- `shell.qml` 中已注册的 `GlobalShortcut { name: "display" }` 保留（无害，将来可绑键），但不是主入口，**本期不绑任何快捷键**。
- `PanelState.toggleDisplay()` / `displayOpen` 复用现有定义。

## 测试

- **纯函数**（`display/lib/monitorModel.js`：签名计算、布局↔monitor 串序列化、存储读写与 version 迁移）→ 用 node 跑断言（纯 JS，无 QML 依赖）。
- **手动验证矩阵**（IPC/UI 部分）：
  1. 仅笔记本屏
  2. 笔记本 + 外接（双屏）拖拽摆位
  3. 仅外接
  4. 双向热插拔（拔→恢复单屏布局、插→恢复双屏布局）
  5. 故意配错分辨率/位置 → 15 秒回滚生效
  6. 重启后布局持久化（monitors.local.lua 恢复，无闪烁）
  7. 全新显示器组合 → 收到通知、不自动改动

## 文档维护

- 本改动使用 `hyprland.lua` 既有的 `hl.monitor` / `dofile`，**未改动 cli/lua-api 本身**，故无需更新 `docs/LUA_API.md`。
- 不涉及 AI 工具链，无需更新 `docs/AI_TOOLING.md`。

## 验收标准

- 侧边栏点「显示器」滑出新面板；旧三按钮与 monitor_profile.sh 已移除。
- 可拖拽摆位并吸附对齐；可改分辨率/刷新率/缩放/旋转/主屏/开关。
- apply 出现 15 秒回滚确认；超时/恢复可还原。
- 重启后布局保持；热插拔已知组合自动恢复、未知组合发通知。
- 无定时轮询；显示器逻辑收口在 `MonitorService` 单例。
```
