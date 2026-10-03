pragma ComponentBehavior: Bound

import "../bluetooth"
import "../clipboard"
import "../components"
import "../display"
import "../network"
import "../notifications"
import "../screen-effects"
import "../state"
import "../theme"
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Quickshell

// 一个窗口承载三个任务分区。页面实例常驻，StackView 负责切换动效，不销毁执行中的操作。
PanelOverlay {
    id: root

    required property var notifServer
    readonly property string currentPage: PanelState.controlCenterPage
    readonly property bool controlTab: PanelState.controlCenterTab === 0
    readonly property bool inDetail: controlTab && currentPage !== "home"
    readonly property real targetInfoHeight: controlTab ? Math.min(systemInfo.implicitHeight, root.height * 0.35) : 0
    property real infoHeight: targetInfoHeight
    readonly property real pageHeight: currentPage === "home" ? overview.implicitHeight : currentPage === "effects" ? effects.implicitHeight : 520
    readonly property var pages: ({
            home: homePage,
            network: networkPage,
            bluetooth: bluetoothPage,
            notifications: notificationPage,
            clipboard: clipboardPage,
            display: displayPage,
            effects: effectsPage
        })

    showing: PanelState.controlCenterOpen
    interactiveHeader: true
    alignRight: true
    panelWidth: currentPage === "display" ? 740 : 460
    panelHeight: Math.min(root.height - 100, pageHeight + targetInfoHeight + (controlTab ? 12 : 0) + tabs.implicitHeight + footer.implicitHeight + (hasMorphSource ? 0 : titleRow.implicitHeight) + (inDetail ? backButton.implicitHeight : 0))
    panelTargetY: 54
    closedOffsetY: -20
    onCloseRequested: PanelState.controlCenterOpen = false
    onCurrentPageChanged: pageStack.navigate()
    Component.onCompleted: pageStack.navigate()

    function goBack() {
        if (currentPage === "network" && network.editingSsid !== "")
            network.editingSsid = "";
        else if (currentPage !== "home")
            PanelState.openControlCenter("home");
        else
            root.closeRequested();
    }

    Behavior on panelWidth {
        enabled: root.showing && root.panel.progress === 1
        NumberAnimation {
            duration: 260
            easing.type: Easing.OutCubic
        }
    }
    Behavior on infoHeight {
        enabled: root.showing && root.panel.progress === 1
        NumberAnimation {
            duration: 220
            easing.type: Easing.OutCubic
        }
    }
    Behavior on panelHeight {
        enabled: root.showing && root.panel.progress === 1
        NumberAnimation {
            duration: 220
            easing.type: Easing.OutCubic
        }
    }

    FocusScope {
        anchors.fill: parent
        focus: root.showing
        Keys.onEscapePressed: root.goBack()

        ColumnLayout {
            anchors.fill: parent
            spacing: 0

            RowLayout {
                id: titleRow
                visible: !root.hasMorphSource
                Layout.fillWidth: true
                Layout.leftMargin: Tokens.spaceL
                Layout.rightMargin: Tokens.spaceL
                implicitHeight: 38

                Text {
                    text: "控制中心"
                    color: Colors.text
                    font.family: Fonts.family
                    font.pixelSize: Fonts.title
                    font.weight: Font.DemiBold
                }
                Item {
                    Layout.fillWidth: true
                }
                Text {
                    text: Quickshell.env("USER")
                    color: Colors.overlay1
                    font.family: Fonts.family
                    font.pixelSize: Fonts.small
                }
            }

            CenterTabs {
                id: tabs
                Layout.fillWidth: true
                Layout.leftMargin: Tokens.spaceL
                Layout.rightMargin: Tokens.spaceL
            }

            Button {
                id: backButton
                visible: root.inDetail
                Layout.leftMargin: Tokens.spaceL
                implicitHeight: 34
                implicitWidth: backText.implicitWidth + 12
                text: "‹ 返回控制"
                Accessible.name: "返回控制首页"
                onClicked: PanelState.openControlCenter("home")
                HoverHandler {
                    cursorShape: Qt.PointingHandCursor
                }
                background: Item {}
                contentItem: Text {
                    id: backText
                    text: backButton.text
                    color: backButton.hovered ? Colors.text : Colors.subtext0
                    font.family: Fonts.family
                    font.pixelSize: Fonts.small
                    verticalAlignment: Text.AlignVCenter
                }
            }

            StackView {
                id: pageStack
                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true
                initialItem: homePage
                property string displayedPage: "home"
                property int direction: 1

                // 快速连续选择时保留最后一个目标，等当前过渡完成再进入，避免中断页面生命周期。
                function navigate() {
                    if (busy || displayedPage === root.currentPage)
                        return;
                    const oldTab = PanelState.tabForControlPage(displayedPage);
                    const newTab = PanelState.controlCenterTab;
                    direction = oldTab !== newTab ? (newTab > oldTab ? 1 : -1) : root.currentPage === "home" ? -1 : 1;
                    displayedPage = root.currentPage;
                    replaceCurrentItem(root.pages[displayedPage], {}, root.showing ? StackView.ReplaceTransition : StackView.Immediate);
                }
                onBusyChanged: {
                    if (!busy)
                        navigate();
                }

                replaceEnter: Transition {
                    NumberAnimation {
                        property: "x"
                        from: pageStack.direction * 18
                        to: 0
                        duration: 220
                        easing.type: Easing.OutCubic
                    }
                    NumberAnimation {
                        property: "opacity"
                        from: 0
                        to: 1
                        duration: 220
                        easing.type: Easing.OutCubic
                    }
                }
                replaceExit: Transition {
                    NumberAnimation {
                        property: "x"
                        from: 0
                        to: -pageStack.direction * 18
                        duration: 150
                        easing.type: Easing.OutCubic
                    }
                    NumberAnimation {
                        property: "opacity"
                        from: 1
                        to: 0
                        duration: 150
                        easing.type: Easing.OutCubic
                    }
                }
            }

            SystemInfoCard {
                id: systemInfo
                Layout.fillWidth: true
                Layout.leftMargin: Tokens.spaceL
                Layout.rightMargin: Tokens.spaceL
                Layout.bottomMargin: visible ? 12 : 0
                Layout.preferredHeight: root.infoHeight
                Layout.maximumHeight: root.infoHeight
                visible: root.controlTab
                showing: root.showing && visible
            }

            PowerActions {
                id: footer
                Layout.fillWidth: true
            }
        }

        CenterPage {
            id: homePage
            name: "home"
            Overview {
                id: overview
                anchors.fill: parent
                showing: homePage.showing
            }
        }

        CenterPage {
            id: networkPage
            name: "network"
            NetworkPage {
                id: network
                anchors.fill: parent
                showing: networkPage.showing
            }
        }

        CenterPage {
            id: bluetoothPage
            name: "bluetooth"
            BluetoothPage {
                anchors.fill: parent
                showing: bluetoothPage.showing
            }
        }

        CenterPage {
            id: notificationPage
            name: "notifications"
            NotificationPage {
                anchors.fill: parent
                showing: notificationPage.showing
                notifServer: root.notifServer
            }
        }

        CenterPage {
            id: clipboardPage
            name: "clipboard"
            ClipboardPage {
                anchors.fill: parent
                showing: clipboardPage.showing
                onCloseRequested: root.closeRequested()
            }
        }

        CenterPage {
            id: displayPage
            name: "display"
            DisplayPage {
                anchors.fill: parent
                showing: displayPage.showing
            }
        }

        CenterPage {
            id: effectsPage
            name: "effects"
            Flickable {
                anchors.fill: parent
                contentHeight: effects.implicitHeight
                clip: true
                ScreenEffectsControls {
                    id: effects
                    width: parent.width
                    height: implicitHeight
                    showing: effectsPage.showing
                }
            }
        }
    }

    component CenterPage: FocusScope {
        id: page
        required property string name
        readonly property bool showing: root.showing && root.currentPage === name && pageStack.currentItem === page
        visible: false
        enabled: showing
        focus: showing
    }
}
