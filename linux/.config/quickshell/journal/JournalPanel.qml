import "../components"
import "../theme"
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io

// 系统日志流面板 — 右侧滑出，journalctl 实时日志
PanelOverlay {
    id: root

    showing: PanelState.journalOpen
    panelWidth: 880
    panelHeight: root.height * 0.7
    onCloseRequested: PanelState.journalOpen = false

    property bool paused: false
    property int maxEntries: 500
    property int filterPriority: -1 // -1=全部, 0-7=指定优先级及以上
    property string filterUnit: ""

    function priorityColor(p) {
        if (p <= 2)
            return Colors.red;       // emerg/alert/crit
        if (p === 3)
            return Colors.red;       // error
        if (p === 4)
            return Colors.yellow;    // warning
        if (p === 5)
            return Colors.peach;     // notice
        if (p === 6)
            return Colors.blue;      // info
        return Colors.overlay0;               // debug
    }

    function priorityLabel(p) {
        let labels = ["EMRG", "ALRT", "CRIT", "ERR", "WARN", "NOTC", "INFO", "DBG"];
        return (p >= 0 && p <= 7) ? labels[p] : "???";
    }

    function formatTimestamp(usec) {
        try {
            let ms = parseInt(usec) / 1000;
            let d = new Date(ms);
            let h = d.getHours().toString().padStart(2, '0');
            let m = d.getMinutes().toString().padStart(2, '0');
            let s = d.getSeconds().toString().padStart(2, '0');
            return h + ":" + m + ":" + s;
        } catch (e) {
            return "??:??:??";
        }
    }

    function matchesFilter(priority, unit) {
        if (filterPriority >= 0 && priority > filterPriority)
            return false;
        if (filterUnit.length > 0 && !unit.toLowerCase().includes(filterUnit.toLowerCase()))
            return false;
        return true;
    }

    function addEntry(obj) {
        if (paused)
            return;

        let priority = parseInt(obj.PRIORITY || "6");
        let unit = obj.SYSLOG_IDENTIFIER || obj._COMM || "unknown";
        let message = obj.MESSAGE || "";
        let timestamp = obj.__REALTIME_TIMESTAMP || "0";

        if (!matchesFilter(priority, unit))
            return;

        logModel.append({
            "priority": priority,
            "unit": unit,
            "message": message,
            "time": formatTimestamp(timestamp)
        });

        // 限制条目数
        while (logModel.count > maxEntries)
            logModel.remove(0);

        // 自动滚动到底部
        scrollTimer.start();
    }

    onShowingChanged: {
        if (showing) {
            logModel.clear();
            paused = false;
            journalProc.running = true;
            focusTimer.start();
        } else {
            journalProc.running = false;
        }
    }

    ListModel {
        id: logModel
    }

    Timer {
        id: scrollTimer
        interval: 50
        onTriggered: {
            if (logView.count > 0)
                logView.positionViewAtEnd();
        }
    }

    Timer {
        id: focusTimer
        interval: 50
        onTriggered: unitInput.forceActiveFocus()
    }

    // ── journalctl 进程 ──
    Process {
        id: journalProc

        property string _buf: ""

        command: ["journalctl", "-f", "-o", "json", "--no-pager", "-n", "200"]

        stdout: SplitParser {
            onRead: data => {
                try {
                    let obj = JSON.parse(data);
                    root.addEntry(obj);
                } catch (e) {
                    // 忽略非 JSON 行
                }
            }
        }
    }

    // ── UI ──
    ColumnLayout {
        id: col

        anchors.fill: parent
        anchors.margins: Tokens.spaceL
        spacing: Tokens.spaceS

        // ── 标题栏 ──
        RowLayout {
            Layout.fillWidth: true

            Text {
                text: " 系统日志"
                font.family: Fonts.family
                font.pixelSize: Fonts.title
                font.bold: true
                color: Colors.text
            }

            Item {
                Layout.fillWidth: true
            }

            Text {
                text: logModel.count + " 条"
                color: Colors.subtext0
                font.family: Fonts.family
                font.pixelSize: Fonts.small
            }

            // 暂停/继续
            Rectangle {
                width: pauseText.implicitWidth + 16
                height: 26
                radius: Tokens.radiusFull
                color: pauseArea.containsMouse ? Qt.rgba(Colors.yellow.r, Colors.yellow.g, Colors.yellow.b, 0.15) : "transparent"

                Text {
                    id: pauseText
                    anchors.centerIn: parent
                    text: root.paused ? "继续" : "暂停"
                    color: pauseArea.containsMouse ? Colors.yellow : Colors.subtext0
                    font.family: Fonts.family
                    font.pixelSize: Fonts.small

                    Behavior on color {
                        ColorAnimation {
                            duration: Tokens.animFast
                        }
                    }
                }

                MouseArea {
                    id: pauseArea
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: root.paused = !root.paused
                }

                Behavior on color {
                    ColorAnimation {
                        duration: Tokens.animFast
                    }
                }
            }

            // 清空
            Rectangle {
                width: clearText.implicitWidth + 16
                height: 26
                radius: Tokens.radiusFull
                color: clearArea.containsMouse ? Qt.rgba(Colors.red.r, Colors.red.g, Colors.red.b, 0.15) : "transparent"

                Text {
                    id: clearText
                    anchors.centerIn: parent
                    text: "清空"
                    color: clearArea.containsMouse ? Colors.red : Colors.subtext0
                    font.family: Fonts.family
                    font.pixelSize: Fonts.small

                    Behavior on color {
                        ColorAnimation {
                            duration: Tokens.animFast
                        }
                    }
                }

                MouseArea {
                    id: clearArea
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: logModel.clear()
                }

                Behavior on color {
                    ColorAnimation {
                        duration: Tokens.animFast
                    }
                }
            }
        }

        // ── 过滤栏 ──
        RowLayout {
            Layout.fillWidth: true
            spacing: Tokens.spaceS

            // 优先级过滤按钮
            Repeater {
                model: [
                    {
                        "label": "全部",
                        "value": -1
                    },
                    {
                        "label": "ERR",
                        "value": 3
                    },
                    {
                        "label": "WARN",
                        "value": 4
                    },
                    {
                        "label": "INFO",
                        "value": 6
                    }
                ]

                delegate: Rectangle {
                    required property var modelData

                    width: filterLabel.implicitWidth + 14
                    height: 24
                    radius: Tokens.radiusFull
                    color: root.filterPriority === modelData.value ? Qt.rgba(Colors.mauve.r, Colors.mauve.g, Colors.mauve.b, 0.2) : (filterBtnArea.containsMouse ? Colors.surface1 : "transparent")
                    border.color: root.filterPriority === modelData.value ? Qt.rgba(Colors.mauve.r, Colors.mauve.g, Colors.mauve.b, 0.4) : "transparent"
                    border.width: 1

                    Text {
                        id: filterLabel
                        anchors.centerIn: parent
                        text: modelData.label
                        color: root.filterPriority === modelData.value ? Colors.mauve : Colors.subtext0
                        font.family: Fonts.family
                        font.pixelSize: Fonts.small
                    }

                    MouseArea {
                        id: filterBtnArea
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: {
                            root.filterPriority = modelData.value;
                        }
                    }

                    Behavior on color {
                        ColorAnimation {
                            duration: Tokens.animFast
                        }
                    }
                }
            }

            Item {
                Layout.fillWidth: true
            }

            // Unit 过滤输入框
            Rectangle {
                Layout.preferredWidth: 140
                height: 26
                radius: Tokens.radiusMS
                color: Colors.surface1

                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: 8
                    anchors.rightMargin: 8
                    spacing: 4

                    Text {
                        text: ""
                        color: Colors.overlay1
                        font.family: Fonts.family
                        font.pixelSize: Fonts.small
                    }

                    TextInput {
                        id: unitInput
                        Layout.fillWidth: true
                        color: Colors.text
                        font.family: Fonts.family
                        font.pixelSize: Fonts.small
                        clip: true
                        selectByMouse: true
                        onTextChanged: root.filterUnit = text
                        Keys.onEscapePressed: PanelState.journalOpen = false

                        Text {
                            anchors.fill: parent
                            text: "按服务过滤..."
                            color: Colors.overlay0
                            font: parent.font
                            visible: !parent.text && !parent.activeFocus
                        }
                    }
                }
            }
        }

        Rectangle {
            Layout.fillWidth: true
            height: 1
            color: Colors.surface1
        }

        // ── 日志列表 ──
        ListView {
            id: logView

            Layout.fillWidth: true
            Layout.fillHeight: true
            model: logModel
            spacing: 2
            clip: true

            delegate: Rectangle {
                required property int index
                required property int priority
                required property string unit
                required property string message
                required property string time

                width: ListView.view.width
                height: logRow.implicitHeight + 8
                radius: Tokens.radiusS
                color: logHover.containsMouse ? Colors.surface1 : "transparent"

                MouseArea {
                    id: logHover
                    anchors.fill: parent
                    hoverEnabled: true
                    acceptedButtons: Qt.NoButton
                }

                RowLayout {
                    id: logRow
                    anchors {
                        left: parent.left
                        right: parent.right
                        top: parent.top
                        margins: 4
                    }
                    spacing: 6

                    // 时间
                    Text {
                        text: time
                        color: Colors.overlay1
                        font.family: Fonts.family
                        font.pixelSize: Fonts.xs
                        Layout.preferredWidth: 56
                    }

                    // 优先级标签
                    Rectangle {
                        Layout.preferredWidth: 36
                        Layout.preferredHeight: 16
                        radius: Tokens.radiusXS
                        color: Qt.rgba(root.priorityColor(priority).r, root.priorityColor(priority).g, root.priorityColor(priority).b, 0.15)

                        Text {
                            anchors.centerIn: parent
                            text: root.priorityLabel(priority)
                            color: root.priorityColor(priority)
                            font.family: Fonts.family
                            font.pixelSize: Fonts.xs
                            font.weight: priority <= 3 ? Font.Bold : Font.Normal
                        }
                    }

                    // 服务名
                    Text {
                        text: unit
                        color: Colors.subtext0
                        font.family: Fonts.family
                        font.pixelSize: Fonts.xs
                        Layout.preferredWidth: 80
                        elide: Text.ElideRight
                    }

                    // 消息
                    Text {
                        text: message
                        color: Colors.text
                        font.family: Fonts.family
                        font.pixelSize: Fonts.small
                        Layout.fillWidth: true
                        elide: Text.ElideRight
                        maximumLineCount: 2
                        wrapMode: Text.WordWrap
                    }
                }

                Behavior on color {
                    ColorAnimation {
                        duration: Tokens.animFast
                    }
                }
            }
        }
    }
}
