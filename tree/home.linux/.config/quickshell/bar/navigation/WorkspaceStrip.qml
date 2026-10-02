pragma ComponentBehavior: Bound

import "../../theme"
import "../components"
import QtQuick
import Quickshell.Hyprland._Ipc

// 工作区入口使用稳定的命中区域；活动下划线滑动，悬停只改变底色与文字。
Item {
    id: root

    required property WindowContext context
    readonly property var workspaces: Hyprland.workspaces.values.filter(workspace => workspace.monitor && root.context.monitor && workspace.monitor.name === root.context.monitor.name && workspace.id > 0).sort((a, b) => a.id - b.id)
    readonly property int activeIndex: workspaces.findIndex(workspace => workspace.id === context.workspaceId)

    implicitWidth: workspaceRow.width
    implicitHeight: 36

    Row {
        id: workspaceRow
        spacing: 1
        anchors.verticalCenter: parent.verticalCenter

        Repeater {
            model: root.workspaces

            delegate: Rectangle {
                id: workspaceButton
                required property var modelData
                readonly property bool active: modelData.id === root.context.workspaceId

                width: 26
                height: 32
                radius: Tokens.radiusXS
                color: hover.containsMouse ? Colors.overlay(0.07) : "transparent"

                Text {
                    anchors.centerIn: parent
                    text: workspaceButton.modelData.name
                    textFormat: Text.PlainText
                    color: workspaceButton.active ? Colors.mauve : hover.containsMouse ? Colors.text : Colors.overlay1
                    font.family: Fonts.family
                    font.pixelSize: Fonts.body
                    font.weight: workspaceButton.active ? Font.DemiBold : Font.Normal
                    Behavior on color {
                        BarColorAnimation {}
                    }
                }

                MouseArea {
                    id: hover
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: {
                        console.info("[navigation] activate workspace", workspaceButton.modelData.id, "on", root.context.barScreen.name);
                        workspaceButton.modelData.activate();
                    }
                }

                Behavior on color {
                    BarColorAnimation {}
                }
            }
        }
    }

    Rectangle {
        x: root.activeIndex * 27 + 9
        y: 29
        width: 8
        height: 2
        radius: 1
        visible: root.activeIndex >= 0
        color: Colors.mauve
        Behavior on x {
            NumberAnimation {
                duration: Tokens.animNormal
                easing.type: Easing.OutCubic
            }
        }
    }
}
