import "../../theme"
import QtQuick

// 顶栏胶囊。可展开模块的 capsuleHeader 会暂时移入面板，文字和进度仍使用原组件。
Rectangle {
    id: root

    property color accentColor: Colors.blue
    property color backgroundColor: Colors.surface0
    property color tintColor: "transparent"
    property real backgroundAlpha: Tokens.panelAlpha
    property bool flat: false
    property bool clickable: true
    property real expansion: 0
    property real panelProgress: 0
    property bool hovered: !headerInPanel && hoverArea.containsMouse
    readonly property bool headerInPanel: header.parent !== root
    property real hoverReveal: hovered ? 1 : 0
    readonly property bool hoverDetailsVisible: hoverReveal > 0
    readonly property real detailProgress: headerInPanel ? expansion : hoverReveal
    property real progress: -1
    property bool progressDraggable: false
    readonly property alias capsuleHeader: header
    default property alias contents: inner.data

    signal clicked(var mouse)
    signal rightClicked(var mouse)
    signal scrolled(int delta)
    signal progressDragged(real value)
    signal moved(var mouse)

    clip: true
    radius: Tokens.radiusL
    color: root.flat ? Colors.withAlpha(Colors.surface1, root.hovered ? 0.85 : 0.5)
        : (root.hovered ? Colors.withAlpha(Colors.surface1, Math.min(1, root.backgroundAlpha + 0.08))
            : Colors.withAlpha(root.backgroundColor, root.backgroundAlpha))
    border.color: hovered ? Colors.withAlpha(root.accentColor, Tokens.borderHoverAlpha) : Colors.overlay(0.06)
    border.width: root.flat ? 0 : Tokens.borderWidth
    implicitHeight: 36
    scale: hovered ? 1.03 : 1

    SoftShadow {
        anchors.fill: parent
        radius: root.radius
        shadowColor: "#000000"
        strength: root.hovered ? Tokens.shadowHoverOpacity : Tokens.shadowOpacity
        visible: !root.flat
    }

    Item {
        id: header
        anchors.fill: parent
        clip: true

        Rectangle {
            anchors.fill: parent
            radius: root.radius
            color: root.tintColor
            visible: root.tintColor !== Qt.rgba(0, 0, 0, 0)
            Behavior on color {
                ColorAnimation { duration: Tokens.animElaborate; easing.type: Easing.OutCubic }
            }
        }

        Item {
            property real displayedProgress: Math.max(0, root.progress)
            visible: root.progress >= 0
            anchors.left: parent.left
            anchors.top: parent.top
            anchors.bottom: parent.bottom
            width: displayedProgress * header.width
            clip: true

            Behavior on displayedProgress {
                NumberAnimation { duration: Tokens.animNormal; easing.type: Easing.OutCubic }
            }

            Rectangle {
                anchors.left: parent.left
                anchors.top: parent.top
                anchors.bottom: parent.bottom
                width: header.width
                radius: root.radius
                color: root.accentColor
                opacity: 0.27 + 0.15 * (root.headerInPanel ? root.expansion : (root.hovered ? 1 : 0))
                Behavior on opacity {
                    enabled: !root.headerInPanel
                    NumberAnimation { duration: Tokens.animFast }
                }
            }
        }

        Item {
            id: inner
            anchors.fill: parent
            anchors.leftMargin: 16
            anchors.rightMargin: 14 + root.panelProgress * 28
            anchors.topMargin: 4
            anchors.bottomMargin: 4
        }
    }

    MouseArea {
        visible: root.progressDraggable
        enabled: root.progressDraggable
        anchors.fill: parent
        preventStealing: true
        cursorShape: Qt.PointingHandCursor
        onPressed: mouse => root.progressDragged(Math.max(0, Math.min(1, mouse.x / width)))
        onPositionChanged: mouse => {
            if (pressed)
                root.progressDragged(Math.max(0, Math.min(1, mouse.x / width)));
        }
    }

    MouseArea {
        id: hoverArea
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: root.clickable ? Qt.PointingHandCursor : Qt.ArrowCursor
        acceptedButtons: root.clickable ? Qt.LeftButton | Qt.RightButton : Qt.NoButton
        onClicked: mouse => {
            if (mouse.button === Qt.RightButton)
                root.rightClicked(mouse);
            else
                root.clicked(mouse);
        }
        onPositionChanged: mouse => root.moved(mouse)
        onWheel: wheel => root.scrolled(wheel.angleDelta.y > 0 ? 1 : -1)
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
    Behavior on hoverReveal {
        enabled: !root.headerInPanel
        NumberAnimation { duration: Tokens.animSlow; easing.type: Easing.OutCubic }
    }
    Behavior on implicitWidth {
        enabled: !root.headerInPanel
        NumberAnimation { duration: Tokens.animSlow; easing.type: Easing.OutCubic }
    }
    Behavior on scale {
        enabled: !root.headerInPanel
        NumberAnimation {
            duration: Tokens.animNormal
            easing.type: Easing.BezierSpline
            easing.bezierCurve: Anim.elastic
        }
    }
}
