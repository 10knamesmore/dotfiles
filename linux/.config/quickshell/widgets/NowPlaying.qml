import "../theme"
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Services.Mpris
import Quickshell.Wayland

// 桌面浮动 Now Playing — 专辑封面 + 曲目信息 + 歌词 + 控制
PanelWindow {
    id: root

    required property var modelData
    screen: modelData

    // ── 布局配置（改这里调整位置和大小）──
    property int widgetMarginRight: 20   // 右边距
    property int widgetMarginBottom: 120  // 下边距

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

    function formatTime(seconds) {
        if (!seconds || seconds < 0) return "0:00";
        let m = Math.floor(seconds / 60);
        let s = Math.floor(seconds % 60);
        return m + ":" + (s < 10 ? "0" : "") + s;
    }

    aboveWindows: false
    anchors.bottom: true
    anchors.right: true
    implicitWidth: 380
    implicitHeight: 180
    margins.bottom: widgetMarginBottom
    margins.right: widgetMarginRight
    visible: PanelState.nowPlayingVisible && root.player !== null
    focusable: false
    exclusionMode: ExclusionMode.Ignore
    color: "transparent"

    // 位置刷新
    Timer {
        interval: 1000
        running: root.visible && root.player && root.player.isPlaying
        repeat: true
        onTriggered: {} // 触发 position 绑定更新
    }

    // ── 主体 ──
    Rectangle {
        id: card
        anchors.centerIn: parent
        width: 360
        height: 150
        radius: Tokens.radiusXL
        color: Qt.rgba(Colors.base.r, Colors.base.g, Colors.base.b, Tokens.panelAlpha)
        border.color: Qt.rgba(Tokens.borderBase.r, Tokens.borderBase.g, Tokens.borderBase.b, Tokens.borderAlpha)
        border.width: 1
        clip: true

        SoftShadow { anchors.fill: parent; radius: parent.radius }
        InnerGlow {}

        // 专辑封面色调背景
        Image {
            anchors.fill: parent
            source: root.player && root.player.trackArtUrl ? root.player.trackArtUrl : ""
            fillMode: Image.PreserveAspectCrop
            visible: status === Image.Ready
            opacity: 0.08
        }

        RowLayout {
            anchors.fill: parent
            anchors.margins: 10
            spacing: 12

            // ── 封面 ──
            Rectangle {
                Layout.preferredWidth: 120
                Layout.preferredHeight: 120
                radius: Tokens.radiusM
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
                    text: "\udb80\udf86"
                    font.family: Fonts.family
                    font.pixelSize: Fonts.display1
                    color: Colors.overlay1
                    visible: !coverArt.visible
                }
            }

            // ── 信息区 ──
            ColumnLayout {
                Layout.fillWidth: true
                Layout.fillHeight: true
                spacing: 3

                // 曲名
                Text {
                    Layout.fillWidth: true
                    text: root.player ? (root.player.trackTitle || "未在播放") : ""
                    color: Colors.text
                    font.family: Fonts.family
                    font.pixelSize: Fonts.bodyLarge
                    font.weight: Font.Bold
                    elide: Text.ElideRight
                    maximumLineCount: 1
                }

                // 歌手
                Text {
                    Layout.fillWidth: true
                    text: root.player ? (root.player.trackArtist || "") : ""
                    visible: text.length > 0
                    color: Colors.subtext0
                    font.family: Fonts.family
                    font.pixelSize: Fonts.small
                    elide: Text.ElideRight
                    maximumLineCount: 1
                }

                // 歌词行
                Text {
                    Layout.fillWidth: true
                    text: PanelState.currentLyric || ""
                    visible: text.length > 0
                    color: Colors.mauve
                    font.family: Fonts.family
                    font.pixelSize: Fonts.caption
                    font.italic: true
                    elide: Text.ElideRight
                    maximumLineCount: 1
                    opacity: text.length > 0 ? 1 : 0

                    Behavior on opacity {
                        NumberAnimation { duration: Tokens.animNormal }
                    }
                    Behavior on text {
                        // 文字变化时有微小的淡入效果通过 SequentialAnimation
                    }
                }

                Item { Layout.fillHeight: true }

                // 进度条
                Item {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 16
                    visible: root.player && root.player.lengthSupported

                    Rectangle {
                        id: progressBar
                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.verticalCenter: parent.verticalCenter
                        height: 3
                        radius: 1.5
                        color: Colors.surface1

                        Rectangle {
                            width: {
                                if (!root.player || root.player.length <= 0) return 0;
                                return Math.min(1, root.player.position / root.player.length) * parent.width;
                            }
                            height: parent.height
                            radius: parent.radius
                            color: Colors.pink

                            Behavior on width {
                                NumberAnimation { duration: 500 }
                            }
                        }
                    }

                    RowLayout {
                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.top: progressBar.bottom
                        anchors.topMargin: 1

                        Text {
                            text: root.player ? formatTime(root.player.position) : ""
                            color: Colors.overlay0
                            font.family: Fonts.family
                            font.pixelSize: 8
                        }
                        Item { Layout.fillWidth: true }
                        Text {
                            text: root.player ? formatTime(root.player.length) : ""
                            color: Colors.overlay0
                            font.family: Fonts.family
                            font.pixelSize: 8
                        }
                    }
                }

                // 控制按钮
                RowLayout {
                    Layout.alignment: Qt.AlignHCenter
                    spacing: 12

                    MediaBtn {
                        text: "\udb80\udcae"
                        enabled: root.player && root.player.canGoPrevious
                        onClicked: { if (root.player) root.player.previous(); }
                    }

                    // Play/Pause
                    Rectangle {
                        width: 34
                        height: 34
                        radius: 17
                        color: ppArea.containsMouse ? Colors.pink : Colors.surface1

                        Text {
                            anchors.centerIn: parent
                            text: root.player && root.player.isPlaying ? "\udb81\udc24" : "\udb80\udc4a"
                            color: ppArea.containsMouse ? Colors.base : Colors.text
                            font.family: Fonts.family
                            font.pixelSize: Fonts.heading
                        }

                        MouseArea {
                            id: ppArea
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: { if (root.player) root.player.togglePlaying(); }
                        }

                        Behavior on color {
                            ColorAnimation { duration: Tokens.animFast }
                        }
                    }

                    MediaBtn {
                        text: "\udb80\udcad"
                        enabled: root.player && root.player.canGoNext
                        onClicked: { if (root.player) root.player.next(); }
                    }
                }
            }
        }
    }

    // 可复用小按钮
    component MediaBtn: Rectangle {
        property string text: ""
        signal clicked

        width: 28
        height: 28
        radius: Tokens.radiusL
        color: btnArea.containsMouse ? Colors.surface1 : "transparent"

        Text {
            anchors.centerIn: parent
            text: parent.text
            color: Colors.subtext0
            font.family: Fonts.family
            font.pixelSize: Fonts.icon
        }

        MouseArea {
            id: btnArea
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: parent.clicked()
        }

        Behavior on color {
            ColorAnimation { duration: Tokens.animFast }
        }
    }
}
