import "../theme"
import "./components"
import "./modules"
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Hyprland._Ipc

PanelWindow {
    id: root

    required property var modelData
    property bool revealed: PanelState.isBarVisibleForScreen(root.modelData.name)
    property bool transientReveal: !PanelState.barPinnedVisible && PanelState.barHoverRevealScreen === root.modelData.name
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
    margins.top: root.revealed ? 6 : -(root.implicitHeight + 12)
    margins.left: 2
    margins.right: 2
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

        // ── 左区：工作区 · 布局状态 · 窗口标题 · systemd ──
        RowLayout {
            anchors.left: parent.left
            anchors.verticalCenter: parent.verticalCenter
            anchors.leftMargin: 4
            spacing: 6
            height: parent.height

            WorkspacesModule {
                barScreen: root.modelData
            }

            ScrollStatusModule {
                barScreen: root.modelData
            }

            WindowTitleModule {}

            TrayModule {
                barWindow: root
            }
        }

        // ── 中区：媒体 · 上行速度 · 时钟 · 下行速度 ──
        RowLayout {
            anchors.centerIn: parent
            spacing: 6
            height: parent.height

            MediaModule {}

            NetSpeedModule {
                direction: "up"
            }

            ClockModule {}

            NetSpeedModule {
                direction: "down"
            }
        }

        // ── 右区：CPU · 内存 · 音量 · 网络 · 屏幕效果 · 电量 · 托盘 ──
        RowLayout {
            anchors.right: parent.right
            anchors.verticalCenter: parent.verticalCenter
            anchors.rightMargin: 4
            spacing: 6
            height: parent.height

            CpuModule {}

            MemoryModule {}

            AudioModule {}

            NetworkModule {}

            ClipboardModule {}

            NotificationModule {}

            ScreenEffectsModule {}

            BatteryModule {}
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
                PanelState.hideHoverBar();
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
