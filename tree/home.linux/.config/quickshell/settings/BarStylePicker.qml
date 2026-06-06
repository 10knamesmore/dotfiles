import "../theme"
import QtQuick
import QtQuick.Layouts

// Bar 样式选择器 — 切换顶栏预设（满栏药丸 / 悬浮扁平），运行时生效并持久化
RowLayout {
    spacing: Tokens.spaceS

    Repeater {
        model: BarLayout.presets

        delegate: Rectangle {
            required property var modelData
            required property int index

            property bool isCurrent: BarLayout.currentName === modelData.name
            property bool hovered: area.containsMouse

            Layout.fillWidth: true
            Layout.preferredHeight: 32
            radius: Tokens.radiusS
            color: hovered ? Colors.surface1 : (isCurrent ? Colors.surface0 : "transparent")
            border.color: isCurrent ? Colors.mauve : "transparent"
            border.width: isCurrent ? 2 : 0

            Text {
                anchors.centerIn: parent
                text: modelData.label
                color: isCurrent ? Colors.text : Colors.subtext0
                font.family: Fonts.family
                font.pixelSize: Fonts.caption
                font.weight: isCurrent ? Font.Bold : Font.Normal
            }

            MouseArea {
                id: area

                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: BarLayout.setLayout(modelData.name)
            }

            Behavior on color {
                ColorAnimation {
                    duration: Tokens.animFast
                }
            }

            Behavior on border.color {
                ColorAnimation {
                    duration: Tokens.animFast
                }
            }
        }
    }
}
