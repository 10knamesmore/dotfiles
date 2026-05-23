import "../../theme"
import "../components"
import QtQuick

BarModule {
    id: root

    property string direction: "up" // "up" 或 "down"

    // 数据来自 SystemStats（SystemStatsService 每秒更新；up/down 共用一次 /proc/net/dev 读取）
    readonly property real speed: direction === "up" ? SystemStats.netUpSpeed : SystemStats.netDownSpeed
    readonly property real totalBytes: direction === "up" ? SystemStats.netUpTotal : SystemStats.netDownTotal
    readonly property string ifaceName: SystemStats.netIface

    function formatSpeed(bytesPerSec) {
        if (bytesPerSec < 1024)
            return bytesPerSec.toFixed(0) + " B/s";
        if (bytesPerSec < 1024 * 1024)
            return (bytesPerSec / 1024).toFixed(1) + " KB/s";
        return (bytesPerSec / (1024 * 1024)).toFixed(2) + " MB/s";
    }

    function formatTotal(bytes) {
        if (bytes < 1024)
            return bytes.toFixed(0) + " B";
        if (bytes < 1024 * 1024)
            return (bytes / 1024).toFixed(1) + " KB";
        if (bytes < 1024 * 1024 * 1024)
            return (bytes / (1024 * 1024)).toFixed(1) + " MB";
        return (bytes / (1024 * 1024 * 1024)).toFixed(2) + " GB";
    }

    property string displayText: (direction === "up" ? "󰕒" : "󰁅") + " " + formatSpeed(speed)

    accentColor: Colors.teal
    implicitWidth: root.hovered
        ? Math.max(hoverRow.implicitWidth + 32, 180)
        : Math.max(label.implicitWidth + 32, 120)

    // ── 默认视图 ──
    Text {
        id: label

        visible: !root.hovered
        anchors.centerIn: parent
        text: root.displayText
        color: Colors.teal
        font.family: Fonts.family
        font.pixelSize: Fonts.bodyLarge
        font.weight: Font.DemiBold
    }

    // ── hover 视图：接口名 + 速度 + 累计流量 ──
    Row {
        id: hoverRow

        visible: root.hovered
        anchors.centerIn: parent
        spacing: 6

        // 接口名标签
        Rectangle {
            visible: root.ifaceName !== ""
            color: Qt.rgba(Colors.teal.r, Colors.teal.g, Colors.teal.b, 0.2)
            radius: 4
            width: ifaceText.implicitWidth + 10
            height: ifaceText.implicitHeight + 4
            anchors.verticalCenter: parent.verticalCenter

            Text {
                id: ifaceText

                anchors.centerIn: parent
                text: root.ifaceName
                color: Colors.teal
                font.family: Fonts.family
                font.pixelSize: Fonts.caption
                font.weight: Font.DemiBold
            }
        }

        // 方向图标 + 速度
        Text {
            text: root.displayText
            color: Colors.teal
            font.family: Fonts.family
            font.pixelSize: Fonts.bodyLarge
            font.weight: Font.DemiBold
            anchors.verticalCenter: parent.verticalCenter
        }

        // 累计流量
        Text {
            text: "Σ " + root.formatTotal(root.totalBytes)
            color: Colors.subtext0
            font.family: Fonts.family
            font.pixelSize: Fonts.caption
            font.weight: Font.Normal
            anchors.verticalCenter: parent.verticalCenter
        }
    }
}
