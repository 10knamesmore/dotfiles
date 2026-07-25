import "../theme"
import "../state"
import QtQuick
import QtQuick.Layouts
import Quickshell
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
        opacity: root.showing ? Tokens.backdropDark : 0

        Behavior on opacity {
            NumberAnimation {
                id: _bgAnim

                duration: Tokens.animNormal
                easing.type: Easing.BezierSpline
                easing.bezierCurve: Anim.standard
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
            command: "hyprctl dispatch 'hl.dsp.exit()'"
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

                duration: Tokens.animNormal
                easing.type: Easing.BezierSpline
                easing.bezierCurve: Anim.standard
            }

        }

        Behavior on scale {
            NumberAnimation {
                duration: Tokens.animSlow
                easing.type: Easing.BezierSpline
                easing.bezierCurve: Anim.decelerate
            }

        }

    }

    component PowerButton: Rectangle {
        property string icon: ""
        property string label: ""
        property string command: ""

        width: 100
        height: 110
        radius: Tokens.radiusL
        color: btnArea.containsMouse ? Colors.withAlpha(Colors.surface1, 0.7) : Colors.withAlpha(Colors.base, Tokens.panelAlpha)
        border.color: btnArea.containsMouse ? Colors.blue : Colors.surface1
        border.width: 1

        SoftShadow {
            anchors.fill: parent
            radius: parent.radius
        }

        Column {
            anchors.centerIn: parent
            spacing: Tokens.spaceS

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
                // 必须 execDetached，别退回复用一个 Process：lock 起的 hyprlock 是长命进程，
                // 会永久占住那个 Process 对象，之后 suspend/reboot/poweroff 全部被静默丢弃。
                Quickshell.execDetached(["sh", "-c", command]);
            }
        }

        Behavior on color {
            ColorAnimation {
                duration: Tokens.animFast
                easing.type: Easing.BezierSpline
                easing.bezierCurve: Anim.standard
            }

        }

        Behavior on border.color {
            ColorAnimation {
                duration: Tokens.animFast
                easing.type: Easing.BezierSpline
                easing.bezierCurve: Anim.standard
            }

        }

    }

}
