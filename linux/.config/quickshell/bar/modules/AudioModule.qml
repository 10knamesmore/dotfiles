import "../../theme"
import "../components"
import QtQuick
import Quickshell.Io

BarModule {
    id: root

    property int volumePct: 0
    property bool muted: false
    property bool pendingOsd: false

    function osdIcon() {
        if (muted)
            return "󰝟";
        if (volumePct <= 0)
            return "󰕿";
        if (volumePct < 50)
            return "󰖀";
        return "󰕾";
    }

    function applyAndRefresh(proc) {
        root.pendingOsd = true;
        proc.running = true;
        Qt.callLater(() => {
            volReader.running = false;
            volReader.running = true;
        });
    }

    // Waybar pulseaudio: format-icons ["  ", "  "], format-muted "  "
    function volumeIcon() {
        if (muted)
            return "";

        if (volumePct < 50)
            return "";

        return "";
    }

    accentColor: root.muted ? Colors.overlay0 : Colors.peach
    backgroundColor: root.muted ? Colors.mantle : Colors.surface0
    opacity: root.muted ? 0.7 : 1
    progress: root.muted ? 0 : root.volumePct / 100
    progressDraggable: true
    onProgressDragged: value => {
        let pct = Math.round(value * 100);
        volSetter.command = ["wpctl", "set-volume", "-l", "1.0", "@DEFAULT_AUDIO_SINK@", (pct / 100).toFixed(2)];
        applyAndRefresh(volSetter);
    }
    implicitWidth: label.implicitWidth + 32
    // 立即读取一次，之后每秒轮询
    Component.onCompleted: volReader.running = true
    onClicked: applyAndRefresh(muteToggler)
    onScrolled: delta => {
        return applyAndRefresh(delta > 0 ? volUp : volDown);
    }

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
                    if (root.pendingOsd) {
                        PanelState.osdType = "volume";
                        PanelState.osdValue = root.volumePct;
                        PanelState.osdIcon = root.osdIcon();
                        PanelState.osdVisible = true;
                        root.pendingOsd = false;
                    }
                }
            }
        }
    }

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
            visible: !root.muted
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
