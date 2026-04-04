import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Wayland
import Quickshell.Services.Notifications
import "../theme"

// Toast 通知 — 右上角弹出，自动消失
PanelWindow {
    id: root

    required property var notifServer

    anchors.top: true
    anchors.right: true
    implicitWidth: 360
    implicitHeight: Math.max(1, toastCol.implicitHeight + 20)
    margins.top: 54
    margins.right: 10
    exclusionMode: ExclusionMode.Ignore
    focusable: false
    color: "transparent"
    visible: toastModel.count > 0

    ListModel { id: toastModel }

    Connections {
        target: root.notifServer
        function onNotification(notification) {
            // 面板打开时不弹 toast
            if (PanelState.notificationOpen) return

            let timeout = notification.expireTimeout > 0
                ? notification.expireTimeout * 1000
                : 5000
            toastModel.append({
                notifId: notification.id,
                appName: notification.appName || "",
                summary: notification.summary || "",
                body: notification.body || "",
                timeout: timeout
            })
        }
    }

    Column {
        id: toastCol
        anchors.top: parent.top
        anchors.right: parent.right
        anchors.margins: 0
        width: 340
        spacing: 6

        Repeater {
            model: toastModel

            delegate: Rectangle {
                id: toast
                width: 340
                height: toastContent.implicitHeight + 16
                radius: 12
                color: Qt.rgba(Colors.surface0.r, Colors.surface0.g, Colors.surface0.b, 0.85)
                border.color: Qt.rgba(Colors.surface1.r, Colors.surface1.g, Colors.surface1.b, 0.9)
                border.width: 1
                opacity: 0
                x: 50

                Component.onCompleted: {
                    opacity = 1
                    x = 0
                    dismissTimer.interval = model.timeout
                    dismissTimer.start()
                }

                Behavior on opacity {
                    NumberAnimation { duration: 200; easing.type: Easing.OutCubic }
                }
                Behavior on x {
                    NumberAnimation { duration: 250; easing.type: Easing.OutCubic }
                }

                Timer {
                    id: dismissTimer
                    onTriggered: {
                        toast.opacity = 0
                        toast.x = 50
                        removeTimer.start()
                    }
                }

                Timer {
                    id: removeTimer
                    interval: 250
                    onTriggered: {
                        // 按 notifId 查找并移除，避免索引漂移
                        let targetId = model.notifId
                        for (let i = 0; i < toastModel.count; i++) {
                            if (toastModel.get(i).notifId === targetId) {
                                toastModel.remove(i)
                                break
                            }
                        }
                    }
                }

                ColumnLayout {
                    id: toastContent
                    anchors { fill: parent; margins: 8 }
                    spacing: 2

                    Text {
                        text: model.appName
                        color: Colors.subtext0
                        font.family: "Hack Nerd Font"
                        font.pixelSize: 10
                        visible: model.appName !== ""
                    }

                    Text {
                        text: model.summary
                        color: Colors.text
                        font.family: "Hack Nerd Font"
                        font.pixelSize: 12
                        font.weight: Font.DemiBold
                        elide: Text.ElideRight
                        Layout.fillWidth: true
                    }

                    Text {
                        visible: model.body !== ""
                        text: model.body
                        color: Colors.subtext1
                        font.family: "Hack Nerd Font"
                        font.pixelSize: 11
                        wrapMode: Text.WordWrap
                        maximumLineCount: 2
                        elide: Text.ElideRight
                        Layout.fillWidth: true
                    }
                }

                // 点击关闭 toast
                MouseArea {
                    anchors.fill: parent
                    onClicked: {
                        toast.opacity = 0
                        toast.x = 50
                        removeTimer.start()
                    }
                }
            }
        }
    }
}
