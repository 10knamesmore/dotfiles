import "../../theme"
import "../../state"
import "../../services"
import "../../media/capsule"
import "../components"
import QtQuick

// 切歌先显示标题三秒，再跟随同步歌词；悬停保留播放器与音量。
// 右键切换自动歌词 / 固定标题，滚轮调音量，点击音量区静音。
BarModule {
    id: root

    readonly property var player: MediaService.activePlayer
    readonly property string trackKey: MediaService.activeTrackKey
    property bool autoLyrics: true
    readonly property bool showDetails: hovered || volumeFeedback.running
    readonly property real volumeReveal: headerInPanel ? 0 : hoverReveal
    readonly property real coverWidth: coverFrame.visible ? coverFrame.width + 6 : 0
    readonly property string songTitle: {
        if (!player)
            return "暂无媒体播放";
        const title = player.trackTitle || "";
        const artist = player.trackArtist || "";
        return title ? (artist ? title + " - " + artist : title) : player.identity;
    }
    readonly property var currentLine: LyricsState.currentLyricIndex >= 0 ? LyricsState.lyricsLines[LyricsState.currentLyricIndex] : null
    // 展开面板已有完整歌词区，头部保留歌名，避免曲目信息被歌词替掉。
    readonly property bool showingLyrics: autoLyrics && !titleHold.running && !headerInPanel && LyricsState.lyricsTrackId === trackKey && currentLine && currentLine.text.length > 0
    readonly property string fullContent: showingLyrics ? currentLine.text : songTitle

    function showNewTrack() {
        if (player)
            titleHold.restart();
        else
            titleHold.stop();
        console.debug("[media-caption] title hold:", trackKey, songTitle);
    }

    function openMedia() {
        PanelState.closeAll();
        MorphState.openFrom(root, () => PanelState.toggleMedia());
    }

    function isVolumeClick(mouse) {
        const point = volumeText.mapFromItem(root.moduleHeader, mouse.x, mouse.y);
        return volumeText.visible && point.x >= 0 && point.x <= volumeText.width;
    }

    accentColor: Colors.pink
    progress: player && player.lengthSupported && player.length > 0 ? player.position / player.length : -1
    readonly property real compactWidth: Math.max(compactMeasure.implicitWidth + coverWidth + horizontalPadding * 2, 160)
    readonly property real hoverDetailsWidth: (player ? identityText.implicitWidth + 16 : 0) + volumeText.implicitWidth + 6
    // hover 只展开两侧信息，文字视口不变，往返滚动不中断。
    readonly property real compactCaptionWidth: compactWidth - coverWidth - horizontalPadding * 2
    implicitWidth: compactWidth + (showDetails ? hoverDetailsWidth : 0)
    hoverReveal: showDetails ? 1 : 0
    onClicked: mouse => {
        if (root.isVolumeClick(mouse))
            AudioService.toggleMute();
        else
            root.openMedia();
    }
    onRightClicked: root.autoLyrics = !root.autoLyrics
    onScrolled: delta => {
        AudioService.step(delta * 5);
        volumeFeedback.restart();
    }
    onTrackKeyChanged: showNewTrack()
    Component.onCompleted: showNewTrack()

    Accessible.role: Accessible.Button
    Accessible.name: "媒体"
    Accessible.description: root.fullContent
    Accessible.onPressAction: root.openMedia()

    Timer {
        id: titleHold
        interval: 3000
    }
    Timer {
        id: volumeFeedback
        interval: 1400
    }

    Text {
        id: compactMeasure
        visible: false
        // 只用前段文字确定胶囊宽度；显示内容始终保留完整文本。
        text: root.fullContent.substring(0, root.showingLyrics ? 40 : 35)
        textFormat: Text.PlainText
        font.family: Fonts.family
        font.pixelSize: Fonts.body
        font.weight: Font.Medium
    }

    Row {
        id: row
        anchors.left: parent.left
        anchors.verticalCenter: parent.verticalCenter
        width: parent.width
        // 间距与对应内容一起缩到零，避免 visible 切换时 Row 突然撤掉固定间距。
        spacing: 0

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

        Item {
            width: root.player ? 6 * root.detailProgress : 0
            height: 1
        }

        Rectangle {
            id: coverFrame
            visible: coverArt.status === Image.Ready
            width: 28
            height: 28
            radius: 6
            color: Colors.surface1
            clip: true
            anchors.verticalCenter: parent.verticalCenter

            Image {
                id: coverArt
                anchors.fill: parent
                source: root.player ? root.player.trackArtUrl : ""
                sourceSize: Qt.size(64, 64)
                fillMode: Image.PreserveAspectCrop
            }
        }

        Item {
            width: coverFrame.visible ? 6 : 0
            height: 1
        }

        AnimatedCaption {
            width: root.headerInPanel
                ? Math.max(0, row.width - identityTag.width - root.coverWidth - volumeText.width - root.volumeReveal * 6 - root.detailProgress * 6 * (root.player ? 1 : 0))
                : root.compactCaptionWidth
            height: 28
            anchors.verticalCenter: parent.verticalCenter
            textColor: root.detailProgress > 0 ? Colors.pink : Colors.text
            caption: ({
                    key: root.trackKey + (root.showingLyrics ? ":lyric:" + LyricsState.currentLyricIndex : ":title:" + root.songTitle),
                    text: root.fullContent,
                    lyric: root.showingLyrics,
                    words: root.showingLyrics ? root.currentLine.words : []
                })
        }

        Item {
            width: 6 * root.volumeReveal
            height: 1
        }

        Text {
            id: volumeText
            visible: root.volumeReveal > 0
            width: implicitWidth * root.volumeReveal
            opacity: root.volumeReveal
            clip: true
            text: (AudioService.muted ? "󰝟" : (AudioService.volume < 50 ? "󰖀" : "󰕾")) + " " + (AudioService.muted ? "静音" : AudioService.volume + "%")
            color: AudioService.muted ? Colors.overlay1 : Colors.peach
            font.family: Fonts.family
            font.pixelSize: Fonts.small
            anchors.verticalCenter: parent.verticalCenter
            Accessible.role: Accessible.Button
            Accessible.name: "静音"
            Accessible.checkable: true
            Accessible.checked: AudioService.muted
            Accessible.onPressAction: AudioService.toggleMute()
        }
    }
}
