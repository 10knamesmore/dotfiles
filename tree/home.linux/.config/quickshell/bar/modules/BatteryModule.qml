import "../../theme"
import "../components"
import QtQuick
import QtQuick.Layouts
import Quickshell.Services.UPower

BarModule {
    id: root

    property var dev: UPower.displayDevice
    property int pct: dev ? Math.round(dev.percentage * 100) : 0
    property bool charging: dev ? dev.state === UPowerDeviceState.Charging : false
    property bool full: dev ? dev.state === UPowerDeviceState.FullyCharged : false

    // Waybar: format-charging "⚡", format-full "✔", format-icons 5 levels
    function batteryIcon() {
        if (full)
            return "✔";

        if (charging)
            return "⚡";

        if (pct >= 90)
            return "";

        if (pct >= 60)
            return "";

        if (pct >= 40)
            return "";

        if (pct >= 20)
            return "";

        return "";
    }

    function statusText() {
        if (full)
            return "已充满";
        if (charging)
            return "充电中";
        return "放电中";
    }

    function formatTime(secs) {
        if (!secs || secs <= 0)
            return "";
        let h = Math.floor(secs / 3600);
        let m = Math.floor((secs % 3600) / 60);
        if (h > 0)
            return h + "h " + m + "m";
        return m + "m";
    }

    function timeRemaining() {
        if (full)
            return "";
        if (charging && dev.timeToFull > 0)
            return formatTime(dev.timeToFull);
        if (!charging && dev.timeToEmpty > 0)
            return formatTime(dev.timeToEmpty);
        return "";
    }

    implicitWidth: root.hovered ? Math.max(hoverRow.implicitWidth + 32, label.implicitWidth + 32) : label.implicitWidth + 32
    // 状态底色
    tintColor: {
        if (charging || full)
            return Qt.rgba(0.545, 0.835, 0.792, 0.08); // teal 8%
        if (pct <= 10)
            return Qt.rgba(0.929, 0.529, 0.588, 0.18); // red 18%
        if (pct <= 30)
            return Qt.rgba(0.933, 0.831, 0.624, 0.12); // yellow 12%
        return "transparent";
    }
    // 根据电量/状态动态调整颜色
    accentColor: {
        if (charging || full)
            return Colors.green;
        if (pct <= 10)
            return Colors.red;
        if (pct <= 30)
            return Colors.peach;
        return Colors.green;
    }

    // ── 默认视图 ──
    Row {
        id: label

        visible: !root.hovered
        anchors.centerIn: parent
        spacing: 5

        Text {
            text: root.batteryIcon()
            color: root.accentColor
            font.family: Fonts.family
            font.pixelSize: Fonts.icon
            font.weight: Font.DemiBold
            anchors.verticalCenter: parent.verticalCenter
        }

        Text {
            text: root.pct + "%"
            color: root.pct <= 10 ? Colors.red : (root.pct <= 30 ? Colors.peach : Colors.text)
            font.family: Fonts.family
            font.pixelSize: Fonts.bodyLarge
            font.weight: Font.DemiBold
            anchors.verticalCenter: parent.verticalCenter

            Behavior on color {
                ColorAnimation {
                    duration: 300
                }
            }
        }
    }

    // ── hover 视图：图标 + 百分比 + 状态 + 剩余时间 ──
    Row {
        id: hoverRow

        visible: root.hovered
        anchors.centerIn: parent
        spacing: 6

        Text {
            text: root.batteryIcon()
            color: root.accentColor
            font.family: Fonts.family
            font.pixelSize: Fonts.icon
            font.weight: Font.DemiBold
            anchors.verticalCenter: parent.verticalCenter
        }

        Text {
            text: root.pct + "%"
            color: root.accentColor
            font.family: Fonts.family
            font.pixelSize: Fonts.bodyLarge
            font.weight: Font.DemiBold
            anchors.verticalCenter: parent.verticalCenter
        }

        // 状态标签
        Rectangle {
            color: Qt.rgba(root.accentColor.r, root.accentColor.g, root.accentColor.b, 0.2)
            radius: 4
            width: statusText.implicitWidth + 10
            height: statusText.implicitHeight + 4
            anchors.verticalCenter: parent.verticalCenter

            Text {
                id: statusText

                anchors.centerIn: parent
                text: root.statusText()
                color: root.accentColor
                font.family: Fonts.family
                font.pixelSize: Fonts.caption
                font.weight: Font.DemiBold
            }
        }

        // 剩余时间
        Text {
            visible: root.timeRemaining() !== ""
            text: root.timeRemaining()
            color: Colors.subtext0
            font.family: Fonts.family
            font.pixelSize: Fonts.caption
            font.weight: Font.Normal
            anchors.verticalCenter: parent.verticalCenter
        }
    }
}
