import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Wayland
import Quickshell.Services.Mpris
import "../theme"

// 媒体控制面板 — 显示封面、曲目信息、进度条、控制按钮
PanelWindow {
    id: root

    anchors.top: true
    anchors.bottom: true
    anchors.left: true
    anchors.right: true

    // 双阶段可见性
    property bool showing: PanelState.mediaOpen
    property bool animating: _opacityAnim.running || _slideAnim.running
    visible: showing || animating

    focusable: false
    exclusionMode: ExclusionMode.Ignore
    color: "transparent"

    property var player: Mpris.players.count > 0 ? Mpris.players.values[0] : null

    // 半透明遮罩
    Rectangle {
        anchors.fill: parent
        color: "#000000"
        opacity: root.showing ? 0.15 : 0.0
        Behavior on opacity { NumberAnimation { duration: 250; easing.type: Easing.OutCubic } }
    }

    // 点击面板外部关闭
    MouseArea {
        anchors.fill: parent
        onClicked: PanelState.mediaOpen = false
    }

    Rectangle {
        id: panel
        width: 360
        height: col.implicitHeight + 32
        radius: 16
        color: Qt.rgba(Colors.surface0.r, Colors.surface0.g, Colors.surface0.b, 0.85)
        border.color: Qt.rgba(Colors.surface1.r, Colors.surface1.g, Colors.surface1.b, 0.9)
        border.width: 1
        anchors.top: parent.top
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.topMargin: root.showing ? 54 : 34

        opacity: root.showing ? 1.0 : 0.0

        Behavior on anchors.topMargin {
            NumberAnimation { id: _slideAnim; duration: 250; easing.type: Easing.OutCubic }
        }
        Behavior on opacity {
            NumberAnimation { id: _opacityAnim; duration: 250; easing.type: Easing.OutCubic }
        }

        MouseArea {
            anchors.fill: parent
            onClicked: mouse => mouse.accepted = true
        }

        ColumnLayout {
            id: col
            anchors.fill: parent
            anchors.margins: 16
            spacing: 10

            // ── 封面 ──
            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: 200
                radius: 12
                color: Colors.surface1
                clip: true

                Image {
                    id: coverArt
                    anchors.fill: parent
                    source: root.player && root.player.trackArtUrl ? root.player.trackArtUrl : ""
                    fillMode: Image.PreserveAspectCrop
                    visible: status === Image.Ready
                }

                Text {
                    anchors.centerIn: parent
                    text: "󰎆"
                    font.pixelSize: 48
                    color: Colors.overlay1
                    visible: !coverArt.visible
                }
            }

            // ── 曲目信息 ──
            Text {
                Layout.fillWidth: true
                text: root.player ? (root.player.trackTitle || "未在播放") : "无播放器"
                color: Colors.text
                font.family: "Hack Nerd Font"
                font.pixelSize: 14
                font.weight: Font.Bold
                elide: Text.ElideRight
                horizontalAlignment: Text.AlignHCenter
            }

            Text {
                Layout.fillWidth: true
                text: {
                    if (!root.player) return ""
                    let parts = []
                    if (root.player.trackArtist) parts.push(root.player.trackArtist)
                    if (root.player.trackAlbum)  parts.push(root.player.trackAlbum)
                    return parts.join(" — ")
                }
                color: Colors.subtext0
                font.family: "Hack Nerd Font"
                font.pixelSize: 11
                elide: Text.ElideRight
                horizontalAlignment: Text.AlignHCenter
            }

            // ── 进度条 ──
            Item {
                Layout.fillWidth: true
                Layout.preferredHeight: 24
                visible: root.player && root.player.lengthSupported

                Rectangle {
                    id: progressBg
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.verticalCenter: parent.verticalCenter
                    height: 4
                    radius: 2
                    color: Colors.surface1

                    Rectangle {
                        width: {
                            if (!root.player || root.player.length <= 0) return 0
                            return Math.min(1, root.player.position / root.player.length) * parent.width
                        }
                        height: parent.height
                        radius: parent.radius
                        color: Colors.pink

                        Behavior on width { NumberAnimation { duration: 500 } }
                    }
                }

                // 进度时间
                RowLayout {
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.top: progressBg.bottom
                    anchors.topMargin: 2

                    Text {
                        text: root.player ? formatTime(root.player.position) : "0:00"
                        color: Colors.subtext0
                        font.family: "Hack Nerd Font"
                        font.pixelSize: 10
                    }
                    Item { Layout.fillWidth: true }
                    Text {
                        text: root.player ? formatTime(root.player.length) : "0:00"
                        color: Colors.subtext0
                        font.family: "Hack Nerd Font"
                        font.pixelSize: 10
                    }
                }
            }

            // ── 控制按钮 ──
            RowLayout {
                Layout.alignment: Qt.AlignHCenter
                spacing: 20

                // Shuffle
                MediaButton {
                    text: "󰒟"
                    active: root.player && root.player.shuffle
                    visible: root.player && root.player.shuffleSupported
                    onClicked: if (root.player) root.player.shuffle = !root.player.shuffle
                }

                // Previous
                MediaButton {
                    text: "󰒮"
                    enabled: root.player && root.player.canGoPrevious
                    onClicked: if (root.player) root.player.previous()
                }

                // Play/Pause (大按钮)
                Rectangle {
                    width: 48
                    height: 48
                    radius: 24
                    color: playPauseArea.containsMouse ? Colors.pink : Colors.surface1

                    Behavior on color { ColorAnimation { duration: 200 } }

                    Text {
                        anchors.centerIn: parent
                        text: root.player && root.player.isPlaying ? "󰏤" : "󰐊"
                        color: playPauseArea.containsMouse ? Colors.base : Colors.text
                        font.family: "Hack Nerd Font"
                        font.pixelSize: 22
                    }

                    MouseArea {
                        id: playPauseArea
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: if (root.player) root.player.togglePlaying()
                    }
                }

                // Next
                MediaButton {
                    text: "󰒭"
                    enabled: root.player && root.player.canGoNext
                    onClicked: if (root.player) root.player.next()
                }

                // Loop
                MediaButton {
                    text: {
                        if (!root.player) return "󰑗"
                        switch (root.player.loopState) {
                            case MprisLoopState.Track: return "󰑘"
                            case MprisLoopState.Playlist: return "󰑖"
                            default: return "󰑗"
                        }
                    }
                    active: root.player && root.player.loopState !== MprisLoopState.None
                    visible: root.player && root.player.loopSupported
                    onClicked: {
                        if (!root.player) return
                        // None -> Playlist -> Track -> None
                        switch (root.player.loopState) {
                            case MprisLoopState.None:
                                root.player.loopState = MprisLoopState.Playlist; break
                            case MprisLoopState.Playlist:
                                root.player.loopState = MprisLoopState.Track; break
                            default:
                                root.player.loopState = MprisLoopState.None; break
                        }
                    }
                }
            }

            // ── 音量滑块 ──
            RowLayout {
                Layout.fillWidth: true
                visible: root.player && root.player.volumeSupported
                spacing: 8

                Text {
                    text: "󰕾"
                    color: Colors.subtext0
                    font.family: "Hack Nerd Font"
                    font.pixelSize: 14
                }

                Rectangle {
                    Layout.fillWidth: true
                    height: 4
                    radius: 2
                    color: Colors.surface1

                    Rectangle {
                        width: root.player ? Math.min(1, root.player.volume) * parent.width : 0
                        height: parent.height
                        radius: parent.radius
                        color: Colors.pink
                    }

                    MouseArea {
                        anchors.fill: parent
                        onClicked: mouse => {
                            if (root.player) {
                                root.player.volume = mouse.x / parent.width
                            }
                        }
                    }
                }

                Text {
                    text: root.player ? Math.round(root.player.volume * 100) + "%" : "0%"
                    color: Colors.subtext0
                    font.family: "Hack Nerd Font"
                    font.pixelSize: 10
                    Layout.preferredWidth: 32
                    horizontalAlignment: Text.AlignRight
                }
            }
        }
    }

    // ── 位置更新定时器 ──
    Timer {
        interval: 1000
        running: root.visible && root.player && root.player.isPlaying
        repeat: true
        onTriggered: {} // 触发 position 重新绑定
    }

    function formatTime(seconds) {
        if (!seconds || seconds < 0) return "0:00"
        let m = Math.floor(seconds / 60)
        let s = Math.floor(seconds % 60)
        return m + ":" + (s < 10 ? "0" : "") + s
    }

    // 可复用的小控制按钮
    component MediaButton: Rectangle {
        property string text: ""
        property bool active: false
        signal clicked()

        width: 32
        height: 32
        radius: 16
        color: btnArea.containsMouse ? Colors.surface1 : "transparent"

        Behavior on color { ColorAnimation { duration: 150 } }

        Text {
            anchors.centerIn: parent
            text: parent.text
            color: parent.active ? Colors.pink : Colors.subtext0
            font.family: "Hack Nerd Font"
            font.pixelSize: 16
        }

        MouseArea {
            id: btnArea
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: parent.clicked()
        }
    }
}
