import "../../theme"
import "../components"
import QtQuick
import QtQuick.Layouts
import Quickshell.Io

// 每屏一条连续信息带，宽度由工作区、窗口块与标题共同决定。
// 长标题默认省略，hover 在左侧可用空间内展开；点击标题复制当前窗口 PID。
Rectangle {
    id: root

    required property var barScreen
    readonly property int windowPid: windowContext.activeWindow?.lastIpcObject.pid || 0
    property int copyingPid: 0
    property bool copied: false
    property bool copyFailed: false

    function copyWindowPid() {
        if (root.windowPid <= 0 || copyProcess.running)
            return;
        root.copyingPid = root.windowPid;
        copyProcess.command = ["wl-copy", String(root.copyingPid)];
        copyProcess.running = true;
    }

    implicitWidth: navigationRow.implicitWidth + 18
    implicitHeight: 36
    radius: Tokens.radiusS
    color: railHover.hovered ? Colors.overlay(0.05) : "transparent"
    Behavior on color {
        BarColorAnimation {}
    }
    Behavior on implicitWidth {
        BarWidthAnimation {}
    }

    HoverHandler {
        id: railHover
    }

    WindowContext {
        id: windowContext
        barScreen: root.barScreen
        onActiveWindowChanged: {
            root.copied = false;
            root.copyFailed = false;
            copiedTimer.stop();
        }
    }

    RowLayout {
        id: navigationRow
        anchors.fill: parent
        anchors.leftMargin: 9
        anchors.rightMargin: 9
        spacing: 11
        opacity: windowContext.focused ? 1 : 0.7
        clip: true
        Behavior on opacity {
            NumberAnimation {
                duration: Tokens.animFast
            }
        }

        WorkspaceStrip {
            context: windowContext
            Layout.preferredWidth: implicitWidth
            Layout.preferredHeight: implicitHeight
        }

        Rectangle {
            Layout.preferredWidth: 1
            Layout.preferredHeight: 16
            color: Colors.overlay(0.12)
        }

        WindowMap {
            context: windowContext
            Layout.preferredWidth: implicitWidth
            Layout.preferredHeight: implicitHeight
        }

        Rectangle {
            Layout.preferredWidth: 1
            Layout.preferredHeight: 16
            color: Colors.overlay(0.12)
        }

        Rectangle {
            id: titleButton
            readonly property real compactTextWidth: Math.min(titleText.implicitWidth, 220)
            implicitWidth: (titleHover.containsMouse ? titleText.implicitWidth : compactTextWidth) + 6
            Layout.fillWidth: true
            Layout.minimumWidth: 0
            Layout.preferredHeight: 30
            radius: Tokens.radiusXS
            color: titleHover.containsMouse ? Colors.overlay(0.04) : "transparent"
            Behavior on color {
                BarColorAnimation {}
            }

            Accessible.role: root.windowPid > 0 ? Accessible.Button : Accessible.StaticText
            Accessible.name: titleText.text
            Accessible.description: root.windowPid <= 0 ? ""
                : copyProcess.running ? "正在复制窗口 PID"
                : root.copied ? "窗口 PID 已复制"
                : root.copyFailed ? "复制窗口 PID 失败"
                : "复制窗口 PID"
            Accessible.onPressAction: root.copyWindowPid()

            Item {
                id: titleViewport
                anchors.fill: parent
                anchors.leftMargin: 3
                anchors.rightMargin: 3
                clip: true

                Text {
                    id: titleText
                    anchors.verticalCenter: parent.verticalCenter
                    width: titleViewport.width
                    text: windowContext.activeWindow ? windowContext.activeWindow.title : "桌面"
                    textFormat: Text.PlainText
                    elide: Text.ElideRight
                    font.family: Fonts.family
                    font.pixelSize: Fonts.body
                    font.weight: Font.Medium
                    color: root.copied ? Colors.green : root.copyFailed ? Colors.red : titleHover.containsMouse ? Colors.lavender : Colors.text
                    Accessible.ignored: true
                    Behavior on color {
                        BarColorAnimation {}
                    }
                }
            }

            MouseArea {
                id: titleHover
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: root.windowPid > 0 ? Qt.PointingHandCursor : Qt.ArrowCursor
                acceptedButtons: Qt.LeftButton
                onClicked: root.copyWindowPid()
            }
        }
    }

    Process {
        id: copyProcess
        onExited: (exitCode, exitStatus) => {
            const succeeded = exitCode === 0 && exitStatus === 0;
            if (succeeded)
                console.info("[navigation] copied window PID", root.copyingPid);
            else
                console.warn("[navigation] failed to copy window PID", root.copyingPid, exitCode, exitStatus);
            if (root.windowPid === root.copyingPid) {
                root.copied = succeeded;
                root.copyFailed = !succeeded;
                copiedTimer.restart();
            }
        }
    }

    Timer {
        id: copiedTimer
        interval: 1500
        onTriggered: {
            root.copied = false;
            root.copyFailed = false;
        }
    }
}
