import "../theme"
import QtQuick
import QtQuick.Effects
import QtQuick.Shapes

Shape {
    id: root

    required property point cursorPosition
    required property real radius
    property color accentColor: Colors.blue

    preferredRendererType: Shape.CurveRenderer
    Accessible.ignored: true
    layer.enabled: visible
    layer.effect: MultiEffect {
        shadowEnabled: true
        shadowColor: root.accentColor
        shadowOpacity: 1
        shadowBlur: 1
        blurMax: 12
    }

    ShapePath {
        strokeColor: "transparent"
        fillRule: ShapePath.OddEvenFill
        fillGradient: RadialGradient {
            centerX: root.cursorPosition.x
            centerY: root.cursorPosition.y
            focalX: centerX
            focalY: centerY
            centerRadius: 240

            GradientStop {
                position: 0
                color: Qt.tint(root.accentColor, Qt.rgba(1, 1, 1, 0.55))
            }
            GradientStop {
                position: 0.3
                color: Colors.withAlpha(root.accentColor, 0.75)
            }
            GradientStop {
                position: 0.65
                color: Colors.withAlpha(root.accentColor, 0.25)
            }
            GradientStop {
                position: 1
                color: "transparent"
            }
        }

        PathRectangle {
            width: root.width
            height: root.height
            radius: root.radius
        }
        PathRectangle {
            x: 1.5
            y: 1.5
            width: root.width - 3
            height: root.height - 3
            radius: Math.max(0, root.radius - 1.5)
        }
    }
}
