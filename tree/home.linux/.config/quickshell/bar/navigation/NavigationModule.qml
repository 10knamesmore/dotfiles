import "../../theme"
import "../components"
import QtQuick
import QtQuick.Layouts
import Quickshell.Io

// 每屏一条连续信息带，宽度由工作区、窗口块与标题共同决定。
// 点击标题复制当前窗口 PID；hover 与复制反馈只改变颜色，不改变内容宽度。
Rectangle {
    id: root

    required property var barScreen
    readonly property int windowPid: windowContext.activeWindow?.lastIpcObject.pid || 0
    property int copyingPid: 0
    property bool copied: false
    property bool copyFailed: false

    implicitWidth: navigationRow.implicitWidth + 18
    implicitHeight: 36
    radius: Tokens.radiusS
    color: Colors.withAlpha(Colors.surface0, railHover.hovered ? 0.82 : 0.68)
    Behavior on color { BarColorAnimation {} }
    Behavior on implicitWidth { BarWidthAnimation {} }

    HoverHandler { id: railHover }

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
        Behavior on opacity { NumberAnimation { duration: Tokens.animFast } }

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
            implicitWidth: titleText.implicitWidth + 6
            Layout.fillWidth: true
            Layout.minimumWidth: 0
            Layout.preferredHeight: 30
            radius: Tokens.radiusXS
            color: titleHover.containsMouse ? Colors.overlay(0.04) : "transparent"
            Behavior on color { BarColorAnimation {} }

            Text {
                id: titleText
                anchors.fill: parent
                anchors.leftMargin: 3
                anchors.rightMargin: 3
                verticalAlignment: Text.AlignVCenter
                text: windowContext.activeWindow ? windowContext.activeWindow.title : "桌面"
                textFormat: Text.PlainText
                elide: Text.ElideRight
                font.family: Fonts.family
                font.pixelSize: Fonts.body
                font.weight: Font.Medium
                color: root.copied ? Colors.green : root.copyFailed ? Colors.red
                    : titleHover.containsMouse ? Colors.lavender : Colors.text
                Behavior on color { BarColorAnimation {} }
            }

            MouseArea {
                id: titleHover
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: root.windowPid > 0 ? Qt.PointingHandCursor : Qt.ArrowCursor
                acceptedButtons: Qt.LeftButton
                onClicked: {
                    if (root.windowPid <= 0 || copyProcess.running)
                        return;
                    root.copyingPid = root.windowPid;
                    copyProcess.command = ["wl-copy", String(root.copyingPid)];
                    copyProcess.running = true;
                }
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
