import "../theme"
import QtQuick
import QtQuick.Effects
import QtQuick.Shapes

Shape {
    id: root

    required property point cursorPosition
    required property size screenSize
    required property real radius
    property color accentColor: Colors.blue

    readonly property real gradientRadius: Math.hypot(screenSize.width, screenSize.height)
    readonly property real halfBrightnessDistance: 150

    function glowColorAtDistance(distance) {
        const relativeDistance = distance / halfBrightnessDistance;
        const brightness = 1 / (1 + relativeDistance * relativeDistance);
        const tint = Qt.tint(accentColor, Qt.rgba(1, 1, 1, 0.55 * brightness));
        return Colors.withAlpha(tint, brightness);
    }

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
            centerRadius: root.gradientRadius

            GradientStop {
                position: 0
                color: root.glowColorAtDistance(0)
            }
            GradientStop {
                position: 0.01
                color: root.glowColorAtDistance(0.01 * root.gradientRadius)
            }
            GradientStop {
                position: 0.02
                color: root.glowColorAtDistance(0.02 * root.gradientRadius)
            }
            GradientStop {
                position: 0.035
                color: root.glowColorAtDistance(0.035 * root.gradientRadius)
            }
            GradientStop {
                position: 0.06
                color: root.glowColorAtDistance(0.06 * root.gradientRadius)
            }
            GradientStop {
                position: 0.1
                color: root.glowColorAtDistance(0.1 * root.gradientRadius)
            }
            GradientStop {
                position: 0.18
                color: root.glowColorAtDistance(0.18 * root.gradientRadius)
            }
            GradientStop {
                position: 0.32
                color: root.glowColorAtDistance(0.32 * root.gradientRadius)
            }
            GradientStop {
                position: 0.56
                color: root.glowColorAtDistance(0.56 * root.gradientRadius)
            }
            GradientStop {
                position: 1
                color: root.glowColorAtDistance(root.gradientRadius)
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
