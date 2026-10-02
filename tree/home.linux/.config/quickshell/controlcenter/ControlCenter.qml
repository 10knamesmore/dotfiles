import "../bluetooth"
import "../clipboard"
import "../components"
import "../display"
import "../network"
import "../notifications"
import "../state"
import "../theme"
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

// 所有系统控制共用一个展开窗口；子页常驻，只有当前页执行打开时的刷新与扫描。
PanelOverlay {
    id: root

    required property var notifServer
    readonly property string currentPage: PanelState.controlCenterPage

    showing: PanelState.controlCenterOpen
    panelWidth: currentPage === "display" ? 740 : 460
    panelHeight: Math.min(root.height - 100, currentPage === "home" ? overview.implicitHeight + navigation.implicitHeight : 740)
    panelTargetX: root.width - root.panelWidth - 10
    panelTargetY: 54
    closedOffsetY: -20
    onCloseRequested: PanelState.controlCenterOpen = false

    FocusScope {
        anchors.fill: parent
        focus: root.showing
        Keys.onEscapePressed: root.closeRequested()

        ColumnLayout {
            anchors.fill: parent
            spacing: 0

            RowLayout {
                id: navigation

                Layout.fillWidth: true
                Layout.leftMargin: Tokens.spaceL
                Layout.rightMargin: Tokens.spaceL
                implicitHeight: 46
                spacing: Tokens.spaceS

                HeaderButton {
                    visible: root.currentPage !== "home"
                    text: "‹ 返回"
                    Accessible.name: "返回控制中心"
                    onClicked: PanelState.openControlCenter("home")
                }
                // HeaderButton {

                Text {
                    visible: !root.hasMorphSource || root.currentPage !== "home"
                    text: "控制中心"
                    color: Colors.text
                    font.family: Fonts.family
                    font.pixelSize: Fonts.title
                    font.weight: Font.DemiBold
                }

                Item {
                    Layout.fillWidth: true
                }

            }

            Item {
                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true

                Overview {
                    id: overview

                    anchors.fill: parent
                    visible: root.currentPage === "home"
                    showing: root.showing && visible
                }

                NetworkPage {
                    anchors.fill: parent
                    visible: root.currentPage === "network"
                    showing: root.showing && visible
                    onCloseRequested: root.closeRequested()
                }

                BluetoothPage {
                    anchors.fill: parent
                    visible: root.currentPage === "bluetooth"
                    showing: root.showing && visible
                    onCloseRequested: root.closeRequested()
                }

                ClipboardPage {
                    anchors.fill: parent
                    visible: root.currentPage === "clipboard"
                    showing: root.showing && visible
                    onCloseRequested: root.closeRequested()
                }

                NotificationPage {
                    anchors.fill: parent
                    visible: root.currentPage === "notifications"
                    showing: root.showing && visible
                    notifServer: root.notifServer
                }

                DisplayPage {
                    anchors.fill: parent
                    visible: root.currentPage === "display"
                    showing: root.showing && visible
                }

            }

        }

    }

    Behavior on panelWidth {
        enabled: root.showing && root.panel.progress === 1

        NumberAnimation {
            duration: Tokens.animNormal
            easing.type: Easing.OutCubic
        }

    }

    component HeaderButton: Button {
        id: button

        implicitWidth: label.implicitWidth + 20
        implicitHeight: 30

        contentItem: Text {
            id: label

            text: button.text
            color: button.hovered ? Colors.text : Colors.subtext0
            font.family: Fonts.family
            font.pixelSize: Fonts.small
            horizontalAlignment: Text.AlignHCenter
            verticalAlignment: Text.AlignVCenter
        }

        background: Rectangle {
            radius: Tokens.radiusS
            color: button.hovered ? Colors.surface2 : Colors.surface1
        }

    }

}
