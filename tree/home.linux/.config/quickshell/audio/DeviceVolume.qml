import "../components"
import "../services"
import "../theme"
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

RowLayout {
    id: root

    required property var node
    spacing: Tokens.spaceM
    enabled: node?.ready ?? false

    ActionButton {
        text: root.node?.isSink ? (root.node?.audio?.muted ? "󰝟" : "󰕾")
            : (root.node?.audio?.muted ? "󰍭" : "󰍬")
        Accessible.name: root.node?.audio?.muted ? "取消静音" : "静音"
        Accessible.checkable: true
        Accessible.checked: root.node?.audio?.muted ?? false
        emphasized: root.node?.audio?.muted ?? false
        accent: Colors.peach
        onClicked: root.node.audio.muted = !root.node.audio.muted
    }

    Slider {
        id: volumeSlider
        Layout.fillWidth: true
        implicitHeight: 30
        from: 0
        to: 100
        stepSize: 1
        value: root.node?.audio ? Math.round(root.node.audio.volume * 100) : 0
        Accessible.name: (root.node?.description ?? "") + "音量"
        onMoved: AudioService.setDeviceVolume(root.node, value)
        HoverHandler {
            cursorShape: volumeSlider.enabled ? Qt.PointingHandCursor : Qt.ArrowCursor
        }

        background: Rectangle {
            x: volumeSlider.leftPadding
            y: volumeSlider.topPadding + (volumeSlider.availableHeight - height) / 2
            width: volumeSlider.availableWidth
            height: 5
            radius: 3
            color: Colors.surface2
            Rectangle {
                width: volumeSlider.visualPosition * parent.width
                height: parent.height
                radius: parent.radius
                color: root.node?.audio?.muted ? Colors.overlay1 : Colors.peach
            }
        }
        handle: Rectangle {
            x: volumeSlider.leftPadding + volumeSlider.visualPosition * (volumeSlider.availableWidth - width)
            y: volumeSlider.topPadding + (volumeSlider.availableHeight - height) / 2
            width: 14
            height: 14
            radius: 7
            color: volumeSlider.pressed ? Colors.peach : Colors.text
            border.width: volumeSlider.activeFocus ? 2 : 0
            border.color: Colors.peach
        }
    }

    Text {
        Layout.preferredWidth: 40
        text: Math.round(volumeSlider.value) + "%"
        horizontalAlignment: Text.AlignRight
        color: Colors.subtext0
        font.family: Fonts.family
        font.pixelSize: Fonts.small
    }
}
