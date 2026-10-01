import "../../theme"
import "../../state"
import "../../services"
import "../components"
import QtQuick

BarModule {
    id: root

    // 网络状态收口到 NetworkService 单例（全局一次 fork），本模块只渲染
    readonly property string iconText: NetworkService.iconText
    readonly property string valueText: NetworkService.valueText
    readonly property string tooltipText: NetworkService.tooltipText
    readonly property bool disconnected: NetworkService.disconnected

    accentColor: Colors.sky
    readonly property real compactWidth: icon.implicitWidth + value.implicitWidth + 37
    implicitWidth: compactWidth + (hovered ? detail.implicitWidth + 5 : 0)
    onClicked: mouse => {
        PanelState.closeAll();
        MorphState.openFrom(root, () => PanelState.toggleNetwork());
    }

    Row {
        id: label

        anchors.left: parent.left
        anchors.verticalCenter: parent.verticalCenter
        spacing: 5

        Text {
            id: icon
            text: root.iconText
            color: root.disconnected ? Colors.red : Colors.sky
            font.family: Fonts.family
            font.pixelSize: Fonts.icon
            font.weight: Font.DemiBold
            anchors.verticalCenter: parent.verticalCenter
        }

        Text {
            id: value
            text: root.valueText
            color: root.disconnected ? Colors.red : Colors.text
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

        // 悬停与面板使用同一段 IP/SSID 摘要，收起时跟随轮廓收回。
        Text {
            id: detail
            text: root.tooltipText.split("\n")[0]
            visible: root.detailProgress > 0 && text !== ""
            width: Math.min(implicitWidth, Math.max(0, label.parent.width - root.compactWidth + 27)) * root.detailProgress
            elide: Text.ElideRight
            color: Colors.subtext0
            font.family: Fonts.family
            font.pixelSize: Fonts.caption
            anchors.verticalCenter: parent.verticalCenter
            opacity: root.detailProgress
        }
    }

}
