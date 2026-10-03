pragma ComponentBehavior: Bound

import "../state"
import "../theme"
import QtQuick
import QtQuick.Controls

// 三个任务分区只用文字和滑动下划线区分，详情页仍归属「控制」。
Item {
    id: root

    implicitHeight: 44
    Accessible.role: Accessible.PageTabList
    Accessible.name: "控制中心分区"

    Row {
        anchors.fill: parent

        Repeater {
            model: [
                {
                    label: "控制",
                    page: "home"
                },
                {
                    label: "通知",
                    page: "notifications"
                },
                {
                    label: "剪贴板",
                    page: "clipboard"
                }
            ]

            delegate: Button {
                id: tab
                required property var modelData
                required property int index
                readonly property bool selected: PanelState.controlCenterTab === index
                width: root.width / 3
                height: root.height
                text: modelData.label
                Accessible.role: Accessible.PageTab
                Accessible.name: modelData.label
                Accessible.selected: selected
                onClicked: PanelState.openControlCenter(modelData.page)

                HoverHandler {
                    cursorShape: Qt.PointingHandCursor
                }

                background: Item {}
                contentItem: Item {
                    Row {
                        spacing: 6
                        anchors.centerIn: parent

                        Text {
                            text: tab.text
                            color: tab.selected ? Colors.blue : tab.hovered ? Colors.text : Colors.overlay1
                            font.family: Fonts.family
                            font.pixelSize: Fonts.body
                            font.weight: tab.selected ? Font.DemiBold : Font.Normal
                            Behavior on color {
                                ColorAnimation {
                                    duration: Tokens.animFast
                                }
                            }
                        }

                        Text {
                            visible: tab.index === 1 && SystemState.notificationCount > 0
                            text: SystemState.notificationCount
                            color: tab.selected ? Colors.blue : Colors.overlay1
                            font.family: Fonts.family
                            font.pixelSize: Fonts.small
                            anchors.verticalCenter: parent.verticalCenter
                        }
                    }
                }
            }
        }
    }

    Rectangle {
        anchors.bottom: parent.bottom
        width: parent.width
        height: 1
        color: Colors.overlay(0.08)
    }

    Rectangle {
        anchors.bottom: parent.bottom
        width: root.width / 3
        x: PanelState.controlCenterTab * width
        height: 2
        radius: 1
        color: Colors.blue
        Behavior on x {
            NumberAnimation {
                duration: 220
                easing.type: Easing.OutCubic
            }
        }
    }
}
