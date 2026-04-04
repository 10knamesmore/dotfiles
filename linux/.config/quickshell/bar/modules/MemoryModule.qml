import QtQuick
import Quickshell.Io
import "../components"
import "../../theme"

BarModule {
    id: root
    accentColor: Colors.mauve
    implicitWidth: label.implicitWidth + 32

    property int usagePct: 0
    property string tooltipText: ""

    Process {
        id: reader
        command: ["bash", "-c", "grep -E '^Mem(Total|Available)|^Swap(Total|Free)' /proc/meminfo"]
        stdout: SplitParser {
            property var vals: ({})
            onRead: data => {
                let m = data.match(/^(\w+):\s+(\d+)/);
                if (m)
                    vals[m[1]] = parseInt(m[2]);
            }
        }
        onExited: {
            let v = reader.stdout.vals;
            if (v.MemTotal && v.MemAvailable) {
                let used = v.MemTotal - v.MemAvailable;
                root.usagePct = Math.round(used / v.MemTotal * 100);
                let usedGib = (used / 1048576).toFixed(1);
                let totalGib = (v.MemTotal / 1048576).toFixed(1);
                let swapUsed = v.SwapTotal - (v.SwapFree ?? 0);
                root.tooltipText = "RAM: " + usedGib + " / " + totalGib + " GiB (" + root.usagePct + "%)" + (v.SwapTotal > 0 ? "\nSwap: " + (swapUsed / 1048576).toFixed(1) + " / " + (v.SwapTotal / 1048576).toFixed(1) + " GiB" : "");
            }
            reader.stdout.vals = {};
        }
    }

    Component.onCompleted: reader.running = true

    Timer {
        interval: 1000
        running: true
        repeat: true
        onTriggered: {
            reader.running = false;
            reader.running = true;
        }
    }

    Row {
        id: label
        anchors.centerIn: parent
        spacing: 5

        Text {
            text: ""
            color: Colors.mauve
            font.family: "Hack Nerd Font"
            font.pixelSize: 14
            font.weight: Font.DemiBold
            anchors.verticalCenter: parent.verticalCenter
        }

        Text {
            text: root.usagePct + "%"
            color: root.usagePct > 85 ? Colors.red : (root.usagePct > 60 ? Colors.yellow : Colors.text)
            font.family: "Hack Nerd Font"
            font.pixelSize: 13
            font.weight: Font.DemiBold
            anchors.verticalCenter: parent.verticalCenter
            Behavior on color {
                ColorAnimation {
                    duration: 300
                }
            }
        }
    }
}
