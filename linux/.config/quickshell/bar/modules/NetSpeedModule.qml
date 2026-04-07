import "../../theme"
import "../components"
import QtQuick
import Quickshell.Io

BarModule {
    id: root

    property string direction: "up" // "up" 或 "down"

    // ── 内部状态 ──
    property string ifaceName: ""
    property real speed: 0          // bytes/s
    property real totalBytes: 0     // 累计字节数
    property real _prevBytes: -1

    function formatSpeed(bytesPerSec) {
        if (bytesPerSec < 1024)
            return bytesPerSec.toFixed(0) + " B/s";
        if (bytesPerSec < 1024 * 1024)
            return (bytesPerSec / 1024).toFixed(1) + " KB/s";
        return (bytesPerSec / (1024 * 1024)).toFixed(2) + " MB/s";
    }

    function formatTotal(bytes) {
        if (bytes < 1024)
            return bytes.toFixed(0) + " B";
        if (bytes < 1024 * 1024)
            return (bytes / 1024).toFixed(1) + " KB";
        if (bytes < 1024 * 1024 * 1024)
            return (bytes / (1024 * 1024)).toFixed(1) + " MB";
        return (bytes / (1024 * 1024 * 1024)).toFixed(2) + " GB";
    }

    property string displayText: (direction === "up" ? "󰕒" : "󰁅") + " " + formatSpeed(speed)

    accentColor: Colors.teal
    implicitWidth: root.hovered
        ? Math.max(hoverRow.implicitWidth + 32, 180)
        : Math.max(label.implicitWidth + 32, 120)
    Component.onCompleted: reader.running = true

    // 读 /proc/net/dev，解析非虚拟接口的字节数
    Process {
        id: reader

        property string _buf: ""

        command: ["cat", "/proc/net/dev"]

        stdout: SplitParser {
            onRead: data => reader._buf += data + "\n"
        }

        onExited: {
            let lines = reader._buf.split("\n");
            let skipPrefixes = ["lo", "docker", "br-", "vmnet", "veth"];

            for (let line of lines) {
                let m = line.match(/^\s*(\S+):\s+(.*)/);
                if (!m)
                    continue;

                let iface = m[1];
                let skip = false;
                for (let p of skipPrefixes) {
                    if (iface.startsWith(p)) {
                        skip = true;
                        break;
                    }
                }
                if (skip)
                    continue;

                let fields = m[2].trim().split(/\s+/);
                if (fields.length < 10)
                    continue;

                let rxBytes = parseInt(fields[0]);
                let txBytes = parseInt(fields[8]);
                let curBytes = root.direction === "down" ? rxBytes : txBytes;
                let curTotal = root.direction === "down" ? rxBytes : txBytes;

                root.ifaceName = iface;
                root.totalBytes = curTotal;

                if (root._prevBytes >= 0)
                    root.speed = curBytes - root._prevBytes;
                root._prevBytes = curBytes;
                break; // 只取第一个匹配的物理接口
            }
            reader._buf = "";
        }
    }

    Timer {
        interval: 1000
        running: true
        repeat: true
        onTriggered: {
            reader.running = false;
            reader.running = true;
        }
    }

    // ── 默认视图 ──
    Text {
        id: label

        visible: !root.hovered
        anchors.centerIn: parent
        text: root.displayText
        color: Colors.teal
        font.family: Fonts.family
        font.pixelSize: Fonts.bodyLarge
        font.weight: Font.DemiBold
    }

    // ── hover 视图：接口名 + 速度 + 累计流量 ──
    Row {
        id: hoverRow

        visible: root.hovered
        anchors.centerIn: parent
        spacing: 6

        // 接口名标签
        Rectangle {
            visible: root.ifaceName !== ""
            color: Qt.rgba(Colors.teal.r, Colors.teal.g, Colors.teal.b, 0.2)
            radius: 4
            width: ifaceText.implicitWidth + 10
            height: ifaceText.implicitHeight + 4
            anchors.verticalCenter: parent.verticalCenter

            Text {
                id: ifaceText

                anchors.centerIn: parent
                text: root.ifaceName
                color: Colors.teal
                font.family: Fonts.family
                font.pixelSize: Fonts.caption
                font.weight: Font.DemiBold
            }
        }

        // 方向图标 + 速度
        Text {
            text: root.displayText
            color: Colors.teal
            font.family: Fonts.family
            font.pixelSize: Fonts.bodyLarge
            font.weight: Font.DemiBold
            anchors.verticalCenter: parent.verticalCenter
        }

        // 累计流量
        Text {
            text: "Σ " + root.formatTotal(root.totalBytes)
            color: Colors.subtext0
            font.family: Fonts.family
            font.pixelSize: Fonts.caption
            font.weight: Font.Normal
            anchors.verticalCenter: parent.verticalCenter
        }
    }
}
