import "../theme"
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Services.Notifications
import Quickshell.Wayland

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

    ListModel {
        id: toastModel
    }

    Connections {
        function onNotification(notification) {
            // 勿扰模式或面板打开时不弹 toast
            if (PanelState.dndEnabled || PanelState.notificationOpen)
                return ;

            let timeout = notification.expireTimeout > 0 ? notification.expireTimeout : 5000;
            toastModel.append({
                "notifId": notification.id,
                "appName": notification.appName || "",
                "summary": notification.summary || "",
                "body": notification.body || "",
                "timeout": timeout
            });
        }

        target: root.notifServer
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

                function dismissToast() {
                    toast.opacity = 0;
                    toast.x = 50;
                    toast.height = 0;
                    removeTimer.start();
                }

                width: 340
                height: toastContent.implicitHeight + 16
                radius: 12
                color: Qt.rgba(Colors.base.r, Colors.base.g, Colors.base.b, Tokens.toastAlpha)
                border.color: Qt.rgba(1, 1, 1, Tokens.borderAlpha)
                border.width: 1
                opacity: 0
                x: 50
                clip: true
                Component.onCompleted: {
                    opacity = 1;
                    x = 0;
                    dismissTimer.interval = model.timeout;
                    dismissTimer.start();
                }

                Timer {
                    id: dismissTimer

                    onTriggered: toast.dismissToast()
                }

                Timer {
                    id: removeTimer

                    interval: 250
                    onTriggered: {
                        // 按 notifId 查找并移除，避免索引漂移
                        let targetId = model.notifId;
                        for (let i = 0; i < toastModel.count; i++) {
                            if (toastModel.get(i).notifId === targetId) {
                                toastModel.remove(i);
                                break;
                            }
                        }
                    }
                }

                ColumnLayout {
                    id: toastContent

                    spacing: 2

                    anchors {
                        fill: parent
                        margins: 8
                    }

                    Text {
                        text: model.appName
                        color: Colors.subtext0
                        font.family: Fonts.family
                        font.pixelSize: Fonts.caption
                        visible: model.appName !== ""
                    }

                    Text {
                        text: model.summary
                        color: Colors.text
                        font.family: Fonts.family
                        font.pixelSize: Fonts.body
                        font.weight: Font.DemiBold
                        elide: Text.ElideRight
                        Layout.fillWidth: true
                    }

                    Text {
                        visible: model.body !== ""
                        text: model.body
                        color: Colors.subtext1
                        font.family: Fonts.family
                        font.pixelSize: Fonts.small
                        wrapMode: Text.WordWrap
                        maximumLineCount: 2
                        elide: Text.ElideRight
                        Layout.fillWidth: true
                    }

                }

                // 点击关闭 toast
                MouseArea {
                    anchors.fill: parent
                    onClicked: toast.dismissToast()
                }

                Behavior on opacity {
                    NumberAnimation {
                        duration: 200
                        easing.type: Easing.OutCubic
                    }

                }

                Behavior on x {
                    NumberAnimation {
                        duration: 200
                        easing.type: Easing.OutCubic
                    }

                }

                Behavior on height {
                    NumberAnimation {
                        duration: 200
                        easing.type: Easing.OutCubic
                    }

                }

            }

        }

    }

}
