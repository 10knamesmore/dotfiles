pragma ComponentBehavior: Bound

import "../../theme"
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Services.SystemTray
import Quickshell.Widgets

// 系统托盘 — 不使用 BarModule，因为需要每个图标独立接收点击事件
Item {
    id: root

    property var barWindow: null

    implicitWidth: trayRow.implicitWidth
    implicitHeight: 36
    visible: trayRepeater.count > 0

    RowLayout {
        id: trayRow

        anchors.centerIn: parent
        spacing: BarLayout.spacing

        Repeater {
            id: trayRepeater

            model: SystemTray.items

            delegate: Rectangle {
                id: trayItem

                required property SystemTrayItem modelData
                readonly property bool highlighted: trayArea.containsMouse || activeFocus || menuAnchor.visible

                function openMenu() {
                    if (!modelData.hasMenu)
                        return;
                    console.info("[tray] open-menu", modelData.id);
                    menuAnchor.open();
                }

                function activate() {
                    if (modelData.onlyMenu) {
                        openMenu();
                    } else {
                        console.info("[tray] activate", modelData.id);
                        modelData.activate();
                    }
                }

                width: 36
                height: root.implicitHeight
                radius: Tokens.radiusXS
                color: trayArea.pressed ? Colors.withAlpha(Colors.lavender, 0.3) : highlighted ? Colors.withAlpha(Colors.lavender, 0.17) : "transparent"
                border.width: 1
                border.color: highlighted ? Colors.withAlpha(Colors.lavender, activeFocus ? 0.8 : 0.4) : "transparent"
                Layout.alignment: Qt.AlignVCenter
                activeFocusOnTab: true
                Accessible.role: Accessible.Button
                Accessible.name: modelData.title || modelData.id
                Accessible.description: modelData.tooltipDescription
                Accessible.pressed: trayArea.pressed
                Accessible.onPressAction: activate()
                Keys.onReturnPressed: activate()
                Keys.onSpacePressed: activate()
                Keys.onMenuPressed: openMenu()

                Behavior on border.color {
                    ColorAnimation {
                        duration: Tokens.animFast
                    }
                }

                Behavior on color {
                    ColorAnimation {
                        duration: Tokens.animFast
                    }
                }

                IconImage {
                    anchors.centerIn: parent
                    width: 22
                    height: 22
                    source: trayItem.modelData.icon
                    implicitSize: 22
                    scale: trayArea.pressed ? 0.9 : trayItem.highlighted ? 1.08 : 1

                    Behavior on scale {
                        NumberAnimation {
                            duration: Tokens.animFast
                            easing.type: Easing.OutCubic
                        }
                    }
                }

                MouseArea {
                    id: trayArea

                    anchors.fill: parent
                    acceptedButtons: Qt.LeftButton | Qt.RightButton
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: mouse => {
                        if (mouse.button === Qt.RightButton)
                            trayItem.openMenu();
                        else
                            trayItem.activate();
                    }
                }

                QsMenuAnchor {
                    id: menuAnchor

                    menu: trayItem.modelData.menu
                    anchor.item: trayItem
                    anchor.edges: Edges.Bottom | Edges.Left
                    anchor.gravity: Edges.Bottom | Edges.Right
                    anchor.margins.bottom: -Tokens.spaceXS
                }
            }
        }
    }
}
