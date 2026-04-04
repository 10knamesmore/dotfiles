import "../theme"
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Wayland

// OSD 浮层 — 音量/亮度变化时在屏幕底部居中显示
PanelWindow {
    id: root

    // 双阶段可见性
    property bool showing: PanelState.osdVisible

    anchors.bottom: true
    anchors.left: true
    anchors.right: true
    implicitHeight: 120
    margins.bottom: 40
    exclusionMode: ExclusionMode.Ignore
    focusable: false
    color: "transparent"
    visible: showing || _hideAnim.running
    onShowingChanged: {
        if (showing) {
            _hideAnim.stop();
            osdWidget.opacity = 0;
            osdWidget.scale = 0.95;
            _showAnim.start();
            dismissTimer.restart();
        } else {
            _showAnim.stop();
            _hideAnim.start();
        }
    }

    // 自动关闭定时器
    Timer {
        id: dismissTimer

        interval: 1500
        onTriggered: PanelState.osdVisible = false
    }

    ParallelAnimation {
        id: _showAnim

        NumberAnimation {
            target: osdWidget
            property: "opacity"
            to: 1
            duration: Tokens.animFast
            easing.type: Easing.BezierSpline
            easing.bezierCurve: Anim.decelerate
        }

        NumberAnimation {
            target: osdWidget
            property: "scale"
            to: 1
            duration: Tokens.animFast
            easing.type: Easing.BezierSpline
            easing.bezierCurve: Anim.decelerate
        }

    }

    ParallelAnimation {
        id: _hideAnim

        NumberAnimation {
            target: osdWidget
            property: "opacity"
            to: 0
            duration: Tokens.animFast
            easing.type: Easing.BezierSpline
            easing.bezierCurve: Anim.accelerate
        }

        NumberAnimation {
            target: osdWidget
            property: "scale"
            to: 0.95
            duration: Tokens.animFast
            easing.type: Easing.BezierSpline
            easing.bezierCurve: Anim.accelerate
        }

    }

    // OSD 主体
    Rectangle {
        id: osdWidget

        anchors.horizontalCenter: parent.horizontalCenter
        anchors.bottom: parent.bottom
        width: 240
        height: 80
        radius: Tokens.radiusXL
        color: Qt.rgba(Colors.base.r, Colors.base.g, Colors.base.b, Tokens.toastAlpha)
        border.color: Qt.rgba(1, 1, 1, Tokens.borderAlpha)
        border.width: 1
        opacity: 0

        SoftShadow {
            anchors.fill: parent
            radius: parent.radius
        }

        Row {
            anchors.centerIn: parent
            spacing: 14

            Text {
                text: PanelState.osdIcon
                color: Colors.text
                font.family: Fonts.family
                font.pixelSize: Fonts.h1
                anchors.verticalCenter: parent.verticalCenter
            }

            Column {
                anchors.verticalCenter: parent.verticalCenter
                spacing: 6

                // 进度条
                Rectangle {
                    width: 150
                    height: 6
                    radius: 3
                    color: Colors.surface1

                    Rectangle {
                        width: Math.max(0, Math.min(1, PanelState.osdValue / 100)) * parent.width
                        height: parent.height
                        radius: parent.radius
                        color: PanelState.osdType === "brightness" ? Colors.yellow : Colors.blue

                        Behavior on width {
                            NumberAnimation {
                                duration: 100
                            }

                        }

                        Behavior on color {
                            ColorAnimation {
                                duration: 200
                            }

                        }

                    }

                }

                Text {
                    text: PanelState.osdValue + "%"
                    color: Colors.subtext0
                    font.family: Fonts.family
                    font.pixelSize: Fonts.small
                    anchors.horizontalCenter: parent.horizontalCenter
                }

            }

        }

    }

}
