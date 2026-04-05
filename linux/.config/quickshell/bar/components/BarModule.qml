import "../../theme"
import QtQuick
import QtQuick.Layouts

// 基础模块包装：圆角背景 + 左侧弧形指示器(hover蔓延) + 弹性缩放 + 柔和阴影 + 状态色调
Rectangle {
    id: root

    property color accentColor: Colors.blue
    property color backgroundColor: Colors.surface0
    property color tintColor: "transparent"
    property real backgroundAlpha: Tokens.panelAlpha
    property bool hovered: hoverArea.containsMouse
    property real progress: -1 // 0~1 进度值，-1 不启用
    property bool progressDraggable: false
    default property alias contents: inner.data

    signal clicked(var mouse)
    signal rightClicked(var mouse)
    signal scrolled(int delta)
    signal progressDragged(real value)

    radius: Tokens.radiusL
    color: hovered ? Qt.rgba(Colors.surface1.r, Colors.surface1.g, Colors.surface1.b, Math.min(1, root.backgroundAlpha + 0.08)) : Qt.rgba(root.backgroundColor.r, root.backgroundColor.g, root.backgroundColor.b, root.backgroundAlpha)
    border.color: hovered ? Qt.rgba(root.accentColor.r, root.accentColor.g, root.accentColor.b, Tokens.borderHoverAlpha) : Qt.rgba(1, 1, 1, 0.06)
    border.width: Tokens.borderWidth
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

    // 左侧弧形彩色指示器 — hover 时沿胶囊圆弧蔓延（无 progress 时）
    Item {
        anchors.left: parent.left
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        width: root.hovered ? 8 : 3
        clip: true
        visible: root.progress < 0

        Rectangle {
            anchors.left: parent.left
            anchors.verticalCenter: parent.verticalCenter
            width: root.hovered ? root.radius * 2 : parent.width
            height: root.hovered ? parent.height : parent.height * 0.5
            radius: root.hovered ? root.radius : 1.5
            color: root.accentColor
            opacity: root.hovered ? 1.0 : 0.6

            Behavior on opacity {
                NumberAnimation {
                    duration: Tokens.animNormal
                }
            }

            Behavior on width {
                NumberAnimation {
                    duration: Tokens.animSlow
                    easing.type: Easing.OutCubic
                }
            }

            Behavior on height {
                NumberAnimation {
                    duration: Tokens.animElaborate
                    easing {
                        type: Easing.OutBack
                        overshoot: 1.5
                    }
                }
            }

            Behavior on radius {
                NumberAnimation {
                    duration: Tokens.animSlow
                    easing.type: Easing.OutCubic
                }
            }
        }

        Behavior on width {
            NumberAnimation {
                duration: Tokens.animSlow
                easing.type: Easing.OutCubic
            }
        }
    }

    // 进度填充 — eclipse 风格，accent 色从左到右铺满模块高度
    Item {
        visible: root.progress >= 0
        anchors.fill: parent
        clip: true

        Rectangle {
            anchors.left: parent.left
            anchors.top: parent.top
            anchors.bottom: parent.bottom
            width: root.progress * parent.width
            radius: root.radius
            color: root.accentColor
            opacity: root.hovered ? 0.42 : 0.27

            Behavior on width {
                NumberAnimation {
                    duration: Tokens.animNormal
                    easing.type: Easing.OutCubic
                }
            }

            Behavior on opacity {
                NumberAnimation {
                    duration: Tokens.animFast
                }
            }
        }
    }

    // 拖动调节区域（progressDraggable 时覆盖整个模块）
    MouseArea {
        visible: root.progressDraggable
        enabled: root.progressDraggable
        anchors.fill: parent
        preventStealing: true
        cursorShape: Qt.PointingHandCursor
        onPressed: mouse => {
            let val = mouse.x / width;
            root.progressDragged(Math.max(0, Math.min(1, val)));
        }
        onPositionChanged: mouse => {
            if (pressed) {
                let val = mouse.x / width;
                root.progressDragged(Math.max(0, Math.min(1, val)));
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
        onClicked: mouse => {
            if (mouse.button === Qt.RightButton)
                root.rightClicked(mouse);
            else
                root.clicked(mouse);
        }
        onWheel: wheel => {
            return root.scrolled(wheel.angleDelta.y > 0 ? 1 : -1);
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

    Behavior on scale {
        NumberAnimation {
            duration: Tokens.animNormal
            easing.type: Easing.BezierSpline
            easing.bezierCurve: Anim.elastic
        }
    }
}
