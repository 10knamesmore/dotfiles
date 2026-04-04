import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Services.SystemTray
import Quickshell.Widgets
import "../../theme"

// 系统托盘 — 不使用 BarModule，因为需要每个图标独立接收点击事件
Rectangle {
    id: root
    implicitWidth: Math.max(trayRow.implicitWidth + 24, 36)
    implicitHeight: 36
    radius: 16
    color: Colors.surface0
    visible: trayRepeater.count > 0

    property var barWindow: null
    clip: false

    // 阴影
    Rectangle {
        anchors.fill: parent
        anchors.margins: -1
        anchors.topMargin: 0
        anchors.bottomMargin: -3
        z: -1
        radius: root.radius + 2
        color: "#000000"
        opacity: 0.15
    }

    // 左侧彩色边框
    Rectangle {
        width: 4
        height: parent.height * 0.6
        anchors.left: parent.left
        anchors.verticalCenter: parent.verticalCenter
        radius: 2
        color: Colors.overlay1
        opacity: 0.7
    }

    RowLayout {
        id: trayRow
        anchors.centerIn: parent
        spacing: 8

        Repeater {
            id: trayRepeater
            model: SystemTray.items

            delegate: Item {
                id: trayItem
                width: 22
                height: 22
                Layout.alignment: Qt.AlignVCenter

                required property var modelData

                IconImage {
                    anchors.fill: parent
                    source: trayItem.modelData.icon
                    implicitSize: 22
                }

                MouseArea {
                    id: trayArea
                    anchors.fill: parent
                    acceptedButtons: Qt.LeftButton | Qt.RightButton
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: mouse => {
                        if (mouse.button === Qt.RightButton || trayItem.modelData.onlyMenu) {
                            if (trayItem.modelData.hasMenu)
                                menuAnchor.open()
                        } else {
                            trayItem.modelData.activate()
                        }
                    }
                }

                QsMenuAnchor {
                    id: menuAnchor
                    menu: trayItem.modelData.menu
                    anchor.window: root.barWindow
                    anchor.item: trayItem
                }

            }
        }
    }
}
