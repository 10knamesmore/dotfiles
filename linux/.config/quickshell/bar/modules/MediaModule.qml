import "../../theme"
import "../components"
import QtQuick
import Quickshell.Services.Mpris
import Quickshell.Io

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
    property int playerPid: 0
    property bool copied: false

    // 完整内容文字
    property string fullContent: {
        if (!player)
            return "";
        if (showLyric && PanelState.currentLyric.length > 0)
            return PanelState.currentLyric;
        let t = player.trackTitle || "";
        let a = player.trackArtist || "";
        if (!t)
            return player.identity;
        return a ? t + " - " + a : t;
    }

    // 截断内容文字
    property string truncatedContent: {
        if (!player)
            return "";
        if (showLyric && PanelState.currentLyric.length > 0) {
            let l = PanelState.currentLyric;
            return l.length > 40 ? l.substring(0, 37) + "…" : l;
        }
        let t = player.trackTitle || "";
        let a = player.trackArtist || "";
        if (!t)
            return player.identity;
        let display = a ? t + " - " + a : t;
        return display.length > 35 ? display.substring(0, 32) + "…" : display;
    }

    function playIcon() {
        if (!player)
            return "󰓛";
        return player.isPlaying ? "󰏤" : "󰐊";
    }

    accentColor: Colors.pink
    progress: player && player.lengthSupported && player.length > 0 ? player.position / player.length : -1
    implicitWidth: root.hovered
        ? Math.min(hoverRow.implicitWidth + 32, 600)
        : Math.max(row.implicitWidth + 32, 80)
    visible: player !== null
    onClicked: mouse => {
        PanelState.closeAll();
        let pos = root.mapToItem(null, mouse.x, mouse.y);
        PanelState.morphSourceX = pos.x + 2;
        PanelState.morphSourceY = pos.y + 6;
        PanelState.toggleMedia();
    }
    onRightClicked: {
        if (root.hovered && root.playerPid > 0) {
            copyProc.command = ["wl-copy", String(root.playerPid)];
            copyProc.running = true;
            root.copied = true;
            copiedTimer.restart();
        } else {
            showLyric = !showLyric;
        }
    }

    // ── PID 获取 ──
    Process {
        id: pidReader

        property string _buf: ""

        command: root.player ? ["pgrep", "-fi", root.player.identity] : ["true"]

        stdout: SplitParser {
            onRead: data => pidReader._buf += data + "\n"
        }

        onExited: {
            let line = pidReader._buf.trim().split("\n")[0];
            root.playerPid = parseInt(line) || 0;
            pidReader._buf = "";
        }
    }

    Timer {
        interval: 2000
        running: root.player !== null
        repeat: true
        onTriggered: {
            pidReader.running = false;
            pidReader.running = true;
        }
    }

    Component.onCompleted: {
        if (root.player)
            pidReader.running = true;
    }

    Process {
        id: copyProc
    }

    Timer {
        id: copiedTimer

        interval: 1500
        onTriggered: root.copied = false
    }

    // ── 默认视图 ──
    Row {
        id: row

        visible: !root.hovered
        anchors.centerIn: parent
        spacing: 6

        Text {
            text: root.playIcon()
            color: Colors.pink
            font.family: Fonts.family
            font.pixelSize: Fonts.title
            font.weight: Font.DemiBold
            anchors.verticalCenter: parent.verticalCenter
        }

        Text {
            text: root.truncatedContent
            color: root.showLyric && PanelState.currentLyric.length > 0 ? Colors.mauve : Colors.text
            font.family: Fonts.family
            font.pixelSize: Fonts.body
            font.weight: Font.Medium
            font.italic: root.showLyric && PanelState.currentLyric.length > 0
            anchors.verticalCenter: parent.verticalCenter

            Behavior on color {
                ColorAnimation { duration: Tokens.animFast }
            }
        }
    }

    // 隐藏测量：PID 文字的固定宽度（取较长的那个状态）
    Text {
        id: pidMeasure

        visible: false
        text: "PID " + root.playerPid
        font.family: Fonts.family
        font.pixelSize: Fonts.caption
        font.weight: Font.DemiBold
    }

    // ── hover 视图：进程名 + 播放图标 + 完整内容 + PID ──
    Row {
        id: hoverRow

        property real maxWidth: 600
        // 固定预留宽度，避免内容文字挤压 PID
        property real reservedWidth: (root.player ? identityText.implicitWidth + 10 + spacing : 0) + 20 + spacing + (root.playerPid > 0 ? Math.max(pidMeasure.implicitWidth, 70) + spacing : 0) + 32

        visible: root.hovered
        anchors.centerIn: parent
        spacing: 6

        // 进程名标签
        Rectangle {
            id: identityTag

            visible: root.player !== null
            color: Qt.rgba(Colors.pink.r, Colors.pink.g, Colors.pink.b, 0.2)
            radius: 4
            width: identityText.implicitWidth + 10
            height: identityText.implicitHeight + 4
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

        // 播放图标
        Text {
            text: root.playIcon()
            color: Colors.pink
            font.family: Fonts.family
            font.pixelSize: Fonts.title
            font.weight: Font.DemiBold
            anchors.verticalCenter: parent.verticalCenter
        }

        // 完整内容
        Text {
            text: root.fullContent
            color: root.showLyric && PanelState.currentLyric.length > 0 ? Colors.mauve : Colors.pink
            font.family: Fonts.family
            font.pixelSize: Fonts.body
            font.weight: Font.Medium
            font.italic: root.showLyric && PanelState.currentLyric.length > 0
            elide: Text.ElideRight
            width: Math.min(implicitWidth, hoverRow.maxWidth - hoverRow.reservedWidth)
            anchors.verticalCenter: parent.verticalCenter
        }

        // PID
        Text {
            id: pidText

            visible: root.playerPid > 0
            text: root.copied ? "✓ Copied" : "PID " + root.playerPid
            color: root.copied ? Colors.green : Colors.subtext0
            font.family: Fonts.family
            font.pixelSize: Fonts.caption
            font.weight: root.copied ? Font.DemiBold : Font.Normal
            anchors.verticalCenter: parent.verticalCenter

            Behavior on color {
                ColorAnimation { duration: Tokens.animFast }
            }
        }
    }
}
