import "../theme"
import "../state"
import "./components"
import "./modules"
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Hyprland._Ipc

PanelWindow {
    id: root

    required property var modelData
    property bool revealed: BarState.isBarVisibleForScreen(root.modelData.name)
    property bool transientReveal: !BarState.barPinnedVisible && BarState.barHoverRevealScreen === root.modelData.name
    property int barHeight: 44
    property int trackingBandHeight: 44

    function queueAutoHide() {
        if (!root.transientReveal || PanelState.anyPanelOpen || barHover.hovered || trackingHover.hovered)
            return;

        hideTimer.stop();
        hideTimer.start();
    }

    screen: modelData
    anchors.top: true
    anchors.left: true
    anchors.right: true
    implicitHeight: root.barHeight + (root.transientReveal ? root.trackingBandHeight : 0)
    exclusiveZone: root.revealed ? 50 : 0
    margins.top: root.revealed ? BarLayout.current.topMargin : -(root.implicitHeight + 12)
    margins.left: BarLayout.current.sideMargin
    margins.right: BarLayout.current.sideMargin
    color: "transparent"

    Item {
        id: barContent

        anchors.top: parent.top
        anchors.left: parent.left
        anchors.right: parent.right
        height: root.barHeight

        HoverHandler {
            id: barHover
        }

        // ── floating 悬浮底板（barBackground 预设才显示）──
        Rectangle {
            id: barPlate

            anchors.fill: parent
            visible: BarLayout.current.barBackground
            radius: BarLayout.current.squareCorners ? 0 : Tokens.radiusL
            color: Qt.rgba(Colors.base.r, Colors.base.g, Colors.base.b, BarLayout.current.barAlpha)
            border.color: Qt.rgba(1, 1, 1, Tokens.borderAlpha)
            border.width: BarLayout.current.barBackground ? Tokens.borderWidth : 0

            SoftShadow {
                anchors.fill: parent
                radius: parent.radius
                visible: BarLayout.current.barBackground
            }
        }

        // ── 左区 ──
        RowLayout {
            anchors.left: parent.left
            anchors.verticalCenter: parent.verticalCenter
            anchors.leftMargin: 4
            spacing: BarLayout.current.spacing
            height: parent.height

            Repeater {
                model: BarLayout.current.leftWidgets

                delegate: WidgetSlot {
                    required property var modelData

                    widgetItem: modelData
                    flat: BarLayout.current.moduleFlat
                    barScreen: root.modelData
                    barWindow: root
                    Layout.preferredWidth: implicitWidth
                    Layout.preferredHeight: implicitHeight
                }
            }
        }

        // ── 中区 ──
        RowLayout {
            anchors.centerIn: parent
            spacing: BarLayout.current.spacing
            height: parent.height

            Repeater {
                model: BarLayout.current.centerWidgets

                delegate: WidgetSlot {
                    required property var modelData

                    widgetItem: modelData
                    flat: BarLayout.current.moduleFlat
                    barScreen: root.modelData
                    barWindow: root
                    Layout.preferredWidth: implicitWidth
                    Layout.preferredHeight: implicitHeight
                }
            }
        }

        // ── 右区 ──
        RowLayout {
            anchors.right: parent.right
            anchors.verticalCenter: parent.verticalCenter
            anchors.rightMargin: 4
            spacing: BarLayout.current.spacing
            height: parent.height

            Repeater {
                model: BarLayout.current.rightWidgets

                delegate: WidgetSlot {
                    required property var modelData

                    widgetItem: modelData
                    flat: BarLayout.current.moduleFlat
                    barScreen: root.modelData
                    barWindow: root
                    Layout.preferredWidth: implicitWidth
                    Layout.preferredHeight: implicitHeight
                }
            }
        }
    }

    Item {
        id: trackingZone

        anchors.top: barContent.bottom
        anchors.left: parent.left
        anchors.right: parent.right
        height: root.trackingBandHeight
        visible: root.transientReveal

        HoverHandler {
            id: trackingHover
        }
    }

    Timer {
        id: hideTimer

        interval: 180
        onTriggered: {
            if (root.transientReveal && !PanelState.anyPanelOpen && !barHover.hovered && !trackingHover.hovered)
                BarState.hideHoverBar();
        }
    }

    Connections {
        target: PanelState

        function onAnyPanelOpenChanged() {
            if (PanelState.anyPanelOpen)
                hideTimer.stop();
            else
                root.queueAutoHide();
        }
    }

    Connections {
        target: barHover

        function onHoveredChanged() {
            if (barHover.hovered)
                hideTimer.stop();
            else
                root.queueAutoHide();
        }
    }

    Connections {
        target: trackingHover

        function onHoveredChanged() {
            if (trackingHover.hovered)
                hideTimer.stop();
            else
                root.queueAutoHide();
        }
    }

    Behavior on margins.top {
        NumberAnimation {
            duration: 150
            easing.type: Easing.OutCubic
        }
    }
}
