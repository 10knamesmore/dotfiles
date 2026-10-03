pragma ComponentBehavior: Bound

import "../../theme"
import "../../state"
import "../../state/StatsFormat.js" as StatsFormat
import "../components"
import QtQuick

// 下载、上传纵向排列，hover 时各行补充累计量；顶栏和面板头部都按一秒更新。
BarModule {
    id: root

    readonly property real rateWidth: Math.max(rateSize.width, download.rateTextWidth, upload.rateTextWidth)
    readonly property var stats: SystemStats.network
    readonly property real totalWidth: Math.max(totalSize.width, download.totalTextWidth, upload.totalTextWidth)

    Accessible.checkable: true
    Accessible.checked: PanelState.networkStatsOpen
    Accessible.description: "下载 " + StatsFormat.speed(stats.downSpeed) + "，上传 " + StatsFormat.speed(stats.upSpeed)
    Accessible.name: "网络速率"
    Accessible.role: Accessible.Button
    accentColor: Colors.teal
    activeFocusOnTab: true
    implicitWidth: download.compactWidth + horizontalPadding * 2 + (hovered ? totalWidth + 6 : 0)

    Accessible.onPressAction: PanelState.toggleResourcePanel("network", root)
    Keys.onReturnPressed: PanelState.toggleResourcePanel("network", root)
    Keys.onSpacePressed: PanelState.toggleResourcePanel("network", root)
    onClicked: PanelState.toggleResourcePanel("network", root)

    // 按常用短值留少量余量，更长的数值才按实际文本扩宽。
    TextMetrics {
        id: rateSize

        font.family: Fonts.family
        font.pixelSize: Fonts.small
        font.weight: Font.DemiBold
        text: "9.9 KiB/s"
    }
    TextMetrics {
        id: totalSize

        font.family: Fonts.family
        font.pixelSize: Fonts.caption
        text: "Σ 9.99 GiB"
    }
    Column {
        anchors.left: parent.left
        anchors.verticalCenter: parent.verticalCenter

        TrafficDirection {
            id: download

            downloading: true
        }
        TrafficDirection {
            id: upload

            downloading: false
        }
    }

    component TrafficDirection: Row {
        id: direction

        readonly property real compactWidth: arrow.implicitWidth + 5 + root.rateWidth
        required property bool downloading
        readonly property real rateTextWidth: rate.implicitWidth
        readonly property real totalTextWidth: total.implicitWidth

        spacing: 0

        Text {
            id: arrow

            anchors.verticalCenter: parent.verticalCenter
            color: direction.downloading ? Colors.teal : Colors.blue
            font: rate.font
            text: direction.downloading ? "󰁅" : "󰕒"
        }
        Item {
            height: 1
            width: 5
        }
        Text {
            id: rate

            anchors.verticalCenter: parent.verticalCenter
            color: direction.downloading ? Colors.teal : Colors.blue
            font: rateSize.font
            horizontalAlignment: Text.AlignRight
            text: StatsFormat.speed(direction.downloading ? root.stats.downSpeed : root.stats.upSpeed)
            width: root.rateWidth
        }
        Item {
            height: 1
            width: 6 * root.detailProgress
        }
        Text {
            id: total

            anchors.verticalCenter: parent.verticalCenter
            clip: true
            color: Colors.subtext0
            font: totalSize.font
            opacity: root.detailProgress
            text: "Σ " + StatsFormat.bytes(direction.downloading ? root.stats.downTotal : root.stats.upTotal)
            visible: root.detailProgress > 0
            width: root.totalWidth * root.detailProgress
        }
    }
}
