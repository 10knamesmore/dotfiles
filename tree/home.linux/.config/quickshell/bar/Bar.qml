import "../state"
import "../theme"
import "./components"
import "./modules"
import QtQuick
import QtQuick.Layouts
import QtQuick.Shapes
import Quickshell
import Quickshell.Hyprland._Ipc
import Quickshell.Wayland

PanelWindow {
    id: root

    required property var modelData
    property bool revealed: BarState.isBarVisibleForScreen(root.modelData.name)
    property bool transientReveal: !BarState.barPinnedVisible && BarState.barHoverRevealScreen === root.modelData.name
    property int barHeight: 44
    property int trackingBandHeight: 44
    readonly property int cornerSize: 22
    property real cornerRadius: root.revealed ? root.cornerSize : 0

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
    implicitHeight: root.barHeight + Math.max(root.cornerSize, root.transientReveal ? root.trackingBandHeight : 0)
    exclusiveZone: root.revealed ? root.barHeight : 0
    margins.top: root.revealed ? 0 : -(root.barHeight + root.cornerSize)
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

    Repeater {
        model: 2

        Shape {
            id: corner

            required property int index

            width: root.cornerRadius
            height: root.cornerRadius
            y: root.barHeight
            x: index === 0 ? 0 : root.width - width
            visible: width > 0
            preferredRendererType: Shape.CurveRenderer

            // 方形减去四分之一圆，形成接到屏幕侧边的内凹轮廓。
            ShapePath {
                strokeWidth: 0
                strokeColor: "transparent"
                fillColor: barContent.color
                startX: 0
                startY: 0

                PathLine {
                    x: corner.width
                    y: 0
                }

                PathCubic {
                    x: 0
                    y: corner.height
                    control1X: corner.width * 0.447715
                    control1Y: 0
                    control2X: 0
                    control2Y: corner.height * 0.447715
                }

                PathLine {
                    x: 0
                    y: 0
                }

            }

            transform: Scale {
                origin.x: corner.width / 2
                xScale: corner.index === 0 ? 1 : -1
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

    // 圆角下方的透明区域不拦截桌面；临时唤出时才启用追踪带。
    mask: Region {
        width: root.width
        height: root.barHeight + (root.transientReveal ? root.trackingBandHeight : 0)
    }

    Behavior on cornerRadius {
        SequentialAnimation {
            PauseAnimation {
                duration: root.revealed ? 40 : 0
            }

            NumberAnimation {
                duration: 160
                easing.type: Easing.OutCubic
            }

        }

    }

    Behavior on margins.top {
        NumberAnimation {
            duration: 200
            easing.type: Easing.OutCubic
        }

    }

}
