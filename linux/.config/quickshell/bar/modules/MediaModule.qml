import QtQuick
import Quickshell.Services.Mpris
import "../components"
import "../../theme"

// 媒体播放模块 — 显示当前播放曲目，左键播放/暂停，滚轮调音量，右键展开面板
BarModule {
    id: root
    accentColor: Colors.pink
    implicitWidth: Math.max(row.implicitWidth + 32, 80)
    visible: player !== null

    property var player: Mpris.players.count > 0 ? Mpris.players.values[0] : null

    Row {
        id: row
        anchors.centerIn: parent
        spacing: 6

        Text {
            // Waybar media_volume.sh: 󰏤 (playing=pause icon), 󰐊 (paused=play icon), 󰓛 (stopped)
            text: {
                if (!root.player)
                    return "󰓛";
                return root.player.isPlaying ? "󰏤" : "󰐊";
            }
            color: Colors.pink
            font.family: "Hack Nerd Font"
            font.pixelSize: 15
            font.weight: Font.DemiBold
            anchors.verticalCenter: parent.verticalCenter
        }

        Text {
            text: {
                if (!root.player)
                    return "";
                let t = root.player.trackTitle;
                if (!t || t === "")
                    return root.player.identity;
                return t.length > 25 ? t.substring(0, 22) + "…" : t;
            }
            color: Colors.text
            font.family: "Hack Nerd Font"
            font.pixelSize: 12
            font.weight: Font.Medium
            anchors.verticalCenter: parent.verticalCenter
        }
    }

    onClicked: {
        if (player)
            player.togglePlaying();
    }

    onRightClicked: {
        PanelState.screenEffectsOpen = false;
        PanelState.calendarOpen = false;
        PanelState.toggleMedia();
    }

    onScrolled: delta => {
        if (player && player.volumeSupported) {
            player.volume = Math.max(0, Math.min(1, player.volume + delta * 0.05));
        }
    }
}
