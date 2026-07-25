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
    implicitWidth: label.implicitWidth + 32
    onClicked: mouse => {
        PanelState.closeAll();
        let pos = root.mapToItem(null, mouse.x, mouse.y);
        MorphState.morphSourceX = pos.x + 2;
        MorphState.morphSourceY = pos.y + 6;
        PanelState.toggleNetwork();
    }

    Row {
        id: label

        anchors.centerIn: parent
        spacing: 5

        Text {
            text: root.iconText
            color: root.disconnected ? Colors.red : Colors.sky
            font.family: Fonts.family
            font.pixelSize: Fonts.icon
            font.weight: Font.DemiBold
            anchors.verticalCenter: parent.verticalCenter
        }

        Text {
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

        // hover 展开显示 tooltip（IP/SSID）
        Text {
            visible: root.hovered && root.tooltipText !== ""
            text: root.tooltipText.split("\n")[0]
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
