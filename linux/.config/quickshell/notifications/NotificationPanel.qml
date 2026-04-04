import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Wayland
import Quickshell.Services.Notifications
import "../theme"

// 通知历史面板 — 右上角弹出，显示已追踪的通知列表
PanelWindow {
    id: root

    required property var notifServer

    anchors.top: true
    anchors.bottom: true
    anchors.left: true
    anchors.right: true

    // 双阶段可见性
    property bool showing: PanelState.notificationOpen
    property bool animating: _opacityAnim.running || _slideAnim.running
    visible: showing || animating

    focusable: false
    exclusionMode: ExclusionMode.Ignore
    color: "transparent"

    // 半透明遮罩
    Rectangle {
        anchors.fill: parent
        color: "#000000"
        opacity: root.showing ? 0.15 : 0.0
        Behavior on opacity { NumberAnimation { duration: 250; easing.type: Easing.OutCubic } }
    }

    MouseArea {
        anchors.fill: parent
        onClicked: PanelState.notificationOpen = false
    }

    Rectangle {
        id: panel
        width: 380
        height: Math.min(col.implicitHeight + 32, root.height - 80)
        radius: 16
        color: Qt.rgba(Colors.surface0.r, Colors.surface0.g, Colors.surface0.b, 0.85)
        border.color: Qt.rgba(Colors.surface1.r, Colors.surface1.g, Colors.surface1.b, 0.9)
        border.width: 1
        anchors.top: parent.top
        anchors.right: parent.right
        anchors.topMargin: root.showing ? 54 : 34
        anchors.rightMargin: 10
        clip: true

        opacity: root.showing ? 1.0 : 0.0

        Behavior on anchors.topMargin {
            NumberAnimation { id: _slideAnim; duration: 250; easing.type: Easing.OutCubic }
        }
        Behavior on opacity {
            NumberAnimation { id: _opacityAnim; duration: 250; easing.type: Easing.OutCubic }
        }

        MouseArea {
            anchors.fill: parent
            onClicked: mouse => mouse.accepted = true
        }

        ColumnLayout {
            id: col
            anchors.fill: parent
            anchors.margins: 16
            spacing: 8

            // 标题栏
            RowLayout {
                Layout.fillWidth: true

                Text {
                    text: "通知"
                    font.family: "Hack Nerd Font"
                    font.pixelSize: 15
                    font.bold: true
                    color: Colors.text
                }

                Item { Layout.fillWidth: true }

                // 通知计数
                Text {
                    visible: root.notifServer.trackedNotifications.count > 0
                    text: root.notifServer.trackedNotifications.count + " 条"
                    color: Colors.subtext0
                    font.family: "Hack Nerd Font"
                    font.pixelSize: 11
                }

                // 清除全部按钮
                Rectangle {
                    visible: root.notifServer.trackedNotifications.count > 0
                    width: clearText.implicitWidth + 16
                    height: 26
                    radius: 13
                    color: clearArea.containsMouse ? Colors.surface1 : "transparent"
                    Behavior on color { ColorAnimation { duration: 150 } }

                    Text {
                        id: clearText
                        anchors.centerIn: parent
                        text: "清除全部"
                        color: Colors.subtext0
                        font.family: "Hack Nerd Font"
                        font.pixelSize: 11
                    }

                    MouseArea {
                        id: clearArea
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: PanelState.clearAllNotifications()
                    }
                }
            }

            Rectangle { Layout.fillWidth: true; height: 1; color: Colors.surface1 }

            // 空状态
            Text {
                visible: root.notifServer.trackedNotifications.count === 0
                text: "暂无通知"
                color: Colors.overlay0
                font.family: "Hack Nerd Font"
                font.pixelSize: 13
                Layout.alignment: Qt.AlignHCenter
                Layout.topMargin: 30
                Layout.bottomMargin: 30
            }

            // 通知列表
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
                    height: notifCol.implicitHeight + 16
                    radius: 10
                    color: notifHover.containsMouse ? Colors.surface2 : Colors.surface1
                    Behavior on color { ColorAnimation { duration: 150 } }

                    ColumnLayout {
                        id: notifCol
                        anchors {
                            left: parent.left; right: parent.right; top: parent.top
                            margins: 8
                        }
                        spacing: 4

                        // 应用名 + 关闭按钮
                        RowLayout {
                            Layout.fillWidth: true

                            Text {
                                text: modelData.appName || "未知"
                                color: Colors.subtext0
                                font.family: "Hack Nerd Font"
                                font.pixelSize: 10
                            }

                            Item { Layout.fillWidth: true }

                            Text {
                                text: "󰅖"
                                color: dismissArea.containsMouse ? Colors.red : Colors.overlay0
                                font.family: "Hack Nerd Font"
                                font.pixelSize: 14
                                Behavior on color { ColorAnimation { duration: 150 } }

                                MouseArea {
                                    id: dismissArea
                                    anchors.fill: parent
                                    anchors.margins: -4
                                    hoverEnabled: true
                                    cursorShape: Qt.PointingHandCursor
                                    onClicked: modelData.dismiss()
                                }
                            }
                        }

                        // 标题
                        Text {
                            text: modelData.summary || ""
                            color: Colors.text
                            font.family: "Hack Nerd Font"
                            font.pixelSize: 13
                            font.weight: Font.DemiBold
                            Layout.fillWidth: true
                            elide: Text.ElideRight
                        }

                        // 正文
                        Text {
                            visible: (modelData.body || "") !== ""
                            text: modelData.body || ""
                            color: Colors.subtext1
                            font.family: "Hack Nerd Font"
                            font.pixelSize: 11
                            Layout.fillWidth: true
                            wrapMode: Text.WordWrap
                            maximumLineCount: 3
                            elide: Text.ElideRight
                        }
                    }

                    MouseArea {
                        id: notifHover
                        anchors.fill: parent
                        hoverEnabled: true
                        acceptedButtons: Qt.NoButton
                    }
                }
            }
        }
    }
}
