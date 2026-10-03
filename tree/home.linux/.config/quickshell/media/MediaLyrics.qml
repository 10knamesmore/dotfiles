import "../state"
import "../theme"
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

// 歌词独立滚动；手动浏览后保持位置，直到点击「回到当前」或重新打开面板。
ColumnLayout {
    id: root

    required property bool active
    required property bool canSeek
    property bool autoFollow: true
    readonly property bool hasLyrics: LyricsState.lyricsLines.length > 0

    signal seekRequested(real seconds)

    function scrollToCurrent(animated) {
        if (!active || !hasLyrics || !autoFollow)
            return ;

        scrollMotion.stop();
        const before = lyricsView.contentY;
        lyricsView.positionViewAtIndex(Math.max(0, LyricsState.currentLyricIndex), ListView.Center);
        const target = lyricsView.contentY;
        if (animated && Math.abs(target - before) > 1) {
            // positionViewAtIndex 不经过 QML Behavior，先取目标再显式滚动。
            lyricsView.contentY = before;
            scrollMotion.from = before;
            scrollMotion.to = target;
            scrollMotion.start();
        }
    }

    function followCurrent() {
        autoFollow = true;
        scrollToCurrent(true);
    }

    spacing: 4
    onActiveChanged: {
        if (active) {
            autoFollow = true;
            Qt.callLater(() => {
                return scrollToCurrent(false);
            });
        } else {
            scrollMotion.stop();
            settings.close();
        }
    }

    RowLayout {
        Layout.fillWidth: true
        Layout.preferredHeight: 28
        spacing: 6

        Text {
            text: "歌词"
            color: Colors.overlay2
            font.family: Fonts.family
            font.pixelSize: Fonts.xs
        }

        Item {
            Layout.fillWidth: true
        }

        Button {
            id: returnToCurrent

            visible: root.hasLyrics && !root.autoFollow
            text: "回到当前"
            implicitHeight: 26
            horizontalPadding: 8
            onClicked: {
                console.debug("[media-panel] resume lyrics follow");
                root.followCurrent();
            }

            background: Rectangle {
                radius: Tokens.radiusS
                color: returnToCurrent.hovered || returnToCurrent.visualFocus ? Colors.overlay(0.07) : "transparent"
            }

            contentItem: Text {
                text: returnToCurrent.text
                font.family: Fonts.family
                font.pixelSize: Fonts.xs
                color: Colors.mauve
                verticalAlignment: Text.AlignVCenter
            }

        }

        MediaButton {
            text: "󰇙"
            label: "歌词设置"
            implicitHeight: 28
            enabled: root.hasLyrics
            selected: settings.opened
            Accessible.description: settings.opened ? "已展开" : "已收起"
            onClicked: settings.opened ? settings.close() : settings.open()
        }

    }

    Item {
        Layout.fillWidth: true
        Layout.fillHeight: true
        clip: true

        ListView {
            id: lyricsView

            anchors.fill: parent
            visible: root.hasLyrics
            model: LyricsState.lyricsLines
            clip: true
            spacing: 4
            boundsBehavior: Flickable.StopAtBounds
            highlightFollowsCurrentItem: false
            onHeightChanged: Qt.callLater(() => {
                return root.scrollToCurrent(false);
            })
            onMovementStarted: {
                root.autoFollow = false;
                scrollMotion.stop();
                console.debug("[media-panel] browsing lyrics");
            }

            header: Item {
                height: Math.max(0, (lyricsView.height - 76) / 2)
            }

            footer: Item {
                height: Math.max(0, (lyricsView.height - 76) / 2)
            }

            delegate: LyricLine {
                required property var modelData
                required property int index

                width: lyricsView.width
                line: modelData
                current: index === LyricsState.currentLyricIndex
                playbackTimeMs: root.active && current ? LyricsState.currentTimeMs : 0
                showTranslation: LyricsState.showTranslation
                browsing: !root.autoFollow
                distance: Math.abs(index - LyricsState.currentLyricIndex)
                enabled: root.canSeek
                onClicked: {
                    root.autoFollow = true;
                    root.seekRequested(Math.max(0, modelData.time - LyricsState.lyricsOffset));
                }
            }

        }

        ColumnLayout {
            anchors.centerIn: parent
            width: parent.width
            visible: !root.hasLyrics
            spacing: 12

            Text {
                Layout.alignment: Qt.AlignHCenter
                text: "󰎆"
                color: Colors.overlay0
                font.family: Fonts.family
                font.pixelSize: Fonts.h2
            }

            Text {
                Layout.alignment: Qt.AlignHCenter
                text: "暂无歌词"
                color: Colors.overlay2
                font.family: Fonts.family
                font.pixelSize: Fonts.small
            }

        }

    }

    LyricsSettings {
        id: settings

        parent: root
        x: root.width - width
        y: 32
    }

    NumberAnimation {
        id: scrollMotion

        target: lyricsView
        property: "contentY"
        duration: 420
        easing.type: Easing.OutCubic
    }

    Connections {
        function onCurrentLyricIndexChanged() {
            root.scrollToCurrent(true);
        }

        function onLyricsLinesChanged() {
            root.autoFollow = true;
            Qt.callLater(() => {
                return root.scrollToCurrent(false);
            });
        }

        function onShowTranslationChanged() {
            Qt.callLater(() => {
                return root.scrollToCurrent(false);
            });
        }

        target: LyricsState
    }

}
