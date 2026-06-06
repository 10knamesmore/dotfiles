import "../theme"
import QtQuick
import QtQuick.Layouts

// 主题口味选择器 — 4 个 Catppuccin 风味圆形色块
RowLayout {
    spacing: Tokens.spaceS

    Repeater {
        model: [
            { name: "mocha", base: "#1e1e2e", accent: "#89b4fa", label: "Mocha" },
            { name: "macchiato", base: "#24273a", accent: "#8aadf4", label: "Macchiato" },
            { name: "frappe", base: "#303446", accent: "#8caaee", label: "Frappe" },
            { name: "latte", base: "#eff1f5", accent: "#1e66f5", label: "Latte" }
        ]

        delegate: Rectangle {
            required property var modelData
            required property int index

            property bool isCurrent: Colors.currentFlavor === modelData.name
            property bool hovered: area.containsMouse

            Layout.fillWidth: true
            // 兜底：cell 不会窄于内容，避免均分宽度时长标签（Macchiato）溢出、选中边框穿过文字
            Layout.minimumWidth: content.implicitWidth + 12
            Layout.preferredHeight: 32
            radius: Tokens.radiusS
            color: hovered ? Colors.surface1 : (isCurrent ? Colors.surface0 : "transparent")
            border.color: isCurrent ? modelData.accent : "transparent"
            border.width: isCurrent ? 2 : 0

            Row {
                id: content

                anchors.centerIn: parent
                spacing: 5

                Rectangle {
                    width: 12
                    height: 12
                    radius: 6
                    color: modelData.base
                    border.color: modelData.accent
                    border.width: 1.5
                    anchors.verticalCenter: parent.verticalCenter
                }

                Text {
                    text: modelData.label
                    color: isCurrent ? Colors.text : Colors.subtext0
                    font.family: Fonts.family
                    font.pixelSize: Fonts.xs
                    font.weight: isCurrent ? Font.Bold : Font.Normal
                    anchors.verticalCenter: parent.verticalCenter
                }
            }

            MouseArea {
                id: area
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: Colors.setFlavor(modelData.name)
            }

            Behavior on color {
                ColorAnimation { duration: Tokens.animFast }
            }
            Behavior on border.color {
                ColorAnimation { duration: Tokens.animFast }
            }
        }
    }
}
