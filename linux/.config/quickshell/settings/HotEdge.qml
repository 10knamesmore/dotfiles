import "../theme"
import QtQuick
import Quickshell
import Quickshell.Wayland

// 屏幕左侧热边缘 — 鼠标推到边缘并停留一会即可唤出 QuickSettings
PanelWindow {
    id: root

    required property var modelData

    screen: modelData
    anchors.left: true
    anchors.top: true
    anchors.bottom: true
    implicitWidth: 2
    exclusionMode: ExclusionMode.Ignore
    color: "transparent"
    // 面板已打开或正在动画时隐藏热边缘，避免冲突
    visible: !PanelState.settingsOpen

    MouseArea {
        anchors.fill: parent
        hoverEnabled: true
        onEntered: pressureTimer.start()
        onExited: pressureTimer.stop()
    }

    Timer {
        id: pressureTimer

        interval: 250
        onTriggered: {
            PanelState.closeAll();
            PanelState.settingsOpen = true;
        }
    }

}
