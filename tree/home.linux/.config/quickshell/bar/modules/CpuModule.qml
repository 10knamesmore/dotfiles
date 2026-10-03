pragma ComponentBehavior: Bound

import "../../theme"
import "../../state"
import "../components"
import QtQuick

// 头部始终读取一秒快照；逐核图在 hover 时预览，移入面板后铺满头部剩余宽度。
BarModule {
    id: root

    readonly property var barColors: ["#69ff94", "#2aa9ff", "#f8f8f2", "#f8f8f2", "#ffffa5", "#ffffa5", "#ff9977", "#dd532e"]
    // 预览目标宽度只随核心数变化，不依赖图形当前的显隐和动画宽度。
    readonly property real previewWidth: stats.cores.length * 8
    readonly property var stats: SystemStats.cpu
    readonly property int usage: Math.round(stats.usage)

    Accessible.checkable: true
    Accessible.checked: PanelState.cpuOpen
    Accessible.description: "使用率 " + usage + "%"
    Accessible.name: "CPU"
    Accessible.role: Accessible.Button
    accentColor: Colors.blue
    activeFocusOnTab: true
    implicitWidth: summary.implicitWidth + 30 + (hovered ? previewWidth + 5 : 0)

    Accessible.onPressAction: PanelState.toggleResourcePanel("cpu", root)
    Keys.onReturnPressed: PanelState.toggleResourcePanel("cpu", root)
    Keys.onSpacePressed: PanelState.toggleResourcePanel("cpu", root)
    onClicked: PanelState.toggleResourcePanel("cpu", root)

    Row {
        id: summary

        anchors.left: parent.left
        anchors.verticalCenter: parent.verticalCenter
        spacing: 5

        Text {
            anchors.verticalCenter: parent.verticalCenter
            color: Colors.blue
            font.family: Fonts.family
            font.pixelSize: Fonts.icon
            font.weight: Font.DemiBold
            text: ""
        }
        Text {
            anchors.verticalCenter: parent.verticalCenter
            color: root.usage > 80 ? Colors.red : root.usage > 50 ? Colors.yellow : Colors.text
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
    Item {
        id: coreChart

        Accessible.description: root.stats.cores.map((usage, index) => "核心 " + index + "，" + Math.round(usage) + "%").join("；")
        Accessible.name: "逐核 CPU 使用率"
        Accessible.role: Accessible.Graphic
        anchors.left: summary.right
        anchors.leftMargin: 5
        anchors.verticalCenter: parent.verticalCenter
        clip: true
        height: parent.height - Tokens.spaceS
        opacity: root.detailProgress
        visible: root.detailProgress > 0
        width: Math.max(0, parent.width - summary.width - 5)

        Repeater {
            model: root.stats.cores.length

            delegate: Rectangle {
                required property int index
                readonly property real usage: root.stats.cores[index]

                anchors.bottom: parent.bottom
                color: root.barColors[Math.min(Math.floor(usage / 12.5), 7)]
                height: Math.max(2, coreChart.height * usage / 100)
                width: Math.max(0, coreChart.width / root.stats.cores.length - 2)
                x: index * coreChart.width / root.stats.cores.length
            }
        }
    }
}
