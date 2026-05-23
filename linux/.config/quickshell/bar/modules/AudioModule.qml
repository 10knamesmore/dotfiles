import "../../theme"
import "../components"
import QtQuick
import Quickshell.Io
import Quickshell.Services.Pipewire

BarModule {
    id: root

    // 音量/静音直接从 PipeWire 取，事件驱动，不再 100ms 轮询 wpctl。
    // OSD 弹出由 shell.qml 的全局 PipeWire Connections 统一处理。
    readonly property var _sink: Pipewire.defaultAudioSink
    readonly property int volumePct: _sink && _sink.audio ? Math.round(_sink.audio.volume * 100) : 0
    readonly property bool muted: _sink && _sink.audio ? _sink.audio.muted : false

    // PipeWire 节点属性需要 tracker 才会实时同步，否则 volume 恒为 0
    PwObjectTracker {
        objects: root._sink ? [root._sink] : []
    }

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
    onProgressDragged: value => {
        let pct = Math.round(value * 100);
        volSetter.command = ["wpctl", "set-volume", "-l", "1.0", "@DEFAULT_AUDIO_SINK@", (pct / 100).toFixed(2)];
        volSetter.running = true;
    }
    implicitWidth: hovered ? (label.implicitWidth + 32) : (compactLabel.implicitWidth + 32)
    onClicked: muteToggler.running = true
    onScrolled: delta => {
        (delta > 0 ? volUp : volDown).running = true;
    }

    // ── 写操作 ──
    Process {
        id: muteToggler

        command: ["wpctl", "set-mute", "@DEFAULT_AUDIO_SINK@", "toggle"]
    }

    Process {
        id: volUp

        command: ["wpctl", "set-volume", "-l", "1.0", "@DEFAULT_AUDIO_SINK@", "5%+"]
    }

    Process {
        id: volDown

        command: ["wpctl", "set-volume", "@DEFAULT_AUDIO_SINK@", "5%-"]
    }

    Process {
        id: volSetter
    }

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
