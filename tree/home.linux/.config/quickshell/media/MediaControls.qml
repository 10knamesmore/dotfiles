import "../theme"
import "../components"
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Quickshell.Services.Mpris

// 固定在双栏下方的播放控制；拖动进度由面板统一写入播放器。
ColumnLayout {
    id: root

    required property var player

    signal seekRequested(real seconds)

    function formatTime(seconds) {
        const total = Math.max(0, Math.floor(seconds));
        return Math.floor(total / 60) + ":" + (total % 60).toString().padStart(2, "0");
    }

    function togglePlaying() {
        if (!player || !player.canTogglePlaying)
            return ;

        console.info("[media-panel] toggle playback:", player.dbusName);
        player.togglePlaying();
    }

    spacing: 6

    ColumnLayout {
        Layout.fillWidth: true
        Layout.preferredHeight: 32
        spacing: 0

        Slider {
            id: progress

            Layout.fillWidth: true
            Layout.preferredHeight: 20
            from: 0
            to: root.player && root.player.length > 0 ? root.player.length : 1
            value: root.player ? root.player.position : 0
            enabled: root.player !== null && root.player.canSeek && root.player.lengthSupported && root.player.length > 0
            hoverEnabled: true
            padding: 0
            leftPadding: 5
            rightPadding: 5
            Accessible.name: "播放进度"
            onMoved: root.seekRequested(value)
            ToolTip.visible: pressed
            ToolTip.text: root.formatTime(value)

            background: Rectangle {
                x: progress.leftPadding
                y: (progress.height - height) / 2
                width: progress.availableWidth
                height: progress.hovered || progress.pressed ? 5 : 3
                radius: height / 2
                color: Colors.overlay(0.1)

                Rectangle {
                    width: progress.visualPosition * parent.width
                    height: parent.height
                    radius: parent.radius
                    color: Colors.mauve
                }

                Behavior on height {
                    NumberAnimation {
                        duration: Tokens.animFast
                    }

                }

            }

            handle: Rectangle {
                x: progress.leftPadding + progress.visualPosition * progress.availableWidth - width / 2
                y: (progress.height - height) / 2
                implicitWidth: 10
                implicitHeight: 10
                radius: 5
                color: Colors.mauve
                opacity: progress.enabled && (progress.hovered || progress.pressed || progress.visualFocus) ? 1 : 0
                scale: progress.pressed ? 1.2 : 1

                SliderGlow {
                    anchors.centerIn: parent
                    pressed: progress.pressed
                    accentColor: Colors.mauve
                }

                Behavior on opacity {
                    NumberAnimation {
                        duration: Tokens.animFast
                    }

                }

                Behavior on scale {
                    NumberAnimation {
                        duration: Tokens.animFast
                    }

                }

            }

        }

        RowLayout {
            Layout.fillWidth: true
            Layout.leftMargin: 5
            Layout.rightMargin: 5

            Text {
                text: root.player && root.player.positionSupported ? root.formatTime(progress.pressed ? progress.value : root.player.position) : "—:—"
                color: Colors.overlay2
                font.family: Fonts.family
                font.pixelSize: Fonts.xs
            }

            Item {
                Layout.fillWidth: true
            }

            Text {
                text: root.player && root.player.lengthSupported ? root.formatTime(root.player.length) : "—:—"
                color: Colors.overlay2
                font.family: Fonts.family
                font.pixelSize: Fonts.xs
            }

        }

    }

    RowLayout {
        Layout.alignment: Qt.AlignHCenter
        spacing: 18

        MediaButton {
            text: "󰒟"
            label: "随机播放"
            enabled: root.player !== null && root.player.shuffleSupported && root.player.canControl
            checkable: true
            checked: root.player !== null && root.player.shuffle
            onClicked: {
                root.player.shuffle = checked;
                console.info("[media-panel] shuffle:", checked);
            }
        }

        MediaButton {
            text: "󰒮"
            label: "上一首"
            enabled: root.player !== null && root.player.canGoPrevious
            onClicked: {
                console.info("[media-panel] previous:", root.player.dbusName);
                root.player.previous();
            }
        }

        MediaButton {
            primary: true
            text: root.player && root.player.isPlaying ? "󰏤" : "󰐊"
            label: root.player && root.player.isPlaying ? "暂停" : "播放"
            enabled: root.player !== null && root.player.canTogglePlaying
            onClicked: root.togglePlaying()
        }

        MediaButton {
            text: "󰒭"
            label: "下一首"
            enabled: root.player !== null && root.player.canGoNext
            onClicked: {
                console.info("[media-panel] next:", root.player.dbusName);
                root.player.next();
            }
        }

        MediaButton {
            text: root.player && root.player.loopState === MprisLoopState.Track ? "󰑘" : "󰑖"
            label: "循环播放"
            Accessible.description: !root.player || root.player.loopState === MprisLoopState.None ? "关闭" : root.player.loopState === MprisLoopState.Track ? "单曲循环" : "列表循环"
            enabled: root.player !== null && root.player.loopSupported && root.player.canControl
            selected: root.player !== null && root.player.loopState !== MprisLoopState.None
            onClicked: {
                switch (root.player.loopState) {
                case MprisLoopState.None:
                    root.player.loopState = MprisLoopState.Playlist;
                    break;
                case MprisLoopState.Playlist:
                    root.player.loopState = MprisLoopState.Track;
                    break;
                default:
                    root.player.loopState = MprisLoopState.None;
                }
                console.info("[media-panel] loop:", root.player.loopState);
            }
        }

    }

}
