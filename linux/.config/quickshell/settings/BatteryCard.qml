import "../theme"
import QtQuick
import QtQuick.Layouts
import Quickshell.Services.UPower

// 电池详���卡片 — 仅笔记本可见
InfoCard {
    id: root

    property var dev: UPower.displayDevice
    property int pct: dev ? Math.round(dev.percentage * 100) : 0
    property bool charging: dev ? dev.state === UPowerDeviceState.Charging : false
    property bool full: dev ? dev.state === UPowerDeviceState.FullyCharged : false

    function batteryIcon() {
        if (full)
            return "✔";

        if (charging)
            return "⚡";

        if (pct >= 90)
            return "";

        if (pct >= 60)
            return "";

        if (pct >= 40)
            return "";

        if (pct >= 20)
            return "";

        return "";
    }

    function statusText() {
        if (full)
            return "已充满";

        if (charging)
            return "充电中";

        return "放电中";
    }

    function statusColor() {
        if (charging || full)
            return Colors.green;

        if (pct <= 10)
            return Colors.red;

        if (pct <= 30)
            return Colors.peach;

        return Colors.text;
    }

    function formatTime(secs) {
        if (!secs || secs <= 0)
            return "";

        let h = Math.floor(secs / 3600);
        let m = Math.floor((secs % 3600) / 60);
        if (h > 0)
            return h + " 小时 " + m + " 分钟";

        return m + " 分钟";
    }

    function timeRemaining() {
        if (full)
            return "";

        if (charging && dev.timeToFull > 0)
            return "充满需 " + formatTime(dev.timeToFull);

        if (!charging && dev.timeToEmpty > 0)
            return "剩余 " + formatTime(dev.timeToEmpty);

        return "";
    }

    visible: dev !== null

    contentItem: ColumnLayout {
        spacing: 6

        RowLayout {
            Layout.fillWidth: true
            spacing: 8

            Text {
                text: batteryIcon()
                color: statusColor()
                font.family: Fonts.family
                font.pixelSize: Fonts.heading
            }

            Text {
                text: root.pct + "%"
                color: statusColor()
                font.family: Fonts.family
                font.pixelSize: Fonts.icon
                font.weight: Font.Bold
            }

            Item {
                Layout.fillWidth: true
            }

            Text {
                text: statusText()
                color: Colors.subtext0
                font.family: Fonts.family
                font.pixelSize: Fonts.small
            }

        }

        // 进度条
        Rectangle {
            Layout.fillWidth: true
            height: 6
            radius: 3
            color: Colors.surface1

            Rectangle {
                width: parent.width * root.pct / 100
                height: parent.height
                radius: 3
                color: statusColor()

                Behavior on width {
                    NumberAnimation {
                        duration: 300
                        easing.type: Easing.OutCubic
                    }

                }

            }

        }

        // 剩余时间
        Text {
            visible: text.length > 0
            text: timeRemaining()
            color: Colors.overlay1
            font.family: Fonts.family
            font.pixelSize: Fonts.caption
        }

    }

}
