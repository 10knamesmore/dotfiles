import "../../theme"
import "../components"
import QtQuick
import Quickshell.Hyprland._Ipc
import Quickshell.Io

// 活动窗口标题，带图标替换（对应 Waybar hyprland/window rewrites）
BarModule {
    id: root

    property string rawTitle: Hyprland.activeToplevel ? Hyprland.activeToplevel.title : ""
    property string windowClass: ""
    property int windowPid: 0

    // 最多显示 50 个字符，hover 时显示完整标题
    property string displayTitle: {
        let t = rawTitle;
        if (hovered)
            return t;
        return t.length > 50 ? t.substring(0, 47) + "…" : t;
    }

    property bool copied: false

    accentColor: Colors.mauve
    implicitWidth: hovered ? Math.min(hoverRow.implicitWidth + 32, 600) : Math.min(titleText.implicitWidth + 32, 400)
    clip: true
    Component.onCompleted: winReader.running = true
    onClicked: {
        if (root.windowPid > 0) {
            copyProc.command = ["wl-copy", String(root.windowPid)];
            copyProc.running = true;
            root.copied = true;
            copiedTimer.restart();
        }
    }

    Process {
        id: copyProc
    }

    Timer {
        id: copiedTimer

        interval: 1500
        onTriggered: root.copied = false
    }

    Process {
        id: winReader

        property string _buf: ""

        command: ["hyprctl", "activewindow", "-j"]

        stdout: SplitParser {
            onRead: data => winReader._buf += data + "\n"
        }

        onExited: {
            try {
                let obj = JSON.parse(winReader._buf);
                root.windowClass = obj.class ?? "";
                root.windowPid = obj.pid ?? 0;
            } catch (e) {}
            winReader._buf = "";
        }
    }

    Timer {
        interval: 500
        running: true
        repeat: true
        onTriggered: {
            winReader.running = false;
            winReader.running = true;
        }
    }

    // 默认：仅标题
    Text {
        id: titleText

        visible: !root.hovered
        anchors.centerIn: parent
        width: Math.min(implicitWidth, root.width - 32)
        text: root.displayTitle
        color: Colors.text
        font.family: Fonts.family
        font.pixelSize: Fonts.bodyLarge
        font.weight: Font.Medium
        font.italic: true
        elide: Text.ElideRight
    }

    // hover：类名标签 + 完整标题 + PID
    Row {
        id: hoverRow

        // 辅助项的固定宽度（用于计算标题可用空间）
        property real extrasWidth: (root.windowClass !== "" ? classTag.width + spacing : 0) + (root.windowPid > 0 ? pidText.width + spacing : 0)
        property real maxWidth: 600

        visible: root.hovered
        anchors.centerIn: parent
        spacing: 6

        Rectangle {
            id: classTag

            visible: root.windowClass !== ""
            color: Qt.rgba(Colors.mauve.r, Colors.mauve.g, Colors.mauve.b, 0.2)
            radius: 4
            width: classText.implicitWidth + 10
            height: classText.implicitHeight + 4
            anchors.verticalCenter: parent.verticalCenter

            Text {
                id: classText

                anchors.centerIn: parent
                text: root.windowClass
                color: Colors.mauve
                font.family: Fonts.family
                font.pixelSize: Fonts.caption
                font.weight: Font.DemiBold
            }
        }

        Text {
            id: hoverTitleText

            text: root.rawTitle
            color: Colors.mauve
            font.family: Fonts.family
            font.pixelSize: Fonts.bodyLarge
            font.weight: Font.Medium
            font.italic: true
            elide: Text.ElideRight
            width: Math.min(implicitWidth, hoverRow.maxWidth - 32 - hoverRow.extrasWidth)
            anchors.verticalCenter: parent.verticalCenter
        }

        Text {
            id: pidText

            visible: root.windowPid > 0
            text: root.copied ? "✓ Copied" : "PID " + root.windowPid
            color: root.copied ? Colors.green : Colors.subtext0
            font.family: Fonts.family
            font.pixelSize: Fonts.caption
            font.weight: root.copied ? Font.DemiBold : Font.Normal
            anchors.verticalCenter: parent.verticalCenter

            Behavior on color {
                ColorAnimation { duration: Tokens.animFast }
            }
        }
    }
}
