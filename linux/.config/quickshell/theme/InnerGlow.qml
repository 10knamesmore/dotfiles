import QtQuick

// 顶部内发光 — 1px 水平渐变条模拟毛玻璃面板的顶部光源
Rectangle {
    anchors.top: parent.top
    anchors.left: parent.left
    anchors.right: parent.right
    height: 1
    z: 1

    gradient: Gradient {
        orientation: Gradient.Horizontal

        GradientStop {
            position: 0
            color: "transparent"
        }

        GradientStop {
            position: 0.3
            color: Qt.rgba(1, 1, 1, Tokens.glowAlpha)
        }

        GradientStop {
            position: 0.7
            color: Qt.rgba(1, 1, 1, Tokens.glowAlpha)
        }

        GradientStop {
            position: 1
            color: "transparent"
        }
    }
}
