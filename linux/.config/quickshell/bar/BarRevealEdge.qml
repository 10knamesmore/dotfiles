import "../theme"
import QtQuick
import Quickshell

// 顶部热边缘 — 当 bar 被手动隐藏时，鼠标触顶短暂停留可临时唤出当前屏幕的 bar
PanelWindow {
    id: root

    required property var modelData

    screen: modelData
    anchors.top: true
    anchors.left: true
    anchors.right: true
    implicitHeight: 3
    exclusionMode: ExclusionMode.Ignore
    color: "transparent"
    visible: !PanelState.barPinnedVisible && PanelState.barHoverRevealScreen !== root.modelData.name

    MouseArea {
        anchors.fill: parent
        hoverEnabled: true
        acceptedButtons: Qt.NoButton
        onEntered: revealTimer.start()
        onExited: revealTimer.stop()
    }

    Timer {
        id: revealTimer

        interval: 140
        onTriggered: PanelState.showBarForScreen(root.modelData.name)
    }
}
