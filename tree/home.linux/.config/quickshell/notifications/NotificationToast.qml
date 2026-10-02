import "../theme"
import "../state"
import QtQuick
import QtQuick.Layouts
import QtQuick.Shapes
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
    WlrLayershell.namespace: "quickshell-toast"
    color: "transparent"
    visible: toastModel.count > 0

    ListModel {
        id: toastModel
    }

    Connections {
        function onNotification(notification) {
            // 通知面板打开时不弹 toast
            if (PanelState.notificationOpen)
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
        clip: true

        Repeater {
            model: toastModel

            delegate: Rectangle {
                id: toast

                property bool exiting: false
                property real _startTime: 0
                property real _remainingTime: 0

                function dismissToast() {
                    if (exiting)
                        return;
                    exiting = true;
                    dismissTimer.stop();
                    progressAnim.stop();
                    toast.x = toast.width;
                    removeTimer.start();
                }

                width: 340
                height: toastContent.implicitHeight + 16
                radius: Tokens.radiusM
                color: Colors.withAlpha(Colors.base, Tokens.toastAlpha)
                x: width
                clip: true

                Component.onCompleted: {
                    x = 0;
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

                    interval: Tokens.animNormal
                    onTriggered: {
                        // 水平滑出后才腾出位置，避免卡片在离场时向上收缩。
                        toast.height = 0;
                        toastModel.setProperty(index, "dismissed", true);
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

                // 倒计时只淡去边框：左上先消失，右下最后消失。
                Shape {
                    id: countdownBorder

                    property real elapsedFraction: 0 // 已用时间，0～1
                    readonly property real borderThickness: 2
                    readonly property real fadeSoftness: 0.24
                    readonly property real fadeFront: elapsedFraction * (1 + 2 * fadeSoftness) - fadeSoftness

                    anchors.fill: parent
                    preferredRendererType: Shape.CurveRenderer

                    ShapePath {
                        strokeColor: "transparent"
                        fillRule: ShapePath.OddEvenFill
                        // 归一到卡片宽高，让右上与左下同时渐隐，不受长宽比影响。
                        fillTransform: Qt.matrix4x4(countdownBorder.width, 0, 0, 0, 0, countdownBorder.height, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1)
                        fillGradient: LinearGradient {
                            x1: countdownBorder.fadeFront - countdownBorder.fadeSoftness
                            y1: x1
                            x2: countdownBorder.fadeFront + countdownBorder.fadeSoftness
                            y2: x2

                            GradientStop {
                                position: 0
                                color: Colors.withAlpha(Colors.blue, 0)
                            }
                            GradientStop {
                                position: 0.25
                                color: Colors.withAlpha(Colors.blue, 0.08)
                            }
                            GradientStop {
                                position: 0.5
                                color: Colors.withAlpha(Colors.blue, 0.25)
                            }
                            GradientStop {
                                position: 0.75
                                color: Colors.withAlpha(Colors.blue, 0.42)
                            }
                            GradientStop {
                                position: 1
                                color: Colors.withAlpha(Colors.blue, 0.5)
                            }
                        }

                        PathRectangle {
                            width: countdownBorder.width
                            height: countdownBorder.height
                            radius: toast.radius
                        }
                        PathRectangle {
                            x: countdownBorder.borderThickness
                            y: countdownBorder.borderThickness
                            width: Math.max(0, countdownBorder.width - 2 * countdownBorder.borderThickness)
                            height: Math.max(0, countdownBorder.height - 2 * countdownBorder.borderThickness)
                            radius: toast.radius - countdownBorder.borderThickness
                        }
                    }

                    NumberAnimation on elapsedFraction {
                        id: progressAnim

                        from: 0
                        to: 1
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
                        if (toast.exiting)
                            return;
                        toast._remainingTime = Math.max(500, dismissTimer.interval - (Date.now() - toast._startTime));
                        dismissTimer.stop();
                        if (progressAnim.running)
                            progressAnim.pause();
                    }
                    onExited: {
                        if (toast.exiting)
                            return;
                        dismissTimer.interval = toast._remainingTime;
                        toast._startTime = Date.now();
                        dismissTimer.start();
                        if (progressAnim.running)
                            progressAnim.resume();
                    }
                }

                Behavior on x {
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
