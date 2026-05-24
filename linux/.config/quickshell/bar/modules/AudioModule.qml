import "../../theme"
import "../../services"
import "../components"
import QtQuick

BarModule {
    id: root

    // 音量/静音统一走 AudioService（PipeWire 事件驱动）。
    readonly property int volumePct: AudioService.volume
    readonly property bool muted: AudioService.muted

    // Waybar pulseaudio: format-icons ["  ", "  "], format-muted "  "
    function volumeIcon() {
        if (muted)
            return "";

        if (volumePct < 50)
            return "";

        return "";
    }

    accentColor: root.muted ? Colors.overlay0 : Colors.peach
    backgroundColor: root.muted ? Colors.mantle : Colors.surface0
    opacity: root.muted ? 0.7 : 1
    progress: root.muted ? 0 : root.volumePct / 100
    progressDraggable: true
    onProgressDragged: value => AudioService.setVolume(Math.round(value * 100))
    implicitWidth: hovered ? (label.implicitWidth + 32) : (compactLabel.implicitWidth + 32)
    onClicked: AudioService.toggleMute()
    onScrolled: delta => AudioService.step(delta > 0 ? 5 : -5)

    Row {
        id: compactLabel
        visible: false
        spacing: 5
        Text { text: root.volumeIcon(); font.family: Fonts.family; font.pixelSize: Fonts.icon }
    }

    Row {
        id: label

        anchors.centerIn: parent
        spacing: 5

        Text {
            text: root.volumeIcon()
            color: root.muted ? Colors.overlay1 : Colors.peach
            font.family: Fonts.family
            font.pixelSize: Fonts.icon
            font.weight: Font.DemiBold
            anchors.verticalCenter: parent.verticalCenter
        }

        Text {
            visible: !root.muted && root.hovered
            text: root.volumePct + "%"
            color: Colors.text
            font.family: Fonts.family
            font.pixelSize: Fonts.bodyLarge
            font.weight: Font.DemiBold
            anchors.verticalCenter: parent.verticalCenter
        }
    }

    Behavior on opacity {
        NumberAnimation {
            duration: 300
        }
    }
}
