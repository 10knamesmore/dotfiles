import "../theme"
import QtQuick
import QtQuick.Layouts

// 会话历史侧边栏 — 面板内覆盖层
Rectangle {
    id: drawer

    property var sessions: []
    property string currentSessionId: ""

    signal sessionSelected(string sessionId)
    signal newSessionRequested()
    signal sessionDeleted(string sessionId)
    signal closeRequested()

    color: Qt.rgba(Colors.base.r, Colors.base.g, Colors.base.b, 0.95)
    radius: Tokens.radiusL

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: Tokens.spaceL
        spacing: Tokens.spaceS

        // ── Header ──
        RowLayout {
            Layout.fillWidth: true

            Text {
                text: "󰋚 会话历史"
                font.family: Fonts.family
                font.pixelSize: Fonts.title
                font.bold: true
                color: Colors.text
            }

            Item { Layout.fillWidth: true }

            Rectangle {
                width: 28
                height: 28
                radius: Tokens.radiusFull
                color: closeArea.containsMouse ? Qt.rgba(1, 1, 1, 0.08) : "transparent"

                Text {
                    anchors.centerIn: parent
                    text: "󰅖"
                    color: closeArea.containsMouse ? Colors.text : Colors.subtext0
                    font.family: Fonts.family
                    font.pixelSize: Fonts.iconLarge
                }

                MouseArea {
                    id: closeArea
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: drawer.closeRequested()
                }
            }
        }

        Rectangle {
            Layout.fillWidth: true
            height: 1
            color: Colors.surface1
        }

        // ── 会话列表 ──
        ListView {
            id: sessionList

            Layout.fillWidth: true
            Layout.fillHeight: true
            model: drawer.sessions
            spacing: 4
            clip: true

            Text {
                anchors.centerIn: parent
                visible: drawer.sessions.length === 0
                text: "暂无会话"
                color: Colors.overlay0
                font.family: Fonts.family
                font.pixelSize: Fonts.body
            }

            delegate: Rectangle {
                required property int index
                required property var modelData

                width: sessionList.width
                height: 52
                radius: Tokens.radiusMS
                color: {
                    if (modelData.session_id === drawer.currentSessionId)
                        return Qt.rgba(Colors.blue.r, Colors.blue.g, Colors.blue.b, 0.15);
                    return rowHover.hovered ? Qt.rgba(1, 1, 1, 0.06) : "transparent";
                }

                HoverHandler {
                    id: rowHover
                }

                ColumnLayout {
                    anchors.fill: parent
                    anchors.leftMargin: 12
                    anchors.rightMargin: 40
                    anchors.topMargin: 8
                    anchors.bottomMargin: 8
                    spacing: 2

                    Text {
                        Layout.fillWidth: true
                        text: modelData.title || "未命名会话"
                        color: modelData.session_id === drawer.currentSessionId ? Colors.blue : Colors.text
                        font.family: Fonts.family
                        font.pixelSize: Fonts.body
                        elide: Text.ElideRight
                    }

                    Text {
                        Layout.fillWidth: true
                        text: {
                            let d = new Date(modelData.updated_at);
                            return d.toLocaleDateString() + " " + d.toLocaleTimeString().substring(0, 5);
                        }
                        color: Colors.overlay0
                        font.family: Fonts.family
                        font.pixelSize: Fonts.xs
                    }
                }

                // 删除按钮
                Rectangle {
                    width: 24
                    height: 24
                    radius: Tokens.radiusFull
                    anchors.right: parent.right
                    anchors.rightMargin: 8
                    anchors.verticalCenter: parent.verticalCenter
                    color: delArea.containsMouse ? Qt.rgba(Colors.red.r, Colors.red.g, Colors.red.b, 0.15) : "transparent"
                    visible: rowHover.hovered

                    Text {
                        anchors.centerIn: parent
                        text: "󰆴"
                        color: delArea.containsMouse ? Colors.red : Colors.overlay0
                        font.family: Fonts.family
                        font.pixelSize: Fonts.small
                    }

                    MouseArea {
                        id: delArea
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: drawer.sessionDeleted(modelData.session_id)
                    }
                }

                MouseArea {
                    id: itemArea
                    anchors.fill: parent
                    anchors.rightMargin: 36
                    cursorShape: Qt.PointingHandCursor
                    onClicked: drawer.sessionSelected(modelData.session_id)
                }

                Behavior on color {
                    ColorAnimation { duration: Tokens.animFast }
                }
            }
        }

        // ── 新建会话按钮 ──
        Rectangle {
            Layout.fillWidth: true
            height: 40
            radius: Tokens.radiusMS
            color: newArea.containsMouse ? Qt.rgba(Colors.green.r, Colors.green.g, Colors.green.b, 0.15) : Qt.rgba(1, 1, 1, 0.04)

            Text {
                anchors.centerIn: parent
                text: "󰐕 新建会话"
                color: newArea.containsMouse ? Colors.green : Colors.subtext0
                font.family: Fonts.family
                font.pixelSize: Fonts.body

                Behavior on color {
                    ColorAnimation { duration: Tokens.animFast }
                }
            }

            MouseArea {
                id: newArea
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: drawer.newSessionRequested()
            }

            Behavior on color {
                ColorAnimation { duration: Tokens.animFast }
            }
        }
    }
}
