import "../theme"
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Quickshell
import Quickshell.Io

// 沿用系统信息卡片的运行时间、折叠详情与重载入口；展开内容末尾提供低频性能切换。
Rectangle {
    id: root

    required property bool showing
    property bool expanded: false
    property string uptime: "运行时间…"
    implicitHeight: header.implicitHeight + 20 + (expanded ? details.implicitHeight + Tokens.spaceS : 0)
    radius: Tokens.radiusM
    color: Colors.withAlpha(Colors.surface0, Tokens.cardAlpha)
    border.width: 1
    border.color: Colors.overlay(0.06)
    clip: true

    Timer {
        running: root.showing
        interval: 60000
        repeat: true
        triggeredOnStart: true
        onTriggered: uptimeReader.running = true
    }
    Process {
        id: uptimeReader
        command: ["sh", "-c", "uptime -p | sed 's/up //'"]
        stdout: StdioCollector {
            onStreamFinished: root.uptime = text.trim()
        }
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 10
        spacing: Tokens.spaceS

        RowLayout {
            id: header
            Layout.fillWidth: true
            Layout.alignment: Qt.AlignTop
            spacing: 6

            Button {
                id: expandButton
                Layout.fillWidth: true
                implicitHeight: 28
                checkable: true
                checked: root.expanded
                Accessible.name: "系统信息"
                Accessible.description: root.uptime
                onToggled: root.expanded = checked
                HoverHandler {
                    cursorShape: Qt.PointingHandCursor
                }
                background: Rectangle {
                    radius: Tokens.radiusS
                    color: expandButton.hovered ? Colors.overlay(0.04) : "transparent"
                }
                contentItem: RowLayout {
                    spacing: 10
                    Text {
                        text: "󰍹"
                        color: Colors.overlay1
                        font.family: Fonts.family
                        font.pixelSize: Fonts.icon
                    }
                    Text {
                        Layout.fillWidth: true
                        text: root.uptime
                        color: Colors.subtext0
                        font.family: Fonts.family
                        font.pixelSize: Fonts.small
                        elide: Text.ElideRight
                    }
                    Text {
                        text: root.expanded ? "󰅃" : "󰅀"
                        color: Colors.overlay1
                        font.family: Fonts.family
                        font.pixelSize: Fonts.icon
                    }
                }
            }

            Button {
                id: reloadButton
                implicitWidth: 28
                implicitHeight: 28
                text: "󰑓"
                Accessible.name: "重载桌面配置"
                onClicked: Quickshell.reload(true)
                HoverHandler {
                    cursorShape: Qt.PointingHandCursor
                }
                background: Rectangle {
                    radius: Tokens.radiusFull
                    color: reloadButton.hovered ? Colors.surface2 : "transparent"
                }
                contentItem: Text {
                    text: reloadButton.text
                    color: Colors.subtext0
                    font.family: Fonts.family
                    font.pixelSize: Fonts.icon
                    horizontalAlignment: Text.AlignHCenter
                    verticalAlignment: Text.AlignVCenter
                }
            }
        }

        Flickable {
            Layout.fillWidth: true
            Layout.fillHeight: true
            visible: root.height > header.implicitHeight + 20
            enabled: root.expanded
            contentHeight: details.implicitHeight
            clip: true

            ColumnLayout {
                id: details
                width: parent.width
                spacing: Tokens.spaceS

                SystemInfo {
                    Layout.fillWidth: true
                    expanded: true
                    active: root.showing && root.expanded
                }

                PerformanceMode {
                    Layout.fillWidth: true
                    active: root.showing && root.expanded
                }
            }
        }
    }
}
