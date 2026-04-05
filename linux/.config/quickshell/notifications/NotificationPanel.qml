import "../components"
import "../theme"
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Services.Notifications
import Quickshell.Wayland

// 通知历史面板 — 右上角弹出，对齐 ClipboardPanel 交互风格
PanelOverlay {
    id: root

    required property var notifServer

    showing: PanelState.notificationOpen
    panelWidth: 380
    panelHeight: Math.min(col.implicitHeight + 32, root.height - 80)
    panelTargetX: root.width - 390
    panelTargetY: 54
    closedOffsetY: -20
    onCloseRequested: PanelState.notificationOpen = false

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
                visible: PanelState.notificationCount > 0
                text: PanelState.notificationCount + " 条"
                color: Colors.subtext0
                font.family: Fonts.family
                font.pixelSize: Fonts.small
            }

            // 清除全部按钮（红色 hover，对齐 ClipboardPanel）
            Rectangle {
                visible: PanelState.notificationCount > 0
                width: clearText.implicitWidth + 16
                height: 26
                radius: Tokens.radiusFull
                color: clearArea.containsMouse ? Qt.rgba(Colors.red.r, Colors.red.g, Colors.red.b, 0.15) : "transparent"

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
                    onClicked: PanelState.clearAllNotifications()
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

        // 空状态
        Text {
            visible: PanelState.notificationCount === 0
            text: "暂无通知"
            color: Colors.overlay0
            font.family: Fonts.family
            font.pixelSize: Fonts.bodyLarge
            Layout.alignment: Qt.AlignHCenter
            Layout.topMargin: 30
            Layout.bottomMargin: 30
        }

        // ── 通知列表 ──
        ListView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.preferredHeight: contentHeight
            model: root.notifServer.trackedNotifications
            spacing: 6
            clip: true

            delegate: Rectangle {
                required property var modelData

                width: ListView.view.width
                height: notifRow.implicitHeight + 16
                radius: Tokens.radiusMS
                color: notifHover.containsMouse ? Colors.surface2 : Colors.surface1

                // hover 检测（底层）
                MouseArea {
                    id: notifHover

                    anchors.fill: parent
                    hoverEnabled: true
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
                            text: modelData.appName || "未知"
                            color: Colors.subtext0
                            font.family: Fonts.family
                            font.pixelSize: Fonts.caption
                        }

                        Text {
                            text: modelData.summary || ""
                            color: Colors.text
                            font.family: Fonts.family
                            font.pixelSize: Fonts.body
                            font.weight: Font.DemiBold
                            Layout.fillWidth: true
                            elide: Text.ElideRight
                        }

                        Text {
                            visible: (modelData.body || "") !== ""
                            text: modelData.body || ""
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
                        color: copyArea.containsMouse ? Qt.rgba(Colors.blue.r, Colors.blue.g, Colors.blue.b, 0.15) : "transparent"

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
                                let text = modelData.summary + (modelData.body ? "\n" + modelData.body : "");
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
                        color: dismissArea.containsMouse ? Qt.rgba(Colors.red.r, Colors.red.g, Colors.red.b, 0.15) : "transparent"

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
                            onClicked: modelData.dismiss()
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
