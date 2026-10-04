import "../theme"
import QtQuick
import QtQuick.Shapes

// 拖动期间在滑块附近发光，放开即淡出，不改变滑块命中范围或数值。
Shape {
    id: root

    property bool pressed: false
    property color accentColor: Colors.blue

    width: 72
    height: 32
    opacity: pressed ? 1 : 0
    preferredRendererType: Shape.CurveRenderer
    Accessible.ignored: true

    ShapePath {
        strokeColor: "transparent"
        fillTransform: Qt.matrix4x4(root.width, 0, 0, 0, 0, root.height, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1)
        fillGradient: RadialGradient {
            centerX: 0.5
            centerY: 0.5
            focalX: 0.5
            focalY: 0.5
            centerRadius: 0.5
            GradientStop {
                position: 0
                color: Colors.withAlpha(root.accentColor, 0.45)
            }
            GradientStop {
                position: 0.35
                color: Colors.withAlpha(root.accentColor, 0.20)
            }
            GradientStop {
                position: 1
                color: "transparent"
            }
        }
        PathRectangle {
            width: root.width
            height: root.height
        }
    }

    Behavior on opacity {
        NumberAnimation {
            duration: root.pressed ? 90 : 240
            easing.type: Easing.OutCubic
        }
    }
}
