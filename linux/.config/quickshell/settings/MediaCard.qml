import QtQuick
import QtQuick.Layouts
import Quickshell.Services.Mpris
import "../theme"

// 紧凑媒体播放器卡片 — 仅在有播放器时可见
InfoCard {
    id: root

    property var player: Mpris.players.count > 0 ? Mpris.players.values[0] : null
    visible: player !== null

    contentItem: RowLayout {
        spacing: 10

        // 封面
        Rectangle {
            Layout.preferredWidth: 48
            Layout.preferredHeight: 48
            radius: 8
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
                font.family: "Hack Nerd Font"
                font.pixelSize: 20
                visible: coverArt.status !== Image.Ready
            }
        }

        // 曲目信息 + 控制
        ColumnLayout {
            Layout.fillWidth: true
            spacing: 4

            Text {
                text: root.player ? (root.player.trackTitle || root.player.identity) : ""
                color: Colors.text
                font.family: "Hack Nerd Font"
                font.pixelSize: 12
                font.weight: Font.Bold
                elide: Text.ElideRight
                Layout.fillWidth: true
            }
            Text {
                text: root.player ? (root.player.trackArtist || "") : ""
                color: Colors.subtext0
                font.family: "Hack Nerd Font"
                font.pixelSize: 10
                elide: Text.ElideRight
                Layout.fillWidth: true
                visible: text.length > 0
            }

            // 控制按钮
            RowLayout {
                spacing: 4

                Repeater {
                    model: [
                        { icon: "󰒮", action: "prev" },
                        { icon: root.player && root.player.isPlaying ? "󰏤" : "󰐊", action: "toggle" },
                        { icon: "󰒭", action: "next" }
                    ]
                    delegate: Rectangle {
                        required property var modelData
                        width: 28; height: 28; radius: 14
                        color: btnArea.containsMouse ? Colors.surface1 : "transparent"
                        Behavior on color { ColorAnimation { duration: 150 } }

                        Text {
                            anchors.centerIn: parent
                            text: modelData.icon
                            color: Colors.text
                            font.family: "Hack Nerd Font"
                            font.pixelSize: modelData.action === "toggle" ? 18 : 14
                        }
                        MouseArea {
                            id: btnArea
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: {
                                if (!root.player) return;
                                if (modelData.action === "prev") root.player.previous();
                                else if (modelData.action === "toggle") root.player.togglePlaying();
                                else root.player.next();
                            }
                        }
                    }
                }
            }
        }
    }
}
