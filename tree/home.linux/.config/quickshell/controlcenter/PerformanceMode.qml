pragma ComponentBehavior: Bound

import "../theme"
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Quickshell.Io

// 低频的电源档位调整，只在系统信息展开时读取；写入后以 daemon 回读结果为准。
RowLayout {
    id: root

    required property bool active
    property string profile: "balanced"
    property bool changeFailed: false
    spacing: Tokens.spaceS
    onActiveChanged: {
        if (active)
            reader.running = true;
    }

    function setProfile(profile) {
        root.changeFailed = false;
        console.info("[control-center] power profile requested", profile);
        writer.command = ["busctl", "--system", "set-property", "net.hadess.PowerProfiles", "/net/hadess/PowerProfiles", "net.hadess.PowerProfiles", "ActiveProfile", "s", profile];
        writer.running = true;
    }

    Process {
        id: reader
        command: ["busctl", "--system", "get-property", "net.hadess.PowerProfiles", "/net/hadess/PowerProfiles", "net.hadess.PowerProfiles", "ActiveProfile"]
        stdout: StdioCollector {
            onStreamFinished: {
                const match = text.match(/"([^"]+)"/);
                if (match)
                    root.profile = match[1];
            }
        }
    }

    Process {
        id: writer
        onExited: (exitCode, exitStatus) => {
            root.changeFailed = exitCode !== 0 || exitStatus !== 0;
            if (root.changeFailed)
                console.warn("[control-center] power profile change failed", exitCode, exitStatus);
            reader.running = false;
            reader.running = true;
        }
    }

    Text {
        Layout.fillWidth: true
        text: root.changeFailed ? "性能模式 · 切换失败" : "性能模式"
        color: root.changeFailed ? Colors.red : Colors.subtext0
        font.family: Fonts.family
        font.pixelSize: Fonts.small
    }

    Repeater {
        model: [
            {
                label: "均衡",
                value: "balanced"
            },
            {
                label: "性能",
                value: "performance"
            }
        ]
        delegate: Button {
            id: option
            required property var modelData
            implicitWidth: 54
            implicitHeight: 26
            checkable: true
            checked: root.profile === modelData.value
            enabled: !writer.running
            text: modelData.label
            onClicked: root.setProfile(modelData.value)

            HoverHandler {
                cursorShape: Qt.PointingHandCursor
            }

            background: Rectangle {
                radius: Tokens.radiusS
                color: option.checked ? Colors.withAlpha(Colors.blue, 0.2) : option.hovered ? Colors.surface2 : Colors.surface1
            }
            contentItem: Text {
                text: option.text
                color: option.checked ? Colors.blue : Colors.subtext0
                font.family: Fonts.family
                font.pixelSize: Fonts.small
                horizontalAlignment: Text.AlignHCenter
                verticalAlignment: Text.AlignVCenter
            }
        }
    }
}
