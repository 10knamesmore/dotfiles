import "../theme"
import QtQuick
import QtQuick.Shapes
import Quickshell
import Quickshell.Wayland

// 屏幕四角圆角遮罩 — 让整块屏幕像一张圆角卡片。
// 全屏透明 overlay 层 + 空输入区域（鼠标键盘全部穿透，不挡任何操作）。
PanelWindow {
    id: root

    required property var modelData
    property int cornerRadius: 14
    property color cornerColor: "#000000"

    screen: modelData
    anchors.top: true
    anchors.bottom: true
    anchors.left: true
    anchors.right: true
    exclusionMode: ExclusionMode.Ignore
    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.namespace: "quickshell:screencorners"
    color: "transparent"

    // 空输入区域 → 整窗输入穿透到下层窗口
    mask: Region {}

    // 实心角 + 圆形凹口（凹口朝屏幕内）。靠 rotation 复用到四角。
    component Corner: Shape {
        width: root.cornerRadius
        height: root.cornerRadius
        antialiasing: true

        ShapePath {
            fillColor: root.cornerColor
            strokeWidth: 0
            startX: 0
            startY: 0

            PathLine {
                x: root.cornerRadius
                y: 0
            }

            PathArc {
                x: 0
                y: root.cornerRadius
                radiusX: root.cornerRadius
                radiusY: root.cornerRadius
                direction: PathArc.Clockwise
            }

            PathLine {
                x: 0
                y: 0
            }
        }
    }

    Corner {
        anchors.top: parent.top
        anchors.left: parent.left
        rotation: 0
    }

    Corner {
        anchors.top: parent.top
        anchors.right: parent.right
        rotation: 90
    }

    Corner {
        anchors.bottom: parent.bottom
        anchors.right: parent.right
        rotation: 180
    }

    Corner {
        anchors.bottom: parent.bottom
        anchors.left: parent.left
        rotation: 270
    }
}
