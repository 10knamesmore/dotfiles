import "../theme"
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Wayland

// 电源菜单 — 全屏遮罩 + 居中操作按钮
PanelWindow {
    id: root

    // 双阶段可见性
    property bool showing: PanelState.powerMenuOpen
    property bool animating: _bgAnim.running || _contentAnim.running

    anchors.top: true
    anchors.bottom: true
    anchors.left: true
    anchors.right: true
    visible: showing || animating
    focusable: root.showing
    exclusionMode: ExclusionMode.Ignore
    color: "transparent"

    // 半透明遮罩
    Rectangle {
        id: bg

        anchors.fill: parent
        color: "#000000"
        opacity: root.showing ? 0.55 : 0

        Behavior on opacity {
            NumberAnimation {
                id: _bgAnim

                duration: 300
                easing.type: Easing.OutCubic
            }

        }

    }

    // 点击遮罩关闭
    MouseArea {
        anchors.fill: parent
        onClicked: PanelState.powerMenuOpen = false
    }

    // Escape 关闭
    Item {
        focus: root.showing
        Keys.onEscapePressed: PanelState.powerMenuOpen = false
    }

    // 居中按钮行
    Row {
        id: buttonRow

        anchors.centerIn: parent
        spacing: 40
        opacity: root.showing ? 1 : 0
        scale: root.showing ? 1 : 0.9

        PowerButton {
            icon: ""
            label: "锁屏"
            command: "hyprlock"
        }

        PowerButton {
            icon: "󰍃"
            label: "注销"
            command: "hyprctl dispatch exit"
        }

        PowerButton {
            icon: "󰤄"
            label: "挂起"
            command: "systemctl suspend"
        }

        PowerButton {
            icon: ""
            label: "重启"
            command: "systemctl reboot"
        }

        PowerButton {
            icon: ""
            label: "关机"
            command: "systemctl poweroff"
        }

        Behavior on opacity {
            NumberAnimation {
                id: _contentAnim

                duration: 300
                easing.type: Easing.OutCubic
            }

        }

        Behavior on scale {
            NumberAnimation {
                duration: 300
                easing.type: Easing.OutCubic
            }

        }

    }

    Process {
        id: powerProc
    }

    component PowerButton: Rectangle {
        property string icon: ""
        property string label: ""
        property string command: ""

        width: 100
        height: 110
        radius: 16
        color: btnArea.containsMouse ? Qt.rgba(Colors.surface1.r, Colors.surface1.g, Colors.surface1.b, 0.9) : Qt.rgba(Colors.surface0.r, Colors.surface0.g, Colors.surface0.b, 0.85)
        border.color: btnArea.containsMouse ? Colors.blue : Colors.surface1
        border.width: 1

        Column {
            anchors.centerIn: parent
            spacing: 8

            Text {
                text: icon
                color: Colors.text
                font.family: Fonts.family
                font.pixelSize: Fonts.display3
                anchors.horizontalCenter: parent.horizontalCenter
            }

            Text {
                text: label
                color: Colors.subtext0
                font.family: Fonts.family
                font.pixelSize: Fonts.body
                anchors.horizontalCenter: parent.horizontalCenter
            }

        }

        MouseArea {
            id: btnArea

            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: {
                PanelState.powerMenuOpen = false;
                powerProc.command = ["sh", "-c", command];
                powerProc.running = true;
            }
        }

        Behavior on color {
            ColorAnimation {
                duration: 200
            }

        }

        Behavior on border.color {
            ColorAnimation {
                duration: 200
            }

        }

    }

}
