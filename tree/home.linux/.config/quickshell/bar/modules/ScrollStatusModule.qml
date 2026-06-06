import "../../theme"
import "../components"
import QtQuick
import QtQuick.Layouts
import Quickshell.Hyprland._Ipc
import Quickshell.Io

// Scrolling layout 微缩窗口布局图 — 每个显示器独立
BarModule {
    id: root

    property var barScreen: null

    // 解析后的布局数据
    property var layoutData: []
    property int curCol: 0
    property int totalCols: 0
    property string specialState: ""

    property string _raw: ""

    function updateLayout() {
        if (!_raw || !barScreen)
            return;

        // 拆分三段 JSON
        let parts = _raw.split("|||");
        if (parts.length < 3)
            return;

        let clients, monitors, activeWin;
        try {
            clients = JSON.parse(parts[0]);
            monitors = JSON.parse(parts[1]);
            activeWin = JSON.parse(parts[2]);
        } catch (e) {
            specialState = "empty";
            layoutData = [];
            return;
        }

        // 找本显示器的 active workspace
        let monName = barScreen.name;
        let wsId = -1;
        for (let m of monitors) {
            if (m.name === monName) {
                wsId = m.activeWorkspace.id;
                break;
            }
        }
        if (wsId < 0) {
            specialState = "empty";
            layoutData = [];
            totalCols = 0;
            return;
        }

        // 找本 workspace 的 focus 窗口（focusHistoryID 最小）
        let wsClients = [];
        for (let c of clients) {
            if (c.workspace.id === wsId && !c.floating && c.mapped)
                wsClients.push(c);
        }

        if (wsClients.length === 0) {
            specialState = "empty";
            layoutData = [];
            totalCols = 0;
            return;
        }

        // 确定当前焦点窗口
        let focusAddr = "";
        if (activeWin && activeWin.address && activeWin.workspace && activeWin.workspace.id === wsId) {
            // activewindow 就在本 workspace
            if (activeWin.floating) {
                specialState = "float";
                layoutData = [];
                return;
            }
            if (activeWin.fullscreen > 0) {
                specialState = "fullscreen";
                layoutData = [];
                return;
            }
            focusAddr = activeWin.address;
        } else {
            // activewindow 在别的 monitor，找本 workspace focusHistoryID 最小的
            let best = null;
            for (let c of wsClients) {
                if (!best || c.focusHistoryID < best.focusHistoryID)
                    best = c;
            }
            if (best)
                focusAddr = best.address;
        }

        specialState = "";

        // 按 X 坐标分组
        let colMap = {};
        for (let w of wsClients) {
            let x = w.at[0];
            if (!colMap[x])
                colMap[x] = [];
            colMap[x].push(w);
        }

        let xKeys = Object.keys(colMap).sort((a, b) => Number(a) - Number(b));
        let result = [];
        let focusedColIdx = -1;

        for (let ci = 0; ci < xKeys.length; ci++) {
            let colWins = colMap[xKeys[ci]];
            colWins.sort((a, b) => a.at[1] - b.at[1]);

            let isFocusedCol = false;
            let windowList = [];
            for (let w of colWins) {
                let isActive = w.address === focusAddr;
                if (isActive)
                    isFocusedCol = true;
                windowList.push({
                    active: isActive,
                    address: w.address
                });
            }

            if (isFocusedCol)
                focusedColIdx = ci;
            result.push({
                focused: isFocusedCol,
                windows: windowList
            });
        }

        layoutData = result;
        totalCols = result.length;
        curCol = focusedColIdx + 1;
    }

    // 当前 hover 命中的窗口地址（驱动方块高亮）
    property string hoverAddress: ""

    // 命中测试：找离 (mx,my) 最近的窗口微缩方块（带容差，方块只有 10px 宽）。
    // BarModule 的 hoverArea 盖在内容之上，delegate 内 MouseArea/HoverHandler 收不到事件，
    // 故点击/hover 都走 BarModule 信号 + 本函数。
    function rectAt(mx, my) {
        if (specialState !== "" || layoutData.length === 0)
            return null;
        let best = null, bestDist = 1e9;
        for (let ci = 0; ci < colRepeater.count; ci++) {
            let col = colRepeater.itemAt(ci);
            if (!col)
                continue;
            for (let wi = 0; wi < col.winRepeater.count; wi++) {
                let r = col.winRepeater.itemAt(wi);
                if (!r)
                    continue;
                let p = r.mapToItem(root, r.width / 2, r.height / 2);
                let d = Math.hypot(p.x - mx, p.y - my);
                if (d < bestDist) {
                    bestDist = d;
                    best = r;
                }
            }
        }
        return bestDist < 24 ? best : null;
    }

    accentColor: Colors.yellow
    implicitWidth: contentRow.implicitWidth + 32
    Component.onCompleted: fetchProc.running = true
    onClicked: mouse => {
        let r = rectAt(mouse.x, mouse.y);
        if (r)
            Hyprland.dispatch('hl.dsp.focus({ window = "address:' + r.winAddress + '" })');
    }
    onMoved: mouse => {
        let r = rectAt(mouse.x, mouse.y);
        hoverAddress = r ? r.winAddress : "";
    }
    onHoveredChanged: {
        if (!hovered)
            hoverAddress = "";
    }
    onScrolled: delta => {
        // Hyprland 7.0+ dispatch 串按 lua 解析，须用 hl.dsp.* dispatcher 写法
        Hyprland.dispatch('hl.dsp.layout("colresize ' + (delta > 0 ? "+" : "-") + '0.05")');
    }

    // 单个 Process 一次取全
    Process {
        id: fetchProc
        property string _buf: ""
        command: ["sh", "-c", "hyprctl -j clients; echo '|||'; hyprctl -j monitors; echo '|||'; hyprctl -j activewindow"]
        stdout: SplitParser {
            onRead: data => fetchProc._buf += data + "\n"
        }
        onExited: {
            root._raw = fetchProc._buf;
            fetchProc._buf = "";
            root.updateLayout();
        }
    }

    // 事件驱动：监听 Hyprland IPC 事件，仅在窗口/工作区变化时 fetch（debounce 合并突发事件）。
    // 取代旧的 100ms 无条件轮询 —— 空闲时完全不调用 hyprctl。
    Connections {
        target: Hyprland
        function onRawEvent(event) {
            fetchDebounce.restart();
        }
    }

    Timer {
        id: fetchDebounce
        interval: 80
        onTriggered: {
            fetchProc.running = false;
            fetchProc.running = true;
        }
    }

    // ── 视觉 ──
    Row {
        id: contentRow
        anchors.centerIn: parent
        spacing: 4

        Text {
            visible: root.specialState !== ""
            text: root.specialState === "float" ? "󰹙" : root.specialState === "fullscreen" ? "󰊓" : "󰽥"
            color: Colors.yellow
            font.family: Fonts.family
            font.pixelSize: Fonts.title
            anchors.verticalCenter: parent.verticalCenter

            Behavior on color {
                ColorAnimation {
                    duration: Tokens.animFast
                }
            }
        }

        // hover 时显示列位置文字
        Text {
            visible: root.hovered && root.specialState === "" && root.totalCols > 0
            text: root.curCol + "/" + root.totalCols
            color: Colors.yellow
            font.family: Fonts.family
            font.pixelSize: Fonts.caption
            font.weight: Font.DemiBold
            anchors.verticalCenter: parent.verticalCenter
            opacity: root.hovered ? 1 : 0

            Behavior on opacity {
                NumberAnimation {
                    duration: Tokens.animNormal
                }
            }
        }

        Row {
            visible: root.specialState === "" && root.layoutData.length > 0
            spacing: 2
            anchors.verticalCenter: parent.verticalCenter

            Repeater {
                id: colRepeater

                model: root.layoutData

                delegate: Column {
                    id: colDelegate
                    required property var modelData
                    required property int index

                    property bool colFocused: modelData.focused
                    property int winCount: modelData.windows.length
                    property alias winRepeater: winRepeater

                    spacing: 1
                    anchors.verticalCenter: parent.verticalCenter

                    Repeater {
                        id: winRepeater

                        model: colDelegate.modelData.windows

                        delegate: Rectangle {
                            required property var modelData
                            required property int index

                            property string winAddress: modelData.address
                            property bool hoverHit: root.hoverAddress !== "" && root.hoverAddress === winAddress

                            width: 10
                            height: Math.max(4, Math.floor(24 / colDelegate.winCount) - 1)
                            radius: 2
                            scale: hoverHit ? 1.25 : 1
                            color: modelData.active ? Colors.yellow : hoverHit ? Qt.rgba(Colors.yellow.r, Colors.yellow.g, Colors.yellow.b, 0.85) : colDelegate.colFocused ? Qt.rgba(Colors.yellow.r, Colors.yellow.g, Colors.yellow.b, root.hovered ? 0.6 : 0.4) : (root.hovered ? Colors.surface1 : Colors.surface2)

                            Behavior on color {
                                ColorAnimation {
                                    duration: Tokens.animFast
                                }
                            }

                            Behavior on scale {
                                NumberAnimation {
                                    duration: Tokens.animFast
                                    easing.type: Easing.OutCubic
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
