import "../../theme"
import "../../state"
import "../components"
import QtQuick

BarModule {
    id: root

    // 数据来自 SystemStats（SystemStatsService 每秒更新）
    accentColor: Colors.mauve
    implicitWidth: compactLabel.implicitWidth + 32 + (hovered && SystemStats.memTooltipText !== "" ? memoryDetails.implicitWidth + label.spacing : 0)
    clickable: false

    Row {
        id: compactLabel
        visible: false
        spacing: 5
        Text {
            text: memoryIcon.text
            font.family: Fonts.family
            font.pixelSize: Fonts.icon
        }
        Text {
            text: SystemStats.memUsagePct + "%"
            font.family: Fonts.family
            font.pixelSize: Fonts.bodyLarge
        }
    }

    Row {
        id: label

        anchors.centerIn: parent
        spacing: 5

        Text {
            id: memoryIcon
            text: "󰍛"
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

        // hover 展开显示详细内存（RAM + Swap，memTooltipText 折成一行）
        Text {
            id: memoryDetails
            visible: root.hoverDetailsVisible && SystemStats.memTooltipText !== ""
            width: implicitWidth * root.hoverReveal
            clip: true
            text: SystemStats.memTooltipText.replace("\n", "  ·  ")
            color: Colors.subtext0
            font.family: Fonts.family
            font.pixelSize: Fonts.caption
            anchors.verticalCenter: parent.verticalCenter
            opacity: root.hoverReveal
        }
    }
}
