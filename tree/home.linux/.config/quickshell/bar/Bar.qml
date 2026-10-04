import "../state"
import "../capture"
import "../theme"
import "./components"
import "./modules"
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Hyprland._Ipc
import Quickshell.Wayland

PanelWindow {
    id: root

    required property var modelData
    property bool revealed: CaptureService.recordingActive || BarState.isBarVisibleForScreen(root.modelData.name)
    property bool transientReveal: !CaptureService.recordingActive && !BarState.barPinnedVisible && BarState.barHoverRevealScreen === root.modelData.name
    property int barHeight: 44
    property int trackingBandHeight: 44

    function queueAutoHide() {
        if (!root.transientReveal || PanelState.anyPanelOpen || MorphState.heldItems.length > 0 || barHover.hovered || trackingHover.hovered)
            return ;

        hideTimer.stop();
        hideTimer.start();
    }

    onRevealedChanged: console.info("[bar]", root.modelData.name, root.revealed ? "show" : "hide")
    screen: modelData
    anchors.top: true
    anchors.left: true
    anchors.right: true
    implicitHeight: root.barHeight + (root.transientReveal ? root.trackingBandHeight : 0)
    exclusiveZone: root.revealed ? root.barHeight : 0
    margins.top: root.revealed ? 0 : -root.barHeight
    color: "transparent"
    // 单独 namespace
    WlrLayershell.namespace: "quickshell-bar"

    Rectangle {
        id: barContent

        color: Colors.withAlpha(Colors.base, Tokens.panelAlpha)
        anchors.top: parent.top
        anchors.left: parent.left
        anchors.right: parent.right
        height: root.barHeight

        SmokedGlass {
            anchors.fill: parent
            radius: 0
        }

        HoverHandler {
            id: barHover
        }

        // ── 左区 ──
        RowLayout {
            anchors.left: parent.left
            anchors.right: centerRow.left
            anchors.rightMargin: 16
            anchors.verticalCenter: parent.verticalCenter
            anchors.leftMargin: 12
            spacing: BarLayout.spacing
            height: parent.height

            Repeater {
                model: BarLayout.leftWidgets

                delegate: WidgetSlot {
                    required property var modelData

                    widgetItem: modelData
                    barScreen: root.modelData
                    barWindow: root
                    Layout.fillWidth: modelData === "navigation"
                    Layout.minimumWidth: modelData === "navigation" ? 0 : implicitWidth
                    Layout.maximumWidth: implicitWidth
                    Layout.preferredWidth: implicitWidth
                    Layout.preferredHeight: implicitHeight
                }

            }

            Item {
                Layout.fillWidth: true
            }

        }

        // ── 中区 ──
        RowLayout {
            id: centerRow

            anchors.centerIn: parent
            spacing: BarLayout.spacing
            height: parent.height

            Repeater {
                model: BarLayout.centerWidgets

                delegate: WidgetSlot {
                    required property var modelData

                    widgetItem: modelData
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
            anchors.rightMargin: 12
            spacing: BarLayout.spacing
            height: parent.height

            Repeater {
                model: BarLayout.rightWidgets

                delegate: WidgetSlot {
                    required property var modelData

                    widgetItem: modelData
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
            if (root.transientReveal && !PanelState.anyPanelOpen && MorphState.heldItems.length === 0 && !barHover.hovered && !trackingHover.hovered)
                BarState.hideHoverBar();

        }
    }

    Connections {
        function onAnyPanelOpenChanged() {
            if (PanelState.anyPanelOpen)
                hideTimer.stop();
            else
                root.queueAutoHide();
        }

        target: PanelState
    }

    Connections {
        function onHeldItemsChanged() {
            if (MorphState.heldItems.length > 0)
                hideTimer.stop();
            else
                root.queueAutoHide();
        }

        target: MorphState
    }

    Connections {
        function onHoveredChanged() {
            if (barHover.hovered)
                hideTimer.stop();
            else
                root.queueAutoHide();
        }

        target: barHover
    }

    Connections {
        function onHoveredChanged() {
            if (trackingHover.hovered)
                hideTimer.stop();
            else
                root.queueAutoHide();
        }

        target: trackingHover
    }

    Behavior on margins.top {
        NumberAnimation {
            duration: 200
            easing.type: Easing.OutCubic
        }

    }

}
