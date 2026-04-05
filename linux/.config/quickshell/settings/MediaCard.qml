import "../theme"
import QtQuick
import QtQuick.Layouts

// 紧凑媒体播放器卡片 — 可展开显示专辑、进度条
Rectangle {
    id: root

    property var player: null
    property bool expanded: false

    visible: player !== null
    Layout.fillWidth: true
    implicitHeight: mainCol.implicitHeight + 20
    radius: Tokens.radiusM
    color: cardArea.containsMouse
        ? Qt.rgba(Colors.surface1.r, Colors.surface1.g, Colors.surface1.b, Tokens.cardAlpha)
        : Qt.rgba(Colors.surface0.r, Colors.surface0.g, Colors.surface0.b, Tokens.cardAlpha)
    border.color: cardArea.containsMouse
        ? Qt.rgba(Colors.mauve.r, Colors.mauve.g, Colors.mauve.b, Tokens.borderHoverAlpha)
        : Qt.rgba(1, 1, 1, 0.06)
    border.width: 1
    clip: true

    Behavior on implicitHeight {
        NumberAnimation { duration: Tokens.animNormal; easing.type: Easing.OutCubic }
    }

    Behavior on color {
        ColorAnimation { duration: Tokens.animFast; easing.type: Easing.OutCubic }
    }

    Behavior on border.color {
        ColorAnimation { duration: Tokens.animFast; easing.type: Easing.OutCubic }
    }

    MouseArea {
        id: cardArea
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        onClicked: root.expanded = !root.expanded
        z: -1
    }

    ColumnLayout {
        id: mainCol
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.margins: 10
        spacing: Tokens.spaceS

        // ── 紧凑行：封面 + 信息 + 展开按钮 ──
        RowLayout {
            Layout.fillWidth: true
            spacing: 10

            // 封面
            Rectangle {
                Layout.preferredWidth: 48
                Layout.preferredHeight: 48
                radius: Tokens.radiusS
                color: Colors.surface1
                clip: true

                Image {
                    id: coverArt
                    anchors.fill: parent
                    source: root.player ? root.player.trackArtUrl : ""
                    fillMode: Image.PreserveAspectCrop
                    visible: status === Image.Ready
                }

                Text {
                    anchors.centerIn: parent
                    text: "󰎈"
                    color: Colors.overlay1
                    font.family: Fonts.family
                    font.pixelSize: Fonts.h3
                    visible: coverArt.status !== Image.Ready
                }
            }

            // 曲目信息
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 2

                Text {
                    text: root.player ? (root.player.trackTitle || root.player.identity) : ""
                    color: Colors.text
                    font.family: Fonts.family
                    font.pixelSize: Fonts.body
                    font.weight: Font.Bold
                    elide: Text.ElideRight
                    Layout.fillWidth: true
                }

                Text {
                    text: root.player ? (root.player.trackArtist || "") : ""
                    color: Colors.subtext0
                    font.family: Fonts.family
                    font.pixelSize: Fonts.caption
                    elide: Text.ElideRight
                    Layout.fillWidth: true
                    visible: text.length > 0
                }
            }

            Text {
                text: root.expanded ? "󰅃" : "󰅀"
                color: Colors.subtext0
                font.family: Fonts.family
                font.pixelSize: Fonts.icon
            }
        }

        // ── 控制按钮行 ──
        RowLayout {
            Layout.fillWidth: true
            Layout.alignment: Qt.AlignHCenter
            spacing: Tokens.spaceM

            Repeater {
                model: [
                    { "icon": "󰒮", "action": "prev", "size": 18 },
                    { "icon": root.player && root.player.isPlaying ? "󰏤" : "󰐊", "action": "toggle", "size": 24 },
                    { "icon": "󰒭", "action": "next", "size": 18 }
                ]

                delegate: Rectangle {
                    required property var modelData

                    width: 36
                    height: 36
                    radius: Tokens.radiusFull
                    color: mediaBtnArea.containsMouse
                        ? Qt.rgba(Colors.mauve.r, Colors.mauve.g, Colors.mauve.b, 0.15)
                        : "transparent"

                    Text {
                        anchors.centerIn: parent
                        text: modelData.icon
                        color: mediaBtnArea.containsMouse ? Colors.mauve : Colors.text
                        font.family: Fonts.family
                        font.pixelSize: modelData.size

                        Behavior on color { ColorAnimation { duration: Tokens.animFast } }
                    }

                    MouseArea {
                        id: mediaBtnArea
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: {
                            if (!root.player) return;
                            if (modelData.action === "prev")
                                root.player.previous();
                            else if (modelData.action === "toggle")
                                root.player.togglePlaying();
                            else
                                root.player.next();
                        }
                    }

                    Behavior on color { ColorAnimation { duration: Tokens.animFast } }
                }
            }
        }

        // ── 展开区域 ──
        ColumnLayout {
            Layout.fillWidth: true
            spacing: Tokens.spaceS
            visible: root.expanded
            opacity: root.expanded ? 1 : 0

            Behavior on opacity {
                NumberAnimation { duration: Tokens.animNormal; easing.type: Easing.OutCubic }
            }

            // 专辑名
            Text {
                text: root.player ? (root.player.trackAlbum || "") : ""
                color: Colors.subtext0
                font.family: Fonts.family
                font.pixelSize: Fonts.small
                font.italic: true
                elide: Text.ElideRight
                Layout.fillWidth: true
                visible: text.length > 0
            }

            // 进度条
            Rectangle {
                Layout.fillWidth: true
                height: 4
                radius: 2
                color: Colors.surface2

                Rectangle {
                    width: {
                        if (!root.player || !root.player.length || root.player.length <= 0)
                            return 0;
                        return parent.width * Math.min(1, root.player.position / root.player.length);
                    }
                    height: parent.height
                    radius: parent.radius
                    color: Colors.mauve

                    Behavior on width {
                        NumberAnimation { duration: 500; easing.type: Easing.Linear }
                    }
                }
            }

            // 时间
            RowLayout {
                Layout.fillWidth: true

                Text {
                    text: {
                        if (!root.player) return "0:00";
                        let s = Math.floor(root.player.position);
                        let m = Math.floor(s / 60);
                        s = s % 60;
                        return m + ":" + (s < 10 ? "0" : "") + s;
                    }
                    color: Colors.overlay0
                    font.family: Fonts.family
                    font.pixelSize: Fonts.xs
                }

                Item { Layout.fillWidth: true }

                Text {
                    text: {
                        if (!root.player || !root.player.length) return "0:00";
                        let s = Math.floor(root.player.length);
                        let m = Math.floor(s / 60);
                        s = s % 60;
                        return m + ":" + (s < 10 ? "0" : "") + s;
                    }
                    color: Colors.overlay0
                    font.family: Fonts.family
                    font.pixelSize: Fonts.xs
                }
            }

            // 当前歌词
            Text {
                id: cardLyric
                text: PanelState.currentLyric
                color: Colors.mauve
                font.family: Fonts.family
                font.pixelSize: Fonts.small
                font.italic: true
                wrapMode: Text.WordWrap
                horizontalAlignment: Text.AlignHCenter
                Layout.fillWidth: true
                visible: PanelState.currentLyric.length > 0
                opacity: 1

                onTextChanged: {
                    cardLyric.opacity = 0;
                    lyricFadeIn.start();
                }

                NumberAnimation {
                    id: lyricFadeIn
                    target: cardLyric
                    property: "opacity"
                    from: 0; to: 1
                    duration: 250
                    easing.type: Easing.OutCubic
                }
            }

            // 播放器名称
            Text {
                text: root.player ? (root.player.identity || "") : ""
                color: Colors.overlay0
                font.family: Fonts.family
                font.pixelSize: Fonts.xs
                Layout.alignment: Qt.AlignHCenter
            }
        }
    }
}
