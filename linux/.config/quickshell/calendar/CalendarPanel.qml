import "../components"
import "../theme"
import "Lunar.js" as Lunar
import QtQuick
import QtQuick.Layouts

// 月历面板 — 点击时钟弹出，显示当月日历 + 农历
PanelOverlay {
    id: root

    property date currentDate: new Date()
    property int viewYear: currentDate.getFullYear()
    property int viewMonth: currentDate.getMonth() // 0-based

    function monthName(m) {
        return ["一月", "二月", "三月", "四月", "五月", "六月", "七月", "八月", "九月", "十月", "十一月", "十二月"][m];
    }

    function generateDays() {
        let days = [];
        let first = new Date(viewYear, viewMonth, 1);
        // JS: Sunday=0, we want Monday=0
        let startDow = (first.getDay() + 6) % 7;
        let daysInMonth = new Date(viewYear, viewMonth + 1, 0).getDate();
        let prevMonthDays = new Date(viewYear, viewMonth, 0).getDate();
        let today = new Date();
        let todayY = today.getFullYear();
        let todayM = today.getMonth();
        let todayD = today.getDate();
        // 上月尾部
        let prevY = viewMonth === 0 ? viewYear - 1 : viewYear;
        let prevM = viewMonth === 0 ? 12 : viewMonth; // 1-based for Lunar
        for (let i = startDow - 1; i >= 0; i--) {
            let d = prevMonthDays - i;
            let lunar = Lunar.toLunar(prevY, prevM, d);
            days.push({
                "day": d,
                "inMonth": false,
                "isToday": false,
                "lunar": lunar.display,
                "isFestival": lunar.isFestival
            });
        }
        // 当月
        let curM = viewMonth + 1; // 1-based
        for (let d = 1; d <= daysInMonth; d++) {
            let lunar = Lunar.toLunar(viewYear, curM, d);
            days.push({
                "day": d,
                "inMonth": true,
                "isToday": (viewYear === todayY && viewMonth === todayM && d === todayD),
                "lunar": lunar.display,
                "isFestival": lunar.isFestival
            });
        }
        // 下月头部（填满到 6 行 × 7 = 42 格）
        let nextY = viewMonth === 11 ? viewYear + 1 : viewYear;
        let nextM = viewMonth === 11 ? 1 : viewMonth + 2; // 1-based
        let remaining = 42 - days.length;
        for (let i = 1; i <= remaining; i++) {
            let lunar = Lunar.toLunar(nextY, nextM, i);
            days.push({
                "day": i,
                "inMonth": false,
                "isToday": false,
                "lunar": lunar.display,
                "isFestival": lunar.isFestival
            });
        }
        return days;
    }

    showing: PanelState.calendarOpen
    panelWidth: 320
    panelHeight: col.implicitHeight + 32
    panelTargetY: 54
    closedOffsetY: -20
    onCloseRequested: PanelState.calendarOpen = false

    onShowingChanged: {
        if (showing) {
            currentDate = new Date();
            viewYear = currentDate.getFullYear();
            viewMonth = currentDate.getMonth();
        }
    }

    ColumnLayout {
        id: col

        anchors.fill: parent
        anchors.margins: Tokens.spaceL
        spacing: Tokens.spaceS

        // ── 月份导航 ──
        RowLayout {
            Layout.fillWidth: true

            CalNavButton {
                text: "󰅁"
                onClicked: {
                    if (root.viewMonth === 0) {
                        root.viewMonth = 11;
                        root.viewYear--;
                    } else {
                        root.viewMonth--;
                    }
                }
            }

            Item {
                Layout.fillWidth: true
            }

            Text {
                text: monthName(root.viewMonth) + " " + root.viewYear
                color: Colors.text
                font.family: Fonts.family
                font.pixelSize: Fonts.icon
                font.weight: Font.Bold
            }

            Item {
                Layout.fillWidth: true
            }

            CalNavButton {
                text: "󰅂"
                onClicked: {
                    if (root.viewMonth === 11) {
                        root.viewMonth = 0;
                        root.viewYear++;
                    } else {
                        root.viewMonth++;
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
                    font.family: Fonts.family
                    font.pixelSize: Fonts.small
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
                    property bool hovered: dayArea.containsMouse

                    Layout.fillWidth: true
                    Layout.preferredHeight: 42
                    radius: Tokens.radiusL
                    color: {
                        if (modelData.isToday)
                            return Colors.mauve;

                        if (modelData.hovered)
                            return Colors.surface1;

                        return "transparent";
                    }

                    Column {
                        anchors.centerIn: parent
                        spacing: 1

                        Text {
                            anchors.horizontalCenter: parent.horizontalCenter
                            text: modelData.day > 0 ? modelData.day : ""
                            color: {
                                if (modelData.isToday)
                                    return Colors.base;

                                if (!modelData.inMonth)
                                    return Colors.overlay0;

                                return Colors.text;
                            }
                            font.family: Fonts.family
                            font.pixelSize: Fonts.body
                            font.weight: modelData.isToday ? Font.Bold : Font.Normal
                        }

                        Text {
                            anchors.horizontalCenter: parent.horizontalCenter
                            text: modelData.lunar || ""
                            color: {
                                if (modelData.isToday)
                                    return Qt.rgba(Colors.base.r, Colors.base.g, Colors.base.b, 0.8);

                                if (modelData.isFestival)
                                    return Colors.peach;

                                if (!modelData.inMonth)
                                    return Qt.rgba(Colors.overlay0.r, Colors.overlay0.g, Colors.overlay0.b, 0.6);

                                return Colors.overlay0;
                            }
                            font.family: Fonts.family
                            font.pixelSize: Fonts.xs
                        }
                    }

                    MouseArea {
                        id: dayArea

                        anchors.fill: parent
                        hoverEnabled: true
                    }

                    Behavior on color {
                        ColorAnimation {
                            duration: 150
                        }

                    }

                }

            }

        }

        // ── 今天按钮 ──
        Rectangle {
            Layout.alignment: Qt.AlignHCenter
            width: todayText.implicitWidth + 20
            height: 26
            radius: Tokens.radiusFull
            color: todayBtnArea.containsMouse ? Colors.surface1 : "transparent"

            Text {
                id: todayText

                anchors.centerIn: parent
                text: "今天"
                color: Colors.mauve
                font.family: Fonts.family
                font.pixelSize: Fonts.small
                font.weight: Font.DemiBold
            }

            MouseArea {
                id: todayBtnArea

                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: {
                    root.currentDate = new Date();
                    root.viewYear = root.currentDate.getFullYear();
                    root.viewMonth = root.currentDate.getMonth();
                }
            }

            Behavior on color {
                ColorAnimation {
                    duration: 150
                }

            }

        }

    }

    // 可复用导航按钮
    component CalNavButton: Rectangle {
        property string text: ""

        signal clicked()

        width: 28
        height: 28
        radius: Tokens.radiusFull
        color: navArea.containsMouse ? Colors.surface1 : "transparent"

        Text {
            anchors.centerIn: parent
            text: parent.text
            color: Colors.subtext0
            font.family: Fonts.family
            font.pixelSize: Fonts.icon
        }

        MouseArea {
            id: navArea

            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: parent.clicked()
        }

        Behavior on color {
            ColorAnimation {
                duration: 150
            }

        }

    }

}
