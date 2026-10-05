import "../../theme"
import QtQuick
import QtQuick.Controls

// 天气面板的操作按钮；纯图标按钮由 label 提供可读名称。
Button {
    id: root

    property string label: text
    property bool iconOnly: false
    property string glyph: ""
    property bool busy: false

    implicitWidth: iconOnly ? 28 : contentItem.implicitWidth + 16
    implicitHeight: 28
    padding: 0
    hoverEnabled: true
    Accessible.name: label
    opacity: enabled ? 1 : 0.35
    scale: down ? 0.95 : 1
    ToolTip.visible: iconOnly && hovered
    ToolTip.text: label
    ToolTip.delay: 500

    HoverHandler {
        enabled: root.enabled
        cursorShape: Qt.PointingHandCursor
    }

    Behavior on scale {
        NumberAnimation {
            duration: Tokens.animFast
            easing.type: Easing.OutCubic
        }

    }

    Behavior on opacity {
        NumberAnimation {
            duration: Tokens.animFast
        }

    }

    background: Rectangle {
        radius: Tokens.radiusS
        color: root.hovered || root.visualFocus ? Colors.overlay(0.08) : "transparent"
        border.color: root.visualFocus ? Colors.mauve : "transparent"
        border.width: 1

        Behavior on color {
            ColorAnimation {
                duration: Tokens.animFast
            }

        }

    }

    contentItem: Item {
        implicitWidth: buttonRow.implicitWidth
        implicitHeight: buttonRow.implicitHeight
        Accessible.ignored: true

        Row {
            id: buttonRow

            anchors.centerIn: parent
            spacing: 5

            Text {
                visible: root.glyph !== ""
                text: root.glyph
                color: Colors.subtext0
                font.family: Fonts.family
                font.pixelSize: Fonts.caption

                RotationAnimator on rotation {
                    from: 0
                    to: 360
                    duration: 900
                    loops: Animation.Infinite
                    running: root.busy
                }

            }

            Text {
                text: root.text
                color: root.hovered || root.visualFocus ? Colors.text : Colors.subtext0
                font.family: Fonts.family
                font.pixelSize: root.iconOnly ? Fonts.icon : Fonts.caption
            }

        }

    }

}
