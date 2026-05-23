import "../../theme"
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Services.SystemTray
import Quickshell.Widgets

// 系统托盘 — 不使用 BarModule，因为需要每个图标独立接收点击事件
Rectangle {
    id: root

    property var barWindow: null
    property bool flat: false

    implicitWidth: Math.max(trayRow.implicitWidth + 24, 36)
    implicitHeight: 36
    radius: 16
    color: root.flat ? Qt.rgba(Colors.surface1.r, Colors.surface1.g, Colors.surface1.b, 0.5) : Colors.surface0
    visible: trayRepeater.count > 0
    clip: false

    // 柔和阴影
    SoftShadow {
        anchors.fill: parent
        radius: root.radius
        visible: !root.flat
    }

    // 左侧弧形指示器
    Item {
        anchors.left: parent.left
        anchors.verticalCenter: parent.verticalCenter
        width: 4
        height: parent.height * 0.5
        clip: true
        visible: !root.flat

        Rectangle {
            anchors.left: parent.left
            anchors.top: parent.top
            anchors.bottom: parent.bottom
            width: root.radius * 2
            radius: root.radius
            color: Colors.overlay1
            opacity: 0.6
        }

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

                required property var modelData

                width: 22
                height: 22
                Layout.alignment: Qt.AlignVCenter

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
                    onClicked: (mouse) => {
                        if (mouse.button === Qt.RightButton || trayItem.modelData.onlyMenu) {
                            if (trayItem.modelData.hasMenu)
                                menuAnchor.open();

                        } else {
                            trayItem.modelData.activate();
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
