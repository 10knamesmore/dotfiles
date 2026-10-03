import "../theme"
import "../state"
import "../services"
import "../screen-effects"
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Quickshell.Bluetooth

// 控制首页只展示连接摘要和常用亮度；滑块与详情入口分层，不删减原有设置。
Flickable {
    id: root

    required property bool showing
    implicitHeight: mainCol.implicitHeight + Tokens.spaceL * 2
    contentHeight: implicitHeight
    contentWidth: width
    clip: true
    onShowingChanged: {
        if (showing)
            ScreenEffectsState.refresh();
    }

    // 留白放在滚动内容内，卡片缩放不会碰到 viewport 的裁切边缘。
    ColumnLayout {
        id: mainCol
        x: Tokens.spaceL
        y: Tokens.spaceL
        width: root.width - Tokens.spaceL * 2
        spacing: Tokens.spaceM

        RowLayout {
            Layout.fillWidth: true
            spacing: Tokens.spaceM

            ConnectionCard {
                icon: NetworkService.statusIcon
                label: NetworkService.connectionType === "ethernet" ? "有线网络" : "Wi-Fi"
                status: NetworkService.disconnected ? "未连接" : NetworkService.connectionType === "wifi" ? NetworkService.ssid : NetworkService.interfaceName
                detail: NetworkService.disconnected ? "" : NetworkService.address
                connected: !NetworkService.disconnected
                onClicked: PanelState.openControlCenter("network")
            }

            ConnectionCard {
                icon: Bluetooth.defaultAdapter?.enabled ? "󰂯" : "󰂲"
                label: "蓝牙"
                status: Bluetooth.defaultAdapter?.enabled ? "已开启" : "已关闭"
                connected: Bluetooth.defaultAdapter?.enabled ?? false
                onClicked: PanelState.openControlCenter("bluetooth")
            }
        }

        Rectangle {
            Layout.fillWidth: true
            implicitHeight: displayControls.implicitHeight + Tokens.spaceL * 2
            radius: Tokens.radiusM
            color: Colors.withAlpha(Colors.surface1, 0.45)
            border.width: 1
            border.color: Colors.overlay(0.05)

            ColumnLayout {
                id: displayControls
                anchors.fill: parent
                anchors.margins: Tokens.spaceL
                spacing: Tokens.spaceS

                EffectSlider {
                    label: "☀"
                    accessibleName: "亮度"
                    compactLabel: true
                    value: ScreenEffectsState.brightness
                    onMoved: value => ScreenEffectsState.setBrightness(value)
                }

                DetailLink {
                    title: "屏幕效果"
                    description: "色温、颗粒、暗部增强"
                    iconText: "󰒓"
                    onClicked: PanelState.openControlCenter("effects")
                }

                Divider {
                    Layout.fillWidth: true
                }

                DetailLink {
                    title: "显示器布局"
                    description: MonitorState.monitors.length + " 台 · 排列、分辨率与 HDR"
                    iconText: "󰍹"
                    onClicked: PanelState.openControlCenter("display")
                }
            }
        }
    }

    component DetailLink: Button {
        id: link
        required property string title
        required property string description
        required property string iconText
        Layout.fillWidth: true
        implicitHeight: 52
        padding: 0
        Accessible.name: title
        Accessible.description: description

        HoverHandler {
            cursorShape: Qt.PointingHandCursor
        }

        background: Rectangle {
            radius: Tokens.radiusS
            color: link.hovered ? Colors.overlay(0.04) : "transparent"
            Behavior on color {
                ColorAnimation {
                    duration: Tokens.animFast
                }
            }
        }

        contentItem: RowLayout {
            spacing: Tokens.spaceS
            Text {
                Layout.fillWidth: false
                text: link.iconText
                color: Colors.mauve
                font.family: Fonts.family
                font.pixelSize: Fonts.icon
            }
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 3
                Text {
                    Layout.fillWidth: true
                    text: link.title
                    color: Colors.text
                    font.family: Fonts.family
                    font.pixelSize: Fonts.body
                }
                Text {
                    Layout.fillWidth: true
                    text: link.description
                    color: Colors.subtext0
                    font.family: Fonts.family
                    font.pixelSize: Fonts.xs
                }
            }
            Text {
                Layout.fillWidth: false
                text: "›"
                color: Colors.overlay1
                font.family: Fonts.family
                font.pixelSize: Fonts.title
            }
        }
    }
}
