import "../../theme"
import "../components"
import QtQuick
import QtQuick.Layouts
import Quickshell.Services.UPower

BarModule {
    id: root

    property var dev: UPower.displayDevice
    property int pct: dev ? Math.round(dev.percentage * 100) : 0
    property bool charging: dev ? dev.state === UPowerDeviceState.Charging : false
    property bool full: dev ? dev.state === UPowerDeviceState.FullyCharged : false

    // Waybar: format-charging "⚡", format-full "✔", format-icons 5 levels
    function batteryIcon() {
        if (full)
            return "✔";

        if (charging)
            return "⚡";

        if (pct >= 90)
            return "";

        if (pct >= 60)
            return "";

        if (pct >= 40)
            return "";

        if (pct >= 20)
            return "";

        return "";
    }

    implicitWidth: label.implicitWidth + 32
    // 状态底色
    tintColor: {
        if (charging || full)
            return Qt.rgba(0.545, 0.835, 0.792, 0.08);

        // teal 8%
        if (pct <= 10)
            return Qt.rgba(0.929, 0.529, 0.588, 0.18);

        // red 18%
        if (pct <= 30)
            return Qt.rgba(0.933, 0.831, 0.624, 0.12);

        // yellow 12%
        return "transparent";
    }
    // 根据电量/状态动态调整颜色
    accentColor: {
        if (charging || full)
            return Colors.green;

        if (pct <= 10)
            return Colors.red;

        if (pct <= 30)
            return Colors.peach;

        return Colors.green;
    }

    Row {
        id: label

        anchors.centerIn: parent
        spacing: 5

        Text {
            text: root.batteryIcon()
            color: root.accentColor
            font.family: Fonts.family
            font.pixelSize: Fonts.icon
            font.weight: Font.DemiBold
            anchors.verticalCenter: parent.verticalCenter
        }

        Text {
            text: root.pct + "%"
            color: root.pct <= 10 ? Colors.red : (root.pct <= 30 ? Colors.peach : Colors.text)
            font.family: Fonts.family
            font.pixelSize: Fonts.bodyLarge
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
