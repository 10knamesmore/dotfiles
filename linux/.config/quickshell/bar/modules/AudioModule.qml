import QtQuick
import Quickshell.Io
import "../components"
import "../../theme"

BarModule {
    id: root
    accentColor: root.muted ? Colors.overlay0 : Colors.peach
    backgroundColor: root.muted ? Colors.mantle : Colors.surface0
    opacity: root.muted ? 0.7 : 1.0
    implicitWidth: label.implicitWidth + 32

    Behavior on opacity { NumberAnimation { duration: 300 } }

    property int volumePct: 0
    property bool muted: false

    // ── 读取音量（wpctl get-volume @DEFAULT_AUDIO_SINK@）──
    // 输出格式：「Volume: 0.51」或「Volume: 0.51 [MUTED]」
    Process {
        id: volReader
        command: ["wpctl", "get-volume", "@DEFAULT_AUDIO_SINK@"]
        stdout: SplitParser {
            onRead: data => {
                let m = data.match(/Volume:\s+([\d.]+)(\s+\[MUTED\])?/);
                if (m) {
                    root.volumePct = Math.round(parseFloat(m[1]) * 100);
                    root.muted = m[2] !== undefined;
                }
            }
        }
    }

    // 立即读取一次，之后每秒轮询
    Component.onCompleted: volReader.running = true

    Timer {
        interval: 100
        running: true
        repeat: true
        onTriggered: {
            volReader.running = false;
            volReader.running = true;
        }
    }

    // ── 写操作 ──
    Process {
        id: muteToggler
        command: ["wpctl", "set-mute", "@DEFAULT_AUDIO_SINK@", "toggle"]
    }
    Process {
        id: volUp
        command: ["wpctl", "set-volume", "@DEFAULT_AUDIO_SINK@", "5%+"]
    }
    Process {
        id: volDown
        command: ["wpctl", "set-volume", "@DEFAULT_AUDIO_SINK@", "5%-"]
    }

    function applyAndRefresh(proc) {
        proc.running = true;
        Qt.callLater(() => {
            volReader.running = false;
            volReader.running = true;
        });
    }

    onClicked: applyAndRefresh(muteToggler)
    onScrolled: delta => applyAndRefresh(delta > 0 ? volUp : volDown)

    // Waybar pulseaudio: format-icons ["  ", "  "], format-muted "  "
    function volumeIcon() {
        if (muted)
            return "";
        if (volumePct < 50)
            return "";
        return "";
    }

    Row {
        id: label
        anchors.centerIn: parent
        spacing: 5

        Text {
            text: root.volumeIcon()
            color: root.muted ? Colors.overlay1 : Colors.peach
            font.family: "Hack Nerd Font"
            font.pixelSize: 14
            font.weight: Font.DemiBold
            anchors.verticalCenter: parent.verticalCenter
        }

        Text {
            visible: !root.muted
            text: root.volumePct + "%"
            color: Colors.text
            font.family: "Hack Nerd Font"
            font.pixelSize: 13
            font.weight: Font.DemiBold
            anchors.verticalCenter: parent.verticalCenter
        }
    }
}
