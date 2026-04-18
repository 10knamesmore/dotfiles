import "../components"
import "../theme"
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Wayland

// 显示器管理面板 — 查看和修改多显示器配置
PanelOverlay {
    id: root

    property var monitors: []
    property int selectedIndex: 0
    property bool applying: false
    property string errorMsg: ""
    property string _jsonBuf: ""

    function queryMonitors() {
        root._jsonBuf = "";
        queryProc.running = true;
    }

    function applyConfig(config) {
        root.applying = true;
        root.errorMsg = "";
        if (!config.enabled) {
            applyProc.command = ["hyprctl", "keyword", "monitor", config.name + ",disable"];
        } else {
            let monStr = config.name
                + "," + config.res + "@" + config.rate
                + "," + config.x + "x" + config.y
                + "," + config.scale;
            if (config.transform > 0)
                monStr += ",transform," + config.transform;
            applyProc.command = ["hyprctl", "keyword", "monitor", monStr];
        }
        applyProc.running = true;
    }

    showing: PanelState.displayOpen
    panelWidth: 520
    panelHeight: root.height * 0.75
    panelTargetX: (root.width - 520) / 2
    panelTargetY: 54
    closedOffsetY: -20
    onCloseRequested: PanelState.displayOpen = false
    onShowingChanged: {
        if (showing) {
            errorMsg = "";
            applying = false;
            queryMonitors();
        }
    }

    // 定时刷新（热插拔检测）
    Timer {
        running: root.showing
        interval: 10000
        repeat: true
        onTriggered: queryMonitors()
    }

    // ── 查询显示器 ──
    Process {
        id: queryProc
        command: ["hyprctl", "monitors", "-j"]
        onStarted: root._jsonBuf = ""
        stdout: SplitParser {
            onRead: (data) => { root._jsonBuf += data + "\n"; }
        }
        onExited: (code, status) => {
            if (code !== 0) {
                root.errorMsg = "查询失败";
                return;
            }
            try {
                let arr = JSON.parse(root._jsonBuf);
                root.monitors = arr.map(m => ({
                    "name": m.name,
                    "description": m.description || "",
                    "model": m.model || "",
                    "width": m.width,
                    "height": m.height,
                    "refreshRate": m.refreshRate,
                    "x": m.x,
                    "y": m.y,
                    "scale": m.scale,
                    "transform": m.transform,
                    "disabled": m.disabled || false,
                    "availableModes": m.availableModes || []
                }));
                if (root.selectedIndex >= root.monitors.length)
                    root.selectedIndex = 0;
            } catch (e) {
                root.errorMsg = "JSON 解析失败";
            }
        }
    }

    // ── 应用配置 ──
    Process {
        id: applyProc
        property string _errBuf: ""
        onStarted: _errBuf = ""
        onExited: (code, status) => {
            root.applying = false;
            if (code === 0) {
                root.errorMsg = "";
                // 延迟刷新，等 Hyprland 重排
                refreshDelay.start();
            } else {
                root.errorMsg = applyProc._errBuf || "应用失败";
            }
        }
        stderr: SplitParser {
            onRead: (data) => { applyProc._errBuf += data; }
        }
    }

    Timer {
        id: refreshDelay
        interval: 500
        onTriggered: root.queryMonitors()
    }

    // ── UI ──
    ColumnLayout {
        anchors.fill: parent
        anchors.margins: Tokens.spaceL
        spacing: Tokens.spaceM

        // ── 标题 ──
        RowLayout {
            Layout.fillWidth: true

            Text {
                text: "󰍹"
                color: Colors.blue
                font.family: Fonts.family
                font.pixelSize: Fonts.title
            }

            Text {
                text: "显示器"
                font.family: Fonts.family
                font.pixelSize: Fonts.title
                font.bold: true
                color: Colors.text
            }

            Item { Layout.fillWidth: true }

            // 刷新
            Rectangle {
                width: 28
                height: 28
                radius: Tokens.radiusFull
                color: dispRefreshArea.containsMouse ? Colors.surface2 : "transparent"

                Text {
                    anchors.centerIn: parent
                    text: "󰑓"
                    color: dispRefreshArea.containsMouse ? Colors.blue : Colors.subtext0
                    font.family: Fonts.family
                    font.pixelSize: Fonts.icon
                    Behavior on color { ColorAnimation { duration: 150 } }
                }

                MouseArea {
                    id: dispRefreshArea
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: root.queryMonitors()
                }

                Behavior on color { ColorAnimation { duration: 150 } }
            }

            Text {
                text: root.monitors.length + " 台显示器"
                color: Colors.subtext0
                font.family: Fonts.family
                font.pixelSize: Fonts.small
            }
        }

        // ── 布局预览 ──
        MonitorPreview {
            Layout.fillWidth: true
            implicitHeight: 150
            monitors: root.monitors
            selectedIndex: root.selectedIndex
            onMonitorClicked: (idx) => root.selectedIndex = idx
        }

        Divider { Layout.fillWidth: true }

        // ── 选中显示器的配置 ──
        Flickable {
            Layout.fillWidth: true
            Layout.fillHeight: true
            contentHeight: cardCol.implicitHeight
            clip: true

            ColumnLayout {
                id: cardCol
                width: parent.width
                spacing: 0

                // 无显示器
                Text {
                    visible: root.monitors.length === 0
                    text: "未检测到显示器"
                    color: Colors.overlay0
                    font.family: Fonts.family
                    font.pixelSize: Fonts.bodyLarge
                    Layout.alignment: Qt.AlignHCenter
                    Layout.topMargin: 20
                }

                // 显示器配置卡片
                MonitorCard {
                    visible: root.monitors.length > 0 && root.selectedIndex < root.monitors.length
                    Layout.fillWidth: true
                    monitor: root.monitors.length > root.selectedIndex ? root.monitors[root.selectedIndex] : null
                    onApplyRequested: (config) => root.applyConfig(config)
                }
            }
        }

        // ── 状态信息 ──
        Text {
            visible: root.applying
            text: "正在应用..."
            color: Colors.blue
            font.family: Fonts.family
            font.pixelSize: Fonts.small
        }

        Text {
            visible: root.errorMsg !== ""
            text: "⚠ " + root.errorMsg
            color: Colors.red
            font.family: Fonts.family
            font.pixelSize: Fonts.small
            wrapMode: Text.WordWrap
            Layout.fillWidth: true
        }
    }
}
