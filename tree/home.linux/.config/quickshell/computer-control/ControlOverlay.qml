import QtQuick
import QtQuick.Controls
import QtQuick.Window
import Quickshell
import Quickshell.Wayland

// 每屏提示层仅断开按钮接受输入；首帧呈现是 host 握手的前置条件。
PanelWindow {
    id: root

    required property var service
    property bool presented: false
    readonly property ControlConnection connection: service.currentConnection

    anchors {
        top: true
        bottom: true
        left: true
        right: true
    }
    visible: service.controlling
    color: "transparent"
    focusable: false
    exclusionMode: ExclusionMode.Ignore
    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.keyboardFocus: WlrKeyboardFocus.None
    WlrLayershell.namespace: "pi-computer-control"
    mask: Region {
        item: disconnectButton
    }

    onVisibleChanged: {
        if (!visible)
            presented = false;
    }
    onClosed: service.revoke("overlay-closed")
    onResourcesLost: service.revoke("overlay-resources-lost")

    Connections {
        target: root.contentItem.Window.window
        function onFrameSwapped() {
            if (root.visible && root.backingWindowVisible && !root.presented) {
                root.presented = true;
                console.info("[computer-control] overlay presented screen=" + root.screen.name);
                root.service.confirmReady();
            }
        }
    }

    Item {
        anchors.fill: parent
        clip: true
        Accessible.role: Accessible.Pane
        Accessible.name: "桌面接管 · " + root.screen.name

        Rectangle {
            anchors.fill: parent
            anchors.margins: 1
            color: "transparent"
            border.color: "#b4befe"
            border.width: 2
            radius: 10
            Accessible.ignored: true
        }

        VirtualPointer {
            connection: root.connection
            screenName: root.screen.name
            width: parent.width
            height: parent.height
        }

        Rectangle {
            id: banner
            anchors.top: parent.top
            anchors.topMargin: 64
            anchors.horizontalCenter: parent.horizontalCenter
            width: bannerContents.width + 24
            height: 46
            radius: 23
            color: "#f2202131"
            border.width: 1
            border.color: "#70b4befe"

            Row {
                id: bannerContents
                anchors.centerIn: parent
                spacing: 12

                Rectangle {
                    anchors.verticalCenter: parent.verticalCenter
                    width: 6
                    height: 6
                    radius: 3
                    color: "#b4befe"
                    Accessible.ignored: true
                }
                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    text: "Pi 正在操作桌面"
                    color: "#dce2ff"
                    font.pixelSize: 13
                    Accessible.role: Accessible.StaticText
                    Accessible.name: text
                }
                Button {
                    id: disconnectButton
                    width: 92
                    height: 30
                    text: "断开连接"
                    focusPolicy: Qt.NoFocus
                    enabled: root.service.controlling
                    onClicked: root.service.revoke("user")
                    Accessible.role: Accessible.Button
                    Accessible.name: text
                    Accessible.description: "断开当前桌面控制并释放 agent 输入"
                    Accessible.pressed: down
                    background: Rectangle {
                        radius: 15
                        color: disconnectButton.down ? "#8c98d2" : disconnectButton.hovered ? "#c6ceff" : "#b4befe"
                    }
                    contentItem: Text {
                        text: disconnectButton.text
                        color: "#181825"
                        font.pixelSize: 12
                        font.weight: Font.DemiBold
                        horizontalAlignment: Text.AlignHCenter
                        verticalAlignment: Text.AlignVCenter
                        Accessible.ignored: true
                    }
                }
            }
        }

        InputStatus {
            anchors.bottom: parent.bottom
            anchors.bottomMargin: 28
            anchors.horizontalCenter: parent.horizontalCenter
            width: Math.min(implicitWidth, parent.width - 32)
            connection: root.connection
        }
    }
}
