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
    Accessible.description: memoryDetails.text
    Accessible.name: "内存"
    Accessible.role: Accessible.Button
    accentColor: Colors.mauve
    activeFocusOnTab: true
    implicitWidth: summary.implicitWidth + horizontalPadding * 2 + (hovered ? memoryDetails.implicitWidth + 5 : 0)

    Accessible.onPressAction: PanelState.toggleResourcePanel("memory", root)
    Keys.onReturnPressed: PanelState.toggleResourcePanel("memory", root)
    Keys.onSpacePressed: PanelState.toggleResourcePanel("memory", root)
    onClicked: PanelState.toggleResourcePanel("memory", root)

    Row {
        id: summary

        anchors.left: parent.left
        anchors.verticalCenter: parent.verticalCenter
        spacing: 5

        Text {
            id: memoryIcon

            anchors.verticalCenter: parent.verticalCenter
            color: Colors.mauve
            font.family: Fonts.family
            font.pixelSize: Fonts.icon
            font.weight: Font.DemiBold
            text: "󰍛"
        }
        Text {
            id: usageText

            anchors.verticalCenter: parent.verticalCenter
            color: root.usage > 85 ? Colors.red : root.usage > 60 ? Colors.yellow : Colors.text
            font.family: Fonts.family
            font.pixelSize: Fonts.bodyLarge
            font.weight: Font.DemiBold
            text: root.usage + "%"

            Behavior on color {
                ColorAnimation {
                    duration: Tokens.animNormal
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
        width: implicitWidth * root.detailProgress
    }
}
