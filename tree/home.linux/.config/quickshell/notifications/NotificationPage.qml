pragma ComponentBehavior: Bound

import "../components"
import "../theme"
import "../state"
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Services.Notifications

// 控制中心的通知历史子页，与通知服务共享同一份通知列表。
Item {
    id: root

    required property var notifServer
    property int removingItems: 0

    required property bool showing

    Process {
        id: copyProc
    }

    ColumnLayout {
        id: col

        anchors.fill: parent
        anchors.margins: Tokens.spaceL
        spacing: Tokens.spaceS

        // ── 标题栏 ──
        RowLayout {
            Layout.fillWidth: true
            // 清空时保留工具栏高度，避免条目退场前整个列表先向上跳。
            Layout.minimumHeight: 26

            Text {
                text: "󰂚"
                color: Colors.overlay1
                font.family: Fonts.family
                font.pixelSize: Fonts.title
            }

            Text {
                text: "通知"
                font.family: Fonts.family
                font.pixelSize: Fonts.title
                font.bold: true
                color: Colors.text
            }

            Item {
                Layout.fillWidth: true
            }

            // 通知计数
            Text {
                visible: SystemState.notificationCount > 0
                text: SystemState.notificationCount + " 条"
                color: Colors.subtext0
                font.family: Fonts.family
                font.pixelSize: Fonts.small
            }

            // 清除全部按钮（红色 hover，对齐 ClipboardPanel）
            Rectangle {
                visible: SystemState.notificationCount > 0
                width: clearText.implicitWidth + 16
                height: 26
                radius: Tokens.radiusFull
                color: clearArea.containsMouse ? Colors.withAlpha(Colors.red, 0.15) : "transparent"

                Text {
                    id: clearText

                    anchors.centerIn: parent
                    text: "清除全部"
                    color: clearArea.containsMouse ? Colors.red : Colors.subtext0
                    font.family: Fonts.family
                    font.pixelSize: Fonts.small

                    Behavior on color {
                        ColorAnimation {
                            duration: 150
                        }
                    }
                }

                MouseArea {
                    id: clearArea

                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: {
                        console.info("[notifications] clear requested", SystemState.notificationCount);
                        SystemState.clearAllNotifications();
                    }
                }

                Behavior on color {
                    ColorAnimation {
                        duration: 150
                    }
                }
            }
        }

        Rectangle {
            Layout.fillWidth: true
            height: 1
            color: Colors.surface1
        }

        // ── 通知列表 ──
        ListView {
            id: notifList
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.preferredHeight: Math.max(contentHeight, emptyLabel.implicitHeight + 60)
            model: root.notifServer.trackedNotifications
            clip: true

            Text {
                id: emptyLabel
                anchors.horizontalCenter: parent.horizontalCenter
                anchors.top: parent.top
                anchors.topMargin: 30
                visible: opacity > 0
                opacity: notifList.count === 0 && root.removingItems === 0 ? 1 : 0
                text: "暂无通知"
                color: Colors.overlay0
                font.family: Fonts.family
                font.pixelSize: Fonts.bodyLarge
                Behavior on opacity {
                    NumberAnimation {
                        duration: 120
                    }
                }
            }

            delegate: FadeOutListItem {
                id: notifItem
                required property var modelData

                width: notifList.width
                implicitHeight: notifRow.implicitHeight + 16
                itemSpacing: 6
                animateRemoval: root.showing
                onRemovalStarted: root.removingItems++
                onRemovalFinished: root.removingItems--
                radius: Tokens.radiusMS
                color: notifHover.containsMouse ? Colors.surface2 : Colors.surface1

                // dismiss 会销毁通知对象；保留到 delegate 退场完成，文字才不会提前消失。
                RetainableLock {
                    object: notifItem.modelData
                    locked: true
                }

                // hover 检测（底层）
                MouseArea {
                    id: notifHover

                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    acceptedButtons: Qt.NoButton
                }

                RowLayout {
                    id: notifRow

                    spacing: Tokens.spaceS

                    anchors {
                        left: parent.left
                        right: parent.right
                        top: parent.top
                        margins: Tokens.spaceS
                    }

                    // 通知内容
                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 2

                        Text {
                            text: notifItem.modelData.appName || "未知"
                            color: Colors.subtext0
                            font.family: Fonts.family
                            font.pixelSize: Fonts.caption
                        }

                        Text {
                            text: notifItem.modelData.summary || ""
                            color: Colors.text
                            font.family: Fonts.family
                            font.pixelSize: Fonts.body
                            font.weight: Font.DemiBold
                            Layout.fillWidth: true
                            elide: Text.ElideRight
                        }

                        Text {
                            visible: (notifItem.modelData.body || "") !== ""
                            text: notifItem.modelData.body || ""
                            color: Colors.subtext1
                            font.family: Fonts.family
                            font.pixelSize: Fonts.small
                            Layout.fillWidth: true
                            wrapMode: Text.WordWrap
                            maximumLineCount: 3
                            elide: Text.ElideRight
                        }
                    }

                    // 复制按钮
                    Rectangle {
                        width: 28
                        height: 28
                        radius: Tokens.radiusFull
                        color: copyArea.containsMouse ? Colors.withAlpha(Colors.blue, 0.15) : "transparent"

                        Text {
                            anchors.centerIn: parent
                            text: "󰆏"
                            color: copyArea.containsMouse ? Colors.blue : Colors.overlay0
                            font.family: Fonts.family
                            font.pixelSize: Fonts.icon

                            Behavior on color {
                                ColorAnimation {
                                    duration: 150
                                }
                            }
                        }

                        MouseArea {
                            id: copyArea

                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: {
                                let text = notifItem.modelData.summary + (notifItem.modelData.body ? "\n" + notifItem.modelData.body : "");
                                copyProc.command = ["wl-copy", text];
                                copyProc.running = true;
                            }
                        }

                        Behavior on color {
                            ColorAnimation {
                                duration: 150
                            }
                        }
                    }

                    // 删除按钮
                    Rectangle {
                        width: 28
                        height: 28
                        radius: Tokens.radiusFull
                        color: dismissArea.containsMouse ? Colors.withAlpha(Colors.red, 0.15) : "transparent"

                        Text {
                            anchors.centerIn: parent
                            text: "󰅖"
                            color: dismissArea.containsMouse ? Colors.red : Colors.overlay0
                            font.family: Fonts.family
                            font.pixelSize: Fonts.icon

                            Behavior on color {
                                ColorAnimation {
                                    duration: 150
                                }
                            }
                        }

                        MouseArea {
                            id: dismissArea

                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: {
                                console.info("[notifications] dismiss requested", notifItem.modelData.id);
                                notifItem.modelData.dismiss();
                            }
                        }

                        Behavior on color {
                            ColorAnimation {
                                duration: 150
                            }
                        }
                    }
                }

                Behavior on color {
                    ColorAnimation {
                        duration: 150
                    }
                }
            }
        }
    }
}
