import "../../theme"
import "../components"
import QtQuick
import Quickshell.Services.Mpris

// 媒体播放模块 — 显示歌名+artist，左键打开面板，右键切换歌词显示
BarModule {
    id: root

    property var player: {
        let ps = Mpris.players.values;
        for (let i = 0; i < ps.length; i++) {
            if (ps[i].isPlaying) {
                PanelState.lastActivePlayer = ps[i];
                return ps[i];
            }
        }
        if (PanelState.lastActivePlayer && ps.indexOf(PanelState.lastActivePlayer) >= 0)
            return PanelState.lastActivePlayer;
        return ps.length > 0 ? ps[0] : null;
    }

    property bool showLyric: false

    accentColor: Colors.pink
    progress: player && player.lengthSupported && player.length > 0 ? player.position / player.length : -1
    implicitWidth: Math.max(row.implicitWidth + 32, 80)
    visible: player !== null
    onClicked: mouse => {
        PanelState.closeAll();
        let pos = root.mapToItem(null, mouse.x, mouse.y);
        PanelState.morphSourceX = pos.x + 2;
        PanelState.morphSourceY = pos.y + 6;
        PanelState.toggleMedia();
    }
    onRightClicked: showLyric = !showLyric

    Row {
        id: row

        anchors.centerIn: parent
        spacing: 6

        Text {
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

                // 右键切换：歌词模式 / 歌名+artist 模式
                if (root.showLyric && PanelState.currentLyric.length > 0) {
                    let l = PanelState.currentLyric;
                    return l.length > 40 ? l.substring(0, 37) + "…" : l;
                }

                let t = root.player.trackTitle || "";
                let a = root.player.trackArtist || "";

                if (!t)
                    return root.player.identity;

                let display = t;
                if (a)
                    display += " - " + a;

                return display.length > 35 ? display.substring(0, 32) + "…" : display;
            }
            color: root.showLyric && PanelState.currentLyric.length > 0 ? Colors.mauve : Colors.text
            font.family: Fonts.family
            font.pixelSize: Fonts.body
            font.weight: Font.Medium
            font.italic: root.showLyric && PanelState.currentLyric.length > 0
            anchors.verticalCenter: parent.verticalCenter

            Behavior on color {
                ColorAnimation {
                    duration: Tokens.animFast
                }
            }
        }
    }
}
