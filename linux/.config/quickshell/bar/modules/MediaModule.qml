import "../../theme"
import "../components"
import QtQuick
import Quickshell.Services.Mpris

// 媒体播放模块 — 显示当前播放曲目，左键播放/暂停，滚轮调音量，右键展开面板
BarModule {
    id: root

    property var player: Mpris.players.count > 0 ? Mpris.players.values[0] : null

    accentColor: Colors.pink
    implicitWidth: Math.max(row.implicitWidth + 32, 80)
    visible: player !== null
    onClicked: {
        if (player)
            player.togglePlaying();

    }
    onRightClicked: {
        PanelState.screenEffectsOpen = false;
        PanelState.calendarOpen = false;
        PanelState.toggleMedia();
    }
    onScrolled: (delta) => {
        if (player && player.volumeSupported)
            player.volume = Math.max(0, Math.min(1, player.volume + delta * 0.05));

    }

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
            font.family: Fonts.family
            font.pixelSize: Fonts.title
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
            font.family: Fonts.family
            font.pixelSize: Fonts.body
            font.weight: Font.Medium
            anchors.verticalCenter: parent.verticalCenter
        }

    }

}
