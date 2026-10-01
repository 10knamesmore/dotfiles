import "../../theme"
import "../../state"
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

    readonly property real expandedContentWidth: speedLabel.implicitWidth + totalLabel.implicitWidth + 6
        + (ifaceName !== "" ? ifaceText.implicitWidth + 16 : 0)
    accentColor: Colors.teal
    implicitWidth: root.hovered
        ? Math.max(expandedContentWidth + 32, 180)
        : Math.max(speedLabel.implicitWidth + 32, 120)
    clickable: false

    // 方向与网速始终使用同一组件；只有两侧的补充信息渐变显隐。
    Row {
        anchors.centerIn: parent
        spacing: 0

        // 接口名标签
        Rectangle {
            visible: root.ifaceName !== "" && root.hoverDetailsVisible
            color: Colors.withAlpha(Colors.teal, 0.2)
            radius: 4
            width: (ifaceText.implicitWidth + 10) * root.hoverReveal
            height: ifaceText.implicitHeight + 4
            opacity: root.hoverReveal
            clip: true
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

        Item {
            width: root.ifaceName !== "" ? 6 * root.hoverReveal : 0
            height: 1
        }

        // 方向图标 + 速度
        Text {
            id: speedLabel
            text: root.displayText
            color: Colors.teal
            font.family: Fonts.family
            font.pixelSize: Fonts.bodyLarge
            font.weight: Font.DemiBold
            anchors.verticalCenter: parent.verticalCenter
        }

        Item {
            width: 6 * root.hoverReveal
            height: 1
        }

        // 累计流量
        Text {
            id: totalLabel
            visible: root.hoverDetailsVisible
            width: implicitWidth * root.hoverReveal
            opacity: root.hoverReveal
            clip: true
            text: "Σ " + root.formatTotal(root.totalBytes)
            color: Colors.subtext0
            font.family: Fonts.family
            font.pixelSize: Fonts.caption
            font.weight: Font.Normal
            anchors.verticalCenter: parent.verticalCenter
        }
    }
}
