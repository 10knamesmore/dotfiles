import "../theme"
import QtQuick
import QtQuick.Controls

Button {
    id: root

    property color accent: Colors.blue
    property bool emphasized: false

    implicitHeight: 30
    implicitWidth: contentItem.implicitWidth + 22
    padding: 8
    opacity: enabled ? 1 : 0.4
    Accessible.name: text
    ToolTip.visible: hovered && Accessible.name !== text
    ToolTip.text: Accessible.name
    ToolTip.delay: 600

    HoverHandler {
        cursorShape: root.enabled ? Qt.PointingHandCursor : Qt.ArrowCursor
    }

    background: Rectangle {
        radius: Tokens.radiusS
        color: root.down ? Colors.withAlpha(root.accent, 0.25)
            : root.hovered || root.emphasized ? Colors.withAlpha(root.accent, 0.14) : Colors.surface1
        border.width: root.activeFocus ? 1 : 0
        border.color: root.accent
        Behavior on color {
            ColorAnimation { duration: Tokens.animFast }
        }
    }

    contentItem: Text {
        text: root.text
        color: root.emphasized || root.hovered ? root.accent : Colors.subtext0
        font.family: Fonts.family
        font.pixelSize: Fonts.small
        horizontalAlignment: Text.AlignHCenter
        verticalAlignment: Text.AlignVCenter
        Behavior on color {
            ColorAnimation { duration: Tokens.animFast }
        }
    }
}
