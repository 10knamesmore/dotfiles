import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Wayland
import "../theme"

// 月历面板 — 点击时钟弹出，显示当月日历
PanelWindow {
    id: root

    anchors.top: true
    anchors.bottom: true
    anchors.left: true
    anchors.right: true

    // 双阶段可见性：动画结束后才隐藏 PanelWindow
    property bool showing: PanelState.calendarOpen
    property bool animating: _opacityAnim.running || _slideAnim.running
    visible: showing || animating

    focusable: false
    exclusionMode: ExclusionMode.Ignore
    color: "transparent"

    property date currentDate: new Date()
    property int viewYear: currentDate.getFullYear()
    property int viewMonth: currentDate.getMonth() // 0-based

    onShowingChanged: {
        if (showing) {
            currentDate = new Date()
            viewYear  = currentDate.getFullYear()
            viewMonth = currentDate.getMonth()
        }
    }

    // 半透明遮罩
    Rectangle {
        anchors.fill: parent
        color: "#000000"
        opacity: root.showing ? 0.15 : 0.0
        Behavior on opacity { NumberAnimation { duration: 250; easing.type: Easing.OutCubic } }
    }

    // 点击面板外部关闭
    MouseArea {
        anchors.fill: parent
        onClicked: PanelState.calendarOpen = false
    }

    Rectangle {
        id: panel
        width: 300
        height: col.implicitHeight + 32
        radius: 16
        color: Qt.rgba(Colors.surface0.r, Colors.surface0.g, Colors.surface0.b, 0.85)
        border.color: Qt.rgba(Colors.surface1.r, Colors.surface1.g, Colors.surface1.b, 0.9)
        border.width: 1
        anchors.top: parent.top
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.topMargin: root.showing ? 54 : 34

        opacity: root.showing ? 1.0 : 0.0

        Behavior on anchors.topMargin {
            NumberAnimation { id: _slideAnim; duration: 250; easing.type: Easing.OutCubic }
        }
        Behavior on opacity {
            NumberAnimation { id: _opacityAnim; duration: 250; easing.type: Easing.OutCubic }
        }

        MouseArea {
            anchors.fill: parent
            onClicked: mouse => mouse.accepted = true
        }

        ColumnLayout {
            id: col
            anchors.fill: parent
            anchors.margins: 16
            spacing: 8

            // ── 月份导航 ──
            RowLayout {
                Layout.fillWidth: true

                CalNavButton {
                    text: "󰅁"
                    onClicked: {
                        if (root.viewMonth === 0) {
                            root.viewMonth = 11
                            root.viewYear--
                        } else {
                            root.viewMonth--
                        }
                    }
                }

                Item { Layout.fillWidth: true }

                Text {
                    text: monthName(root.viewMonth) + " " + root.viewYear
                    color: Colors.text
                    font.family: "Hack Nerd Font"
                    font.pixelSize: 14
                    font.weight: Font.Bold
                }

                Item { Layout.fillWidth: true }

                CalNavButton {
                    text: "󰅂"
                    onClicked: {
                        if (root.viewMonth === 11) {
                            root.viewMonth = 0
                            root.viewYear++
                        } else {
                            root.viewMonth++
                        }
                    }
                }
            }

            Rectangle {
                Layout.fillWidth: true
                height: 1
                color: Colors.surface1
            }

            // ── 星期头 ──
            RowLayout {
                Layout.fillWidth: true
                spacing: 0

                Repeater {
                    model: ["一", "二", "三", "四", "五", "六", "日"]
                    delegate: Text {
                        Layout.fillWidth: true
                        text: modelData
                        color: Colors.subtext0
                        font.family: "Hack Nerd Font"
                        font.pixelSize: 11
                        horizontalAlignment: Text.AlignHCenter
                    }
                }
            }

            // ── 日期网格 ──
            GridLayout {
                Layout.fillWidth: true
                columns: 7
                rowSpacing: 2
                columnSpacing: 0

                Repeater {
                    model: root.generateDays()

                    delegate: Rectangle {
                        Layout.fillWidth: true
                        Layout.preferredHeight: 32
                        radius: 16
                        color: {
                            if (modelData.isToday) return Colors.mauve
                            if (modelData.hovered) return Colors.surface1
                            return "transparent"
                        }

                        property bool hovered: dayArea.containsMouse

                        Behavior on color { ColorAnimation { duration: 150 } }

                        Text {
                            anchors.centerIn: parent
                            text: modelData.day > 0 ? modelData.day : ""
                            color: {
                                if (modelData.isToday) return Colors.base
                                if (!modelData.inMonth) return Colors.overlay0
                                return Colors.text
                            }
                            font.family: "Hack Nerd Font"
                            font.pixelSize: 12
                            font.weight: modelData.isToday ? Font.Bold : Font.Normal
                        }

                        MouseArea {
                            id: dayArea
                            anchors.fill: parent
                            hoverEnabled: true
                        }
                    }
                }
            }

            // ── 今天按钮 ──
            Rectangle {
                Layout.alignment: Qt.AlignHCenter
                width: todayText.implicitWidth + 20
                height: 26
                radius: 13
                color: todayBtnArea.containsMouse ? Colors.surface1 : "transparent"

                Behavior on color { ColorAnimation { duration: 150 } }

                Text {
                    id: todayText
                    anchors.centerIn: parent
                    text: "今天"
                    color: Colors.mauve
                    font.family: "Hack Nerd Font"
                    font.pixelSize: 11
                    font.weight: Font.DemiBold
                }

                MouseArea {
                    id: todayBtnArea
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: {
                        root.currentDate = new Date()
                        root.viewYear  = root.currentDate.getFullYear()
                        root.viewMonth = root.currentDate.getMonth()
                    }
                }
            }
        }
    }

    // ── 辅助函数 ──

    function monthName(m) {
        return ["一月","二月","三月","四月","五月","六月",
                "七月","八月","九月","十月","十一月","十二月"][m]
    }

    function generateDays() {
        let days = []
        let first = new Date(viewYear, viewMonth, 1)
        // JS: Sunday=0, we want Monday=0
        let startDow = (first.getDay() + 6) % 7
        let daysInMonth = new Date(viewYear, viewMonth + 1, 0).getDate()
        let prevMonthDays = new Date(viewYear, viewMonth, 0).getDate()

        let today = new Date()
        let todayY = today.getFullYear()
        let todayM = today.getMonth()
        let todayD = today.getDate()

        // 上月尾部
        for (let i = startDow - 1; i >= 0; i--) {
            days.push({ day: prevMonthDays - i, inMonth: false, isToday: false })
        }

        // 当月
        for (let d = 1; d <= daysInMonth; d++) {
            days.push({
                day: d,
                inMonth: true,
                isToday: (viewYear === todayY && viewMonth === todayM && d === todayD)
            })
        }

        // 下月头部（填满到 6 行 × 7 = 42 格）
        let remaining = 42 - days.length
        for (let i = 1; i <= remaining; i++) {
            days.push({ day: i, inMonth: false, isToday: false })
        }

        return days
    }

    // 可复用导航按钮
    component CalNavButton: Rectangle {
        property string text: ""
        signal clicked()

        width: 28
        height: 28
        radius: 14
        color: navArea.containsMouse ? Colors.surface1 : "transparent"

        Behavior on color { ColorAnimation { duration: 150 } }

        Text {
            anchors.centerIn: parent
            text: parent.text
            color: Colors.subtext0
            font.family: "Hack Nerd Font"
            font.pixelSize: 14
        }

        MouseArea {
            id: navArea
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: parent.clicked()
        }
    }
}
