import "../components"
import "../services"
import "../state"
import "../theme"
import QtQuick
import QtQuick.Layouts

// 打开即显示封面与歌词双栏；播放控制固定在下方，不提供独立歌词展开模式。
PanelOverlay {
    id: root

    readonly property var player: MediaService.activePlayer
    readonly property int coverSize: 216
    readonly property bool canSeek: player !== null && player.canSeek && player.lengthSupported && player.length > 0

    function seekTo(seconds) {
        if (!canSeek)
            return ;

        const target = Math.max(0, Math.min(player.length, seconds));
        console.debug("[media-panel] seek:", player.dbusName, target);
        player.position = target;
        lyrics.followCurrent();
    }

    showing: PanelState.mediaOpen
    panelWidth: 800
    panelHeight: 480
    onCloseRequested: PanelState.mediaOpen = false
    onShowingChanged: {
        if (showing)
            console.info("[media-panel] opened:", MediaService.activeTrackKey);

    }

    FocusScope {
        anchors.fill: parent
        focus: root.showing
        Keys.onSpacePressed: controls.togglePlaying()

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: Tokens.spaceXL
            spacing: 18

            RowLayout {
                Layout.fillWidth: true
                Layout.fillHeight: true
                spacing: 28

                ColumnLayout {
                    Layout.minimumWidth: root.coverSize
                    Layout.maximumWidth: root.coverSize
                    Layout.preferredWidth: root.coverSize
                    Layout.fillHeight: true
                    spacing: 14

                    Rectangle {
                        Layout.fillWidth: true
                        Layout.minimumHeight: root.coverSize
                        Layout.maximumHeight: root.coverSize
                        Layout.preferredHeight: root.coverSize
                        radius: Tokens.radiusM
                        color: Colors.base
                        clip: true

                        Image {
                            id: coverArt

                            anchors.fill: parent
                            source: root.player ? root.player.trackArtUrl : ""
                            sourceSize: Qt.size(root.coverSize * 2, root.coverSize * 2)
                            fillMode: Image.PreserveAspectFit
                            asynchronous: true
                            opacity: status === Image.Ready ? 1 : 0
                            Accessible.ignored: true

                            Behavior on opacity {
                                NumberAnimation {
                                    duration: 240
                                }

                            }

                        }

                        Text {
                            anchors.centerIn: parent
                            visible: coverArt.status !== Image.Ready
                            text: "󰎆"
                            color: Colors.overlay1
                            font.family: Fonts.family
                            font.pixelSize: Fonts.display2
                            Accessible.ignored: true
                        }

                    }

                    ColumnLayout {
                        id: trackInfo

                        Layout.fillWidth: true
                        spacing: 5

                        Item {
                            id: titleViewport

                            Layout.fillWidth: true
                            implicitHeight: titleText.implicitHeight
                            clip: true

                            Text {
                                id: titleText

                                x: -titleScroll.offset
                                text: root.player ? root.player.trackTitle || "未在播放" : "无播放器"
                                textFormat: Text.PlainText
                                color: Colors.text
                                font.family: Fonts.family
                                font.pixelSize: Fonts.title
                                font.weight: Font.DemiBold
                                onTextChanged: Qt.callLater(() => {
                                    if (titleScroll.running)
                                        titleScroll.restart();

                                })
                            }

                            TextScrollAnimation {
                                id: titleScroll

                                distance: Math.max(0, titleText.implicitWidth - titleViewport.width)
                                running: root.showing && distance > 0
                            }

                        }

                        Text {
                            Layout.fillWidth: true
                            text: root.player ? root.player.trackArtist : ""
                            textFormat: Text.PlainText
                            visible: text.length > 0
                            color: Colors.subtext0
                            font.family: Fonts.family
                            font.pixelSize: Fonts.small
                            elide: Text.ElideRight
                        }

                        Text {
                            Layout.fillWidth: true
                            text: root.player ? root.player.trackAlbum : ""
                            textFormat: Text.PlainText
                            visible: text.length > 0
                            color: Colors.overlay2
                            font.family: Fonts.family
                            font.pixelSize: Fonts.caption
                            elide: Text.ElideRight
                        }

                        transform: Translate {
                            id: trackShift
                        }

                    }

                    Item {
                        Layout.fillHeight: true
                    }

                }

                MediaLyrics {
                    id: lyrics

                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    active: root.showing
                    canSeek: root.canSeek
                    onSeekRequested: (seconds) => {
                        return root.seekTo(seconds);
                    }
                }

            }

            MediaControls {
                id: controls

                Layout.fillWidth: true
                player: root.player
                onSeekRequested: (seconds) => {
                    return root.seekTo(seconds);
                }
            }

        }

    }

    Connections {
        function onActiveTrackKeyChanged() {
            if (root.showing)
                trackTransition.restart();

        }

        target: MediaService
    }

    ParallelAnimation {
        id: trackTransition

        NumberAnimation {
            target: trackInfo
            property: "opacity"
            from: 0
            to: 1
            duration: 240
        }

        NumberAnimation {
            target: trackShift
            property: "y"
            from: 5
            to: 0
            duration: 240
            easing.type: Easing.OutCubic
        }

    }

}
