pragma ComponentBehavior: Bound

import "../../theme"
import "../components"
import QtQuick
import Quickshell.Hyprland._Ipc

// 工作区命中区域与数字不动；活动标记移动时轻微拉长，到位后恢复为短线。
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
        id: activeMarker
        property real destination: root.activeIndex * 27 + 13
        property real center: destination
        property real stretch: 0
        property bool ready: false

        x: center - width / 2
        y: 29
        width: 8 + stretch
        height: 2
        radius: 1
        visible: root.activeIndex >= 0
        color: Colors.mauve
        Accessible.ignored: true
        Component.onCompleted: ready = true
        onDestinationChanged: {
            if (ready && visible)
                inkMotion.restart();
        }

        Behavior on center {
            NumberAnimation {
                duration: Tokens.animNormal
                easing.type: Easing.OutCubic
            }
        }

        SequentialAnimation {
            id: inkMotion
            NumberAnimation { target: activeMarker; property: "stretch"; to: 10; duration: 90; easing.type: Easing.OutCubic }
            NumberAnimation { target: activeMarker; property: "stretch"; to: 0; duration: 200; easing.type: Easing.OutCubic }
        }
    }
}
