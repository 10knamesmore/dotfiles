import "../theme"
import "../state"
import QtQuick
import QtQuick.Layouts
import QtQuick.Shapes
import Quickshell
import Quickshell.Wayland

// OSD 浮层 — 音量/亮度变化时在屏幕底部居中显示
PanelWindow {
    id: root

    // 双阶段可见性
    property bool showing: OsdState.osdVisible

    function refreshDismissTimer() {
        if (root.showing)
            dismissTimer.restart();
    }

    anchors.bottom: true
    anchors.left: true
    anchors.right: true
    implicitHeight: 120
    margins.bottom: 40
    exclusionMode: ExclusionMode.Ignore
    focusable: false
    WlrLayershell.namespace: "quickshell-osd"
    color: "transparent"
    visible: showing || _hideAnim.running
    onShowingChanged: {
        if (showing) {
            _hideAnim.stop();
            osdWidget.opacity = 0;
            osdWidget.scale = 0.95;
            _showAnim.start();
            refreshDismissTimer();
        } else {
            _showAnim.stop();
            _hideAnim.start();
        }
    }

    Connections {
        target: OsdState

        function onOsdValueChanged() {
            root.refreshDismissTimer();
        }

        function onOsdIconChanged() {
            root.refreshDismissTimer();
        }

    }

    // 自动关闭定时器
    Timer {
        id: dismissTimer

        interval: 1500
        onTriggered: OsdState.osdVisible = false
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
        color: Colors.withAlpha(Colors.base, Tokens.toastAlpha)
        border.color: Colors.overlay(Tokens.borderAlpha)
        border.width: 1
        opacity: 0

        SmokedGlass {
            anchors.fill: parent
            radius: osdWidget.radius
        }

        SoftShadow {
            anchors.fill: parent
            radius: parent.radius
        }

        Row {
            anchors.centerIn: parent
            spacing: 14

            Text {
                text: OsdState.osdIcon
                color: Colors.text
                font.family: Fonts.family
                font.pixelSize: Fonts.h1
                anchors.verticalCenter: parent.verticalCenter
            }

            Column {
                anchors.verticalCenter: parent.verticalCenter
                spacing: 6

                // 弯月前沿随数值移动；满值时收成与轨道一致的圆头。
                Rectangle {
                    id: track
                    property real displayedValue: Math.max(0, Math.min(1, OsdState.osdValue / 100))
                    readonly property real fillWidth: displayedValue * width
                    readonly property real capRadius: Math.min(height / 2, fillWidth / 2)
                    readonly property real fullCap: Math.max(0, (displayedValue - 0.95) / 0.05)
                    readonly property real edgeX: fillWidth - capRadius * fullCap
                    readonly property real bend: Math.min(3, fillWidth * 0.25)

                    width: 150
                    height: 8
                    radius: 4
                    color: Colors.surface1

                    Behavior on displayedValue { NumberAnimation { duration: 100 } }

                    Shape {
                        anchors.fill: parent
                        visible: track.fillWidth > 0
                        preferredRendererType: Shape.CurveRenderer
                        Accessible.ignored: true

                        ShapePath {
                            strokeColor: "transparent"
                            fillColor: Colors.blue
                            startX: track.capRadius
                            startY: 0
                            PathLine { x: track.edgeX; y: 0 }
                            PathQuad {
                                x: track.edgeX
                                y: track.height
                                controlX: track.fillWidth - 2 * track.bend * (1 - track.fullCap) + track.capRadius * track.fullCap
                                controlY: track.height / 2
                            }
                            PathLine { x: track.capRadius; y: track.height }
                            PathQuad {
                                x: track.capRadius
                                y: 0
                                controlX: -track.capRadius
                                controlY: track.height / 2
                            }
                        }
                    }
                }

                Text {
                    text: OsdState.osdValue + "%"
                    color: Colors.subtext0
                    font.family: Fonts.family
                    font.pixelSize: Fonts.small
                    anchors.horizontalCenter: parent.horizontalCenter
                }
            }
        }
    }
}
