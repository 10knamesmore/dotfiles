import "../../theme"
import QtQuick
import QtQuick.Layouts

// 基础模块包装：圆角背景 + 左侧弧形指示器(hover蔓延) + 弹性缩放 + 柔和阴影 + 状态色调
Rectangle {
    id: root

    property color accentColor: Colors.blue
    property color backgroundColor: Colors.surface0
    property color tintColor: "transparent"
    property bool hovered: hoverArea.containsMouse
    default property alias contents: inner.data

    signal clicked(var mouse)
    signal rightClicked(var mouse)
    signal scrolled(int delta)

    radius: Tokens.radiusL
    color: hovered ? Colors.surface1 : backgroundColor
    implicitHeight: 36
    scale: hovered ? 1.03 : 1.0

    // 柔和阴影
    SoftShadow {
        anchors.fill: parent
        radius: root.radius
        shadowColor: "#000000"
        strength: root.hovered ? Tokens.shadowHoverOpacity : Tokens.shadowOpacity
    }

    // 状态色调叠加层（电池/systemd 等用）
    Rectangle {
        anchors.fill: parent
        radius: root.radius
        color: root.tintColor
        visible: root.tintColor !== Qt.rgba(0, 0, 0, 0)

        Behavior on color {
            ColorAnimation {
                duration: Tokens.animElaborate
                easing.type: Easing.OutCubic
            }
        }
    }

    // 左侧弧形彩色指示器 — 沿胶囊圆弧蔓延
    Item {
        anchors.left: parent.left
        anchors.verticalCenter: parent.verticalCenter
        width: root.hovered ? 5 : 4
        height: root.hovered ? parent.height : parent.height * 0.5
        clip: true

        Rectangle {
            anchors.left: parent.left
            anchors.top: parent.top
            anchors.bottom: parent.bottom
            width: root.radius * 2
            radius: root.radius
            color: root.accentColor
            opacity: root.hovered ? 1.0 : 0.6

            Behavior on opacity {
                NumberAnimation {
                    duration: 300
                }
            }
        }

        Behavior on height {
            NumberAnimation {
                duration: 400
                easing.type: Easing.OutBack
            }
        }

        Behavior on width {
            NumberAnimation {
                duration: 300
                easing.type: Easing.OutCubic
            }
        }
    }

    // 内容区
    Item {
        id: inner

        anchors {
            fill: parent
            leftMargin: 16
            rightMargin: 14
            topMargin: 4
            bottomMargin: 4
        }
    }

    MouseArea {
        id: hoverArea

        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        acceptedButtons: Qt.LeftButton | Qt.RightButton
        onClicked: (mouse) => {
            if (mouse.button === Qt.RightButton)
                root.rightClicked(mouse);
            else
                root.clicked(mouse);
        }
        onWheel: (wheel) => {
            return root.scrolled(wheel.angleDelta.y > 0 ? 1 : -1);
        }
    }

    Behavior on color {
        ColorAnimation {
            duration: 300
            easing.type: Easing.OutCubic
        }
    }

    Behavior on scale {
        NumberAnimation {
            duration: 300
            easing.type: Easing.OutBack
        }
    }
}
