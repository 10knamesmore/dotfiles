import "../../theme"
import "../../state"
import "../../services"
import "../components"
import QtQuick
import Quickshell.Io

// 曲名、播放状态与进度保留在同一行；悬停和展开面板时显示播放器名与 PID。
BarModule {
    id: root

    readonly property var player: MediaService.activePlayer
    property bool showLyric: false
    property int playerPid: 0
    property bool copied: false
    readonly property string fullContent: {
        if (!player)
            return "暂无媒体播放";
        if (showLyric && LyricsState.currentLyric.length > 0)
            return LyricsState.currentLyric;
        const title = player.trackTitle || "";
        const artist = player.trackArtist || "";
        return title ? (artist ? title + " - " + artist : title) : player.identity;
    }
    readonly property string compactContent: {
        const limit = showLyric && LyricsState.currentLyric.length > 0 ? 40 : 35;
        return fullContent.length > limit ? fullContent.substring(0, limit - 3) + "…" : fullContent;
    }

    function playIcon() {
        return !player ? "󰓛" : (player.isPlaying ? "󰏤" : "󰐊");
    }

    accentColor: Colors.pink
    progress: player && player.lengthSupported && player.length > 0 ? player.position / player.length : -1
    readonly property real compactWidth: Math.max(compactMeasure.implicitWidth + iconText.implicitWidth + 38, 80)
    readonly property real hoverWidth: Math.min(600, fullMeasure.implicitWidth + iconText.implicitWidth + 38
        + (player ? identityText.implicitWidth + 16 : 0) + (playerPid > 0 ? Math.max(pidMeasure.implicitWidth, 70) + 6 : 0))
    implicitWidth: hovered ? hoverWidth : compactWidth
    onClicked: {
        PanelState.closeAll();
        MorphState.openFrom(root, () => PanelState.toggleMedia());
    }
    onRightClicked: {
        if (root.playerPid > 0) {
            copyProc.command = ["wl-copy", String(root.playerPid)];
            copyProc.running = true;
            root.copied = true;
            copiedTimer.restart();
        } else {
            root.showLyric = !root.showLyric;
        }
    }

    Process {
        id: pidReader
        property string output: ""
        command: root.player ? ["pgrep", "-fi", root.player.identity] : ["true"]
        stdout: SplitParser { onRead: data => pidReader.output += data + "\n" }
        onExited: {
            root.playerPid = parseInt(output.trim().split("\n")[0]) || 0;
            output = "";
        }
    }
    onPlayerChanged: {
        if (root.player)
            pidReader.running = true;
        else
            root.playerPid = 0;
    }
    Component.onCompleted: {
        if (root.player)
            pidReader.running = true;
    }
    Process { id: copyProc }
    Timer { id: copiedTimer; interval: 1500; onTriggered: root.copied = false }

    Text {
        id: compactMeasure
        visible: false
        text: root.compactContent
        font.family: Fonts.family
        font.pixelSize: Fonts.body
        font.weight: Font.Medium
    }

    Text {
        id: fullMeasure
        visible: false
        text: root.fullContent
        font.family: Fonts.family
        font.pixelSize: Fonts.body
        font.weight: Font.Medium
    }

    Text {
        id: pidMeasure
        visible: false
        text: "PID " + root.playerPid
        font.family: Fonts.family
        font.pixelSize: Fonts.caption
        font.weight: Font.DemiBold
    }

    Row {
        id: row
        anchors.left: parent.left
        anchors.verticalCenter: parent.verticalCenter
        width: parent.width
        spacing: 6

        Rectangle {
            id: identityTag
            visible: root.player !== null && root.detailProgress > 0
            width: root.player ? (identityText.implicitWidth + 10) * root.detailProgress : 0
            height: identityText.implicitHeight + 4
            radius: 4
            color: Colors.withAlpha(Colors.pink, 0.2)
            opacity: root.detailProgress
            clip: true
            anchors.verticalCenter: parent.verticalCenter

            Text {
                id: identityText
                anchors.centerIn: parent
                text: root.player ? root.player.identity : ""
                color: Colors.pink
                font.family: Fonts.family
                font.pixelSize: Fonts.caption
                font.weight: Font.DemiBold
            }
        }
        Text {
            id: iconText
            text: root.playIcon()
            color: Colors.pink
            font.family: Fonts.family
            font.pixelSize: Fonts.title
            font.weight: Font.DemiBold
            anchors.verticalCenter: parent.verticalCenter
        }
        Text {
            text: root.fullContent
            width: Math.max(0, row.width - iconText.width - identityTag.width - pidText.width
                - 6 - root.detailProgress * 6 * ((root.player ? 1 : 0) + (root.playerPid > 0 ? 1 : 0)))
            elide: Text.ElideRight
            color: root.showLyric && LyricsState.currentLyric.length > 0 ? Colors.mauve
                : (root.detailProgress > 0 ? Colors.pink : Colors.text)
            font.family: Fonts.family
            font.pixelSize: Fonts.body
            font.weight: Font.Medium
            font.italic: root.showLyric && LyricsState.currentLyric.length > 0
            anchors.verticalCenter: parent.verticalCenter
            Behavior on color { ColorAnimation { duration: Tokens.animFast } }
        }
        Text {
            id: pidText
            visible: root.playerPid > 0 && root.detailProgress > 0
            text: root.copied ? "✓ Copied" : "PID " + root.playerPid
            width: root.playerPid > 0 ? Math.max(pidMeasure.implicitWidth, 70) * root.detailProgress : 0
            opacity: root.detailProgress
            clip: true
            color: root.copied ? Colors.green : Colors.subtext0
            font.family: Fonts.family
            font.pixelSize: Fonts.caption
            font.weight: root.copied ? Font.DemiBold : Font.Normal
            anchors.verticalCenter: parent.verticalCenter
            Behavior on color { ColorAnimation { duration: Tokens.animFast } }
        }
    }
}
