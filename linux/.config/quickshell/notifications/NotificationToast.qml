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
                return;

            let timeout = notification.expireTimeout > 0 ? notification.expireTimeout : 5000;
            toastModel.append({
                "notifId": notification.id,
                "appName": notification.appName || "",
                "summary": notification.summary || "",
                "body": notification.body || "",
                "timeout": timeout,
                "dismissed": false
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

                property bool exiting: false
                property real _startTime: 0
                property real _remainingTime: 0

                function dismissToast() {
                    if (model.dismissed)
                        return;
                    toastModel.setProperty(index, "dismissed", true);
                    exiting = true;
                    toast.opacity = 0;
                    toast.x = 50;
                    toast.scale = 0.92;
                    toast.height = 0;
                    removeTimer.start();
                }

                width: 340
                height: toastContent.implicitHeight + 16
                radius: Tokens.radiusM
                color: Qt.rgba(Colors.base.r, Colors.base.g, Colors.base.b, Tokens.toastAlpha)
                border.color: Qt.rgba(1, 1, 1, Tokens.borderAlpha)
                border.width: 1
                opacity: 0
                x: 50
                scale: 0.92
                clip: true

                SoftShadow {
                    anchors.fill: parent
                    radius: parent.radius
                }

                Component.onCompleted: {
                    opacity = 1;
                    x = 0;
                    scale = 1.0;
                    toast._startTime = Date.now();
                    dismissTimer.interval = model.timeout;
                    dismissTimer.start();
                    progressAnim.duration = model.timeout;
                    progressAnim.start();
                }

                Timer {
                    id: dismissTimer

                    onTriggered: toast.dismissToast()
                }

                Timer {
                    id: removeTimer

                    interval: 250
                    onTriggered: {
                        // 等全部 toast 都消失后一次性清空，避免索引漂移
                        for (let i = 0; i < toastModel.count; i++) {
                            if (!toastModel.get(i).dismissed)
                                return;
                        }
                        toastModel.clear();
                    }
                }

                ColumnLayout {
                    id: toastContent

                    spacing: 2

                    anchors {
                        fill: parent
                        margins: Tokens.spaceS
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

                // 进度条 — 底部渐缩显示剩余时间
                Rectangle {
                    id: progressBar

                    anchors.bottom: parent.bottom
                    anchors.left: parent.left
                    height: 2
                    width: 340
                    radius: 1
                    color: Qt.rgba(Colors.blue.r, Colors.blue.g, Colors.blue.b, 0.5)

                    NumberAnimation on width {
                        id: progressAnim

                        from: 340
                        to: 0
                        duration: 5000
                        running: false
                    }
                }

                // 点击关闭 toast，悬停暂停自动消失
                MouseArea {
                    anchors.fill: parent
                    hoverEnabled: true
                    onClicked: toast.dismissToast()
                    onEntered: {
                        toast._remainingTime = Math.max(500, dismissTimer.interval - (Date.now() - toast._startTime));
                        dismissTimer.stop();
                        progressAnim.pause();
                    }
                    onExited: {
                        dismissTimer.interval = toast._remainingTime;
                        toast._startTime = Date.now();
                        dismissTimer.start();
                        progressAnim.resume();
                    }
                }

                Behavior on opacity {
                    NumberAnimation {
                        duration: Tokens.animNormal
                        easing.type: Easing.BezierSpline
                        easing.bezierCurve: toast.exiting ? Anim.accelerate : Anim.decelerate
                    }
                }

                Behavior on x {
                    NumberAnimation {
                        duration: Tokens.animNormal
                        easing.type: Easing.BezierSpline
                        easing.bezierCurve: toast.exiting ? Anim.accelerate : Anim.decelerate
                    }
                }

                Behavior on scale {
                    NumberAnimation {
                        duration: Tokens.animNormal
                        easing.type: Easing.BezierSpline
                        easing.bezierCurve: toast.exiting ? Anim.accelerate : Anim.decelerate
                    }
                }

                Behavior on height {
                    NumberAnimation {
                        duration: Tokens.animNormal
                        easing.type: Easing.BezierSpline
                        easing.bezierCurve: toast.exiting ? Anim.accelerate : Anim.decelerate
                    }
                }
            }
        }
    }
}
