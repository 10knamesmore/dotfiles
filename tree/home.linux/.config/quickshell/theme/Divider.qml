import QtQuick

// 渐变淡出分隔线 — 两端透明，中间柔和，替代硬切直线
Rectangle {
    height: 1

    gradient: Gradient {
        orientation: Gradient.Horizontal

        GradientStop {
            position: 0
            color: "transparent"
        }

        GradientStop {
            position: 0.15
            color: Colors.withAlpha(Colors.surface2, 0.4)
        }

        GradientStop {
            position: 0.5
            color: Colors.withAlpha(Colors.surface2, 0.5)
        }

        GradientStop {
            position: 0.85
            color: Colors.withAlpha(Colors.surface2, 0.4)
        }

        GradientStop {
            position: 1
            color: "transparent"
        }

    }

}
