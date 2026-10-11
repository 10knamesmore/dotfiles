pragma ComponentBehavior: Bound

import "../../theme"
import "../../state"
import "../../state/StatsFormat.js" as StatsFormat
import "../components"
import QtQuick

// 悬停补充 used/total；移入面板后仍保持一秒刷新。
BarModule {
    id: root

    readonly property var stats: SystemStats.memory
    readonly property int usage: stats.totalBytes > 0 ? Math.round(stats.usedBytes / stats.totalBytes * 100) : 0

    Accessible.checkable: true
    Accessible.checked: PanelState.memoryOpen
    Accessible.description: "使用率 " + usage + "%；" + memoryDetails.text
    Accessible.name: "内存"
    Accessible.role: Accessible.Button
    accentColor: Colors.teal
    activeFocusOnTab: true
    implicitWidth: summary.implicitWidth + horizontalPadding * 2 + (hovered ? detailsSize.width + 5 : 0)

    Accessible.onPressAction: PanelState.toggleResourcePanel("memory", root)
    Keys.onReturnPressed: PanelState.toggleResourcePanel("memory", root)
    Keys.onSpacePressed: PanelState.toggleResourcePanel("memory", root)
    onClicked: PanelState.toggleResourcePanel("memory", root)

    TextMetrics {
        id: usageSize
        font: usageText.font
        text: "100%"
    }
    TextMetrics {
        id: detailsSize
        font: memoryDetails.font
        text: StatsFormat.gib(root.stats.totalBytes) + " / " + StatsFormat.gib(root.stats.totalBytes) + " GiB"
    }

    Row {
        id: summary

        anchors.left: parent.left
        anchors.verticalCenter: parent.verticalCenter
        spacing: 8

        Item {
            Accessible.ignored: true
            anchors.verticalCenter: parent.verticalCenter
            height: 18
            width: 29

            Rectangle {
                id: memoryBody

                anchors.top: parent.top
                border.color: root.accentColor
                border.width: 1.5
                color: "transparent"
                height: 14
                radius: 2
                width: parent.width

                Rectangle {
                    anchors.left: parent.left
                    anchors.leftMargin: 3
                    anchors.verticalCenter: parent.verticalCenter
                    color: root.accentColor
                    height: parent.height - 6
                    width: (parent.width - 6) * root.usage / 100

                    Behavior on width {
                        NumberAnimation {
                            duration: Tokens.animNormal
                        }
                    }
                }
            }
            Repeater {
                model: 6

                delegate: Rectangle {
                    required property int index

                    anchors.top: memoryBody.bottom
                    anchors.topMargin: 1
                    color: root.accentColor
                    height: 2
                    width: 2
                    x: 3 + index * 4
                }
            }
        }
        Column {
            anchors.verticalCenter: parent.verticalCenter
            spacing: 1
            width: usageSize.width

            Text {
                Accessible.ignored: true
                color: root.accentColor
                font.family: Fonts.family
                font.pixelSize: Fonts.xs
                font.weight: Font.DemiBold
                text: "RAM"
            }
            Text {
                id: usageText

                Accessible.ignored: true
                color: root.usage > 85 ? Colors.red : root.usage > 60 ? Colors.yellow : Colors.text
                font.family: Fonts.family
                font.pixelSize: Fonts.small
                font.weight: Font.DemiBold
                text: root.usage + "%"
                width: parent.width

                Behavior on color {
                    ColorAnimation {
                        duration: Tokens.animNormal
                    }
                }
            }
        }
    }
    Text {
        id: memoryDetails

        anchors.left: summary.right
        anchors.leftMargin: 5
        anchors.verticalCenter: parent.verticalCenter
        clip: true
        color: Colors.subtext0
        font.family: Fonts.family
        font.pixelSize: Fonts.caption
        opacity: root.detailProgress
        text: StatsFormat.gib(root.stats.usedBytes) + " / " + StatsFormat.gib(root.stats.totalBytes) + " GiB"
        visible: root.detailProgress > 0
        width: detailsSize.width * root.detailProgress
    }
}
