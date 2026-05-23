import "../../theme"
import "../components"
import QtQuick

BarModule {
    id: root

    // 数据来自 SystemStats（SystemStatsService 每秒更新）
    accentColor: Colors.mauve
    implicitWidth: hovered ? (label.implicitWidth + 32) : (compactLabel.implicitWidth + 32)

    Row {
        id: compactLabel
        visible: false
        spacing: 5
        Text { text: label.children[0].text; font.family: Fonts.family; font.pixelSize: Fonts.icon }
        Text { text: SystemStats.memUsagePct + "%"; font.family: Fonts.family; font.pixelSize: Fonts.bodyLarge }
    }

    Row {
        id: label

        anchors.centerIn: parent
        spacing: 5

        Text {
            text: ""
            color: Colors.mauve
            font.family: Fonts.family
            font.pixelSize: Fonts.icon
            font.weight: Font.DemiBold
            anchors.verticalCenter: parent.verticalCenter
        }

        Text {
            text: SystemStats.memUsagePct + "%"
            color: SystemStats.memUsagePct > 85 ? Colors.red : (SystemStats.memUsagePct > 60 ? Colors.yellow : Colors.text)
            font.family: Fonts.family
            font.pixelSize: Fonts.bodyLarge
            font.weight: Font.DemiBold
            anchors.verticalCenter: parent.verticalCenter

            Behavior on color {
                ColorAnimation {
                    duration: 300
                }

            }

        }

        // hover 展开显示详细内存
        Text {
            visible: root.hovered && SystemStats.memDetailText !== ""
            text: SystemStats.memDetailText
            color: Colors.subtext0
            font.family: Fonts.family
            font.pixelSize: Fonts.caption
            anchors.verticalCenter: parent.verticalCenter
            opacity: root.hovered ? 1 : 0

            Behavior on opacity {
                NumberAnimation { duration: Tokens.animNormal }
            }
        }
    }

}
