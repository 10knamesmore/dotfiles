import "../../theme"
import QtQuick

// 顶栏交互模块。可展开模块的 moduleHeader 会暂时移入面板，文字和进度仍使用原组件。
Rectangle {
    id: root

    property color accentColor: Colors.blue
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
    readonly property alias moduleHeader: header
    default property alias contents: inner.data

    signal clicked(var mouse)
    signal rightClicked(var mouse)
    signal scrolled(int delta)
    signal progressDragged(real value)
    signal moved(var mouse)

    clip: true
    radius: Tokens.radiusS
    color: root.hovered ? Colors.overlay(0.07) : "transparent"
    implicitHeight: 36

    Item {
        id: header
        anchors.fill: parent
        clip: true

        Rectangle {
            id: progressTrack
            property real displayedProgress: Math.max(0, Math.min(1, root.progress))
            visible: root.progress >= 0
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.bottom: parent.bottom
            anchors.leftMargin: 15
            anchors.rightMargin: 15
            anchors.bottomMargin: 1
            height: 2
            radius: 1
            color: Colors.withAlpha(root.accentColor, 0.12)

            Rectangle {
                width: parent.width * progressTrack.displayedProgress
                height: parent.height
                radius: 1
                color: root.accentColor
            }

            Behavior on displayedProgress {
                NumberAnimation {
                    duration: Tokens.animNormal
                    easing.type: Easing.OutCubic
                }
            }
        }

        Item {
            id: inner
            anchors.fill: parent
            anchors.leftMargin: 15
            anchors.rightMargin: 15 + root.panelProgress * 28
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
        BarColorAnimation {}
    }
    Behavior on hoverReveal {
        enabled: !root.headerInPanel
        NumberAnimation {
            duration: Tokens.animSlow
            easing.type: Easing.OutCubic
        }
    }
    Behavior on implicitWidth {
        enabled: !root.headerInPanel
        BarWidthAnimation {}
    }
}
