import "../../theme"
import "../components"
import QtQuick
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

    readonly property real compactWidth: batteryIconText.implicitWidth + 5 + percentageText.implicitWidth + 32
    readonly property real detailsWidth: statusLabel.implicitWidth + 16
        + (remainingTime.text !== "" ? remainingTime.implicitWidth + 6 : 0)
    implicitWidth: compactWidth + (hovered ? detailsWidth : 0)
    // 状态底色
    tintColor: {
        if (charging || full)
            return Colors.withAlpha(Colors.teal, 0.08);
        if (pct <= 10)
            return Colors.withAlpha(Colors.red, 0.18);
        if (pct <= 30)
            return Colors.withAlpha(Colors.yellow, 0.12);
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

    // 图标和百分比始终保留；状态与剩余时间随 hover 展开。
    Row {
        anchors.centerIn: parent
        spacing: 0

        Text {
            id: batteryIconText
            text: root.batteryIcon()
            color: root.accentColor
            font.family: Fonts.family
            font.pixelSize: Fonts.icon
            font.weight: Font.DemiBold
            anchors.verticalCenter: parent.verticalCenter
        }

        Item {
            width: 5
            height: 1
        }

        Text {
            id: percentageText
            text: root.pct + "%"
            color: root.hovered ? root.accentColor
                : (root.pct <= 10 ? Colors.red : (root.pct <= 30 ? Colors.peach : Colors.text))
            font.family: Fonts.family
            font.pixelSize: Fonts.bodyLarge
            font.weight: Font.DemiBold
            anchors.verticalCenter: parent.verticalCenter

            Behavior on color {
                ColorAnimation { duration: 300 }
            }
        }

        Item {
            width: 6 * root.hoverReveal
            height: 1
        }

        Rectangle {
            visible: root.hoverDetailsVisible
            color: Colors.withAlpha(root.accentColor, 0.2)
            radius: 4
            width: (statusLabel.implicitWidth + 10) * root.hoverReveal
            height: statusLabel.implicitHeight + 4
            opacity: root.hoverReveal
            clip: true
            anchors.verticalCenter: parent.verticalCenter

            Text {
                id: statusLabel
                anchors.centerIn: parent
                text: root.statusText()
                color: root.accentColor
                font.family: Fonts.family
                font.pixelSize: Fonts.caption
                font.weight: Font.DemiBold
            }
        }

        Item {
            width: remainingTime.text !== "" ? 6 * root.hoverReveal : 0
            height: 1
        }

        Text {
            id: remainingTime
            visible: root.hoverDetailsVisible && text !== ""
            text: root.timeRemaining()
            width: implicitWidth * root.hoverReveal
            opacity: root.hoverReveal
            clip: true
            color: Colors.subtext0
            font.family: Fonts.family
            font.pixelSize: Fonts.caption
            font.weight: Font.Normal
            anchors.verticalCenter: parent.verticalCenter
        }
    }
}
