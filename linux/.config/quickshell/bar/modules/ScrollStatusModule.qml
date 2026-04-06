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
                    active: isActive
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

    accentColor: Colors.yellow
    implicitWidth: contentRow.implicitWidth + 32
    Component.onCompleted: fetchProc.running = true
    onScrolled: delta => {
        Hyprland.dispatch("layoutmsg " + (delta > 0 ? "colresize +0.05" : "colresize -0.05"));
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

    Timer {
        interval: 100
        running: true
        repeat: true
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
        }

        Row {
            visible: root.specialState === "" && root.layoutData.length > 0
            spacing: 2
            anchors.verticalCenter: parent.verticalCenter

            Repeater {
                model: root.layoutData

                delegate: Column {
                    id: colDelegate
                    required property var modelData
                    required property int index

                    property bool colFocused: modelData.focused
                    property int winCount: modelData.windows.length

                    spacing: 1
                    anchors.verticalCenter: parent.verticalCenter

                    Repeater {
                        model: colDelegate.modelData.windows

                        delegate: Rectangle {
                            required property var modelData
                            required property int index

                            width: 10
                            height: Math.max(4, Math.floor(24 / colDelegate.winCount) - 1)
                            radius: 2
                            color: modelData.active ? Colors.yellow : colDelegate.colFocused ? Qt.rgba(Colors.yellow.r, Colors.yellow.g, Colors.yellow.b, 0.4) : Colors.surface2

                            Behavior on color {
                                ColorAnimation {
                                    duration: Tokens.animFast
                                }
                            }
                        }
                    }
                }
            }
        }

    }
}
