import "../state"
import "../display/lib/monitorModel.js" as MM
import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Hyprland

// 显示器管理常驻服务（仿 SystemStatsService：非单例 Scope，在 shell.qml 实例化一次）。
// 负责：监听 Hyprland 显示器事件（事件驱动，无轮询）、查询、按组合签名记忆/恢复布局、
// 应用 + 15s 回滚、读写存储与开机 lua。数据写入 MonitorState 供 UI 读取。
Scope {
    id: root

    readonly property string _home: Quickshell.env("HOME")
    readonly property string _storePath: _home + "/.local/state/quickshell/monitor-profiles.json"
    // 开机布局文件落 ~/.local/state（非 dots 托管目录）：~/.config/hypr 是整目录软链进仓库，
    // 写那里会污染 git；写 state 目录则彻底与仓库隔离。hyprland.lua 从同一路径 loadfile。
    readonly property string _luaPath: _home + "/.local/state/hypr/monitors.local.lua"
    readonly property int _revertSeconds: 15

    property var _store: ({ version: 1, profiles: {} })
    property var _lastIpc: []           // 最近一次 hyprctl monitors -j 的原始数组
    property var _snapshot: []          // 应用前的布局快照（供回滚）
    property var _pending: []           // 待确认的 pending 布局
    property string _pendingPrimary: ""
    property bool _suspendReconcile: false  // 自己应用导致的事件不触发再对账
    property bool _reconcileAfterQuery: false
    property var _notifiedSigs: ({})    // 已就「新组合」通知过的签名集

    // ── 当前显示器（state 形态）→ 布局列表 ──
    function _monitorsToLayouts(mons) {
        return mons.map(function (m) {
            return { name: m.name, enabled: m.enabled, mode: m.mode, x: m.x, y: m.y, scale: m.scale, transform: m.transform || 0, mirror: m.mirror || null };
        });
    }

    // ── 查询（事件触发，非轮询）──
    function queryMonitors(reconcile) {
        root._reconcileAfterQuery = !!reconcile;
        queryProc.running = true;
    }

    function _onMonitorsLoaded(text) {
        var arr;
        try {
            arr = JSON.parse(text);
        } catch (e) {
            MonitorState.errorMsg = "显示器信息解析失败";
            return;
        }
        root._lastIpc = arr;
        var sig = MM.signature(arr);

        // 主屏：优先沿用已存档 profile 的 primary（映射到当前 name），否则焦点屏
        var primaryName = "";
        var profile = MM.getProfile(root._store, sig);
        if (profile && profile.primary) {
            arr.forEach(function (ipc) {
                if (MM.stableId(ipc) === profile.primary)
                    primaryName = ipc.name;
            });
        }
        if (primaryName === "") {
            arr.forEach(function (ipc) {
                if (ipc.focused)
                    primaryName = ipc.name;
            });
        }

        MonitorState.monitors = arr.map(function (ipc) {
            return {
                "name": ipc.name,
                "description": ipc.description || "",
                "enabled": !(ipc.disabled),
                "mode": MM.formatMode(ipc.width, ipc.height, ipc.refreshRate),
                "x": ipc.x,
                "y": ipc.y,
                "scale": ipc.scale,
                "transform": ipc.transform || 0,
                "width": ipc.width,
                "height": ipc.height,
                "refreshRate": ipc.refreshRate,
                "availableModes": ipc.availableModes || [],
                "focused": ipc.focused || false,
                "mirror": null,
                "primary": ipc.name === primaryName
            };
        });
        MonitorState.signature = sig;
        MonitorState.primaryName = primaryName;

        if (root._reconcileAfterQuery && !root._suspendReconcile)
            _reconcile(arr, sig);
    }

    // ── 对账：开机 / 热插拔时按签名恢复已存布局 ──
    function _reconcile(arr, sig) {
        var profile = MM.getProfile(root._store, sig);
        console.log("[MonitorService] reconcile sig=", sig, "profileFound=", !!profile, "suspend=", root._suspendReconcile); // [诊断] 临时
        if (!profile) {
            _notifyNewCombo(sig);
            return;
        }
        var layouts = MM.layoutFromProfile(profile, arr);
        if (layouts.length === 0)
            return;
        if (_differsFromCurrent(layouts, arr))
            _applyLayouts(layouts, false); // 已知良配，直接恢复，不走回滚条
        else
            _writeLua(layouts);            // 已一致，仅确保开机文件最新
    }

    function _differsFromCurrent(layouts, arr) {
        var cur = {};
        arr.forEach(function (ipc) {
            cur[ipc.name] = MM.serializeMonitorString(MM.layoutFromIpc(ipc));
        });
        for (var i = 0; i < layouts.length; i++) {
            if (cur[layouts[i].name] !== MM.serializeMonitorString(layouts[i]))
                return true;
        }
        return false;
    }

    // ── 应用 ──
    function _applyLayouts(layouts, withRevert) {
        var anyEnabled = layouts.some(function (l) { return l.enabled; });
        if (!anyEnabled) {
            MonitorState.errorMsg = "不能禁用全部显示器";
            return;
        }
        root._suspendReconcile = true;
        // Hyprland lua 模式（non-legacy parser）禁用了运行时 hyprctl keyword（会报
        // "keyword can't work with non-legacy parsers. Use eval."）。须用 hyprctl eval
        // 跑 hl.monitor(...)。chunk 作为单个 argv 直传，无需 shell 转义。
        var chunk = layouts.map(function (l) { return MM.monitorLuaLine(l); }).join("\n");
        applyProc._withRevert = withRevert;
        applyProc._layouts = layouts;
        applyProc.command = ["hyprctl", "eval", chunk];
        MonitorState.applying = true;
        MonitorState.errorMsg = "";
        applyProc.running = true;
    }

    function _writeLua(layouts) {
        luaWriter.setText(MM.buildLocalLua(layouts));
    }

    function _writeStore() {
        storeWriter.setText(JSON.stringify(root._store, null, 2));
    }

    function _notifyNewCombo(sig) {
        if (root._notifiedSigs[sig])
            return;
        root._notifiedSigs[sig] = true;
        notifyProc.command = ["notify-send", "-a", "显示器", "新的显示器组合", "点击侧边栏「显示器」进行配置"];
        notifyProc.running = true;
    }

    // ── UI 意图回调 ──
    Connections {
        target: MonitorState

        function onApplyRequested(layouts, primary) {
            root._snapshot = root._monitorsToLayouts(MonitorState.monitors);
            root._pending = layouts;
            root._pendingPrimary = primary;
            root._applyLayouts(layouts, true);
        }
        function onKeepRequested() {
            countdown.stop();
            MonitorState.revertSecs = 0;
            root._store = MM.putProfile(root._store, MonitorState.signature, MM.profileFromLayouts(root._pending, root._lastIpc, root._pendingPrimary));
            root._writeStore();
            root._writeLua(root._pending);
            MonitorState.primaryName = root._pendingPrimary;
            root._suspendReconcile = false;
        }
        function onRevertRequested() {
            countdown.stop();
            MonitorState.revertSecs = 0;
            root._applyLayouts(root._snapshot, false);
        }
        function onRefreshRequested() {
            root.queryMonitors(false);
        }
    }

    // ── Hyprland 事件（事件驱动；去抖后重查）──
    Connections {
        target: Hyprland

        function onRawEvent(event) {
            // [诊断] 临时：打印所有含 mon 的事件名，确认 rawEvent 是否触发
            if (event.name.toLowerCase().indexOf("mon") >= 0)
                console.log("[MonitorService] rawEvent:", event.name);
            // 只关心显示器增删等（monitoradded/monitorremoved[v2]）；focusedmon 太频繁，忽略
            if (event.name.indexOf("monitor") === 0 && event.name.indexOf("monitorfocus") !== 0)
                debounce.restart();
        }
    }

    Timer {
        id: debounce
        interval: 300
        onTriggered: {
            console.log("[MonitorService] debounce → query+reconcile"); // [诊断] 临时
            root.queryMonitors(true);
        }
    }

    // ── 回滚倒计时 ──
    Timer {
        id: countdown
        interval: 1000
        repeat: true
        onTriggered: {
            MonitorState.revertSecs -= 1;
            if (MonitorState.revertSecs <= 0) {
                countdown.stop();
                MonitorState.revert(); // 超时自动恢复
            }
        }
    }

    // ── 进程 ──
    Process {
        id: queryProc
        command: ["hyprctl", "monitors", "-j"]
        stdout: StdioCollector {
            onStreamFinished: root._onMonitorsLoaded(text)
        }
    }

    Process {
        id: applyProc
        property bool _withRevert: false
        property var _layouts: []
        // hyprctl eval 成功打 "ok"、失败打 "error: ..."，退出码均为 0，故按 stdout 文本判错。
        stdout: StdioCollector {
            onStreamFinished: {
                MonitorState.applying = false;
                if (text.indexOf("error") < 0 && text.indexOf("Error") < 0) {
                    MonitorState.errorMsg = "";
                    root.queryMonitors(false); // 刷新显示，不再对账
                    if (applyProc._withRevert) {
                        MonitorState.revertSecs = root._revertSeconds;
                        countdown.start();
                    } else {
                        root._suspendReconcile = false;
                        root._writeLua(applyProc._layouts); // 自动/恢复应用也持久化开机文件
                    }
                } else {
                    MonitorState.errorMsg = text.trim() || "应用失败";
                    root._suspendReconcile = false;
                }
            }
        }
    }

    Process {
        id: storeReadProc
        command: ["cat", root._storePath]
        stdout: StdioCollector {
            onStreamFinished: {
                root._store = MM.migrateStore(text);
                root.queryMonitors(true); // 存储就绪后做开机对账
            }
        }
    }

    Process { id: notifyProc }
    Process { id: mkdirProc }

    // ── 文件写入（FileView：正确处理 UTF-8 + 原子写）──
    // 仅用于写（setText）。printErrors:false 抑制「文件首次不存在」的读取告警。
    FileView {
        id: storeWriter
        path: root._storePath
        atomicWrites: true
        printErrors: false
    }
    FileView {
        id: luaWriter
        path: root._luaPath
        atomicWrites: true
        printErrors: false
    }

    Component.onCompleted: {
        mkdirProc.command = ["mkdir", "-p", root._home + "/.local/state/quickshell", root._home + "/.local/state/hypr"];
        mkdirProc.running = true;
        storeReadProc.running = true; // 读存储 → 开机对账
    }
}
