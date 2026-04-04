import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Hyprland._Ipc
import "./components"
import "./modules"
import "../theme"

PanelWindow {
    id: root

    required property var modelData
    screen: modelData

    anchors.top: true
    anchors.left: true
    anchors.right: true

    implicitHeight: 44
    exclusiveZone: PanelState.barVisible ? 50 : 0
    margins.top: PanelState.barVisible ? 6 : -56
    margins.left: 2
    margins.right: 2
    color: "transparent"

    Behavior on margins.top {
        NumberAnimation {
            duration: 150
            easing.type: Easing.OutCubic
        }
    }

    Item {
        id: barContent
        anchors.fill: parent

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
}
