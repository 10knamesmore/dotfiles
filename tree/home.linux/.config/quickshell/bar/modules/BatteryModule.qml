import "../../theme"
import "../components"
import QtQuick
import Quickshell.Services.UPower

// 电池轮廓常驻，内部显示电量填充、百分比与充电闪电。
BarModule {
    id: root

    property var batteryDevice: UPower.displayDevice
    readonly property int percentage: batteryDevice ? Math.round(batteryDevice.percentage * 100) : 0
    readonly property bool charging: batteryDevice ? batteryDevice.state === UPowerDeviceState.Charging : false
    readonly property bool fullyCharged: batteryDevice ? batteryDevice.state === UPowerDeviceState.FullyCharged : false

    clickable: false
    implicitWidth: batteryContent.implicitWidth + 32
    Accessible.role: Accessible.Indicator
    Accessible.name: "电池 " + percentage + "%"
    Accessible.description: fullyCharged ? "已充满" : (charging ? "充电中" : "放电中")
    accentColor: percentage <= 10 ? Colors.red : Colors.overlay(0.85)

    Item {
        id: batteryContent

        anchors.centerIn: parent
        implicitWidth: batteryBody.width + 3
        implicitHeight: batteryBody.height

        Rectangle {
            id: batteryBody

            width: batteryLabel.implicitWidth + 10
            height: 18
            radius: 3
            color: Colors.overlay(0.12)
            border.width: 1
            border.color: Colors.withAlpha(root.accentColor, 0.45)

            Text {
                id: batteryLabel

                anchors.centerIn: parent
                text: (root.charging ? " " : "") + root.percentage + "%"
                color: Colors.overlay(0.9)
                font.family: Fonts.family
                font.pixelSize: Fonts.caption
                font.weight: Font.DemiBold
                Accessible.ignored: true
            }

            Rectangle {
                id: batteryFill

                anchors.left: parent.left
                anchors.leftMargin: 2
                anchors.verticalCenter: parent.verticalCenter
                width: (parent.width - 4) * root.percentage / 100
                height: parent.height - 4
                radius: 1
                color: root.accentColor
                clip: true

                // 填充区域使用深色文字，未填充区域保留浅色，避免电量变化时文字失去对比度。
                Text {
                    x: batteryLabel.x - batteryFill.x
                    y: batteryLabel.y - batteryFill.y
                    text: batteryLabel.text
                    font: batteryLabel.font
                    color: Colors.base
                    Accessible.ignored: true
                }
            }
        }

        Rectangle {
            anchors.left: batteryBody.right
            anchors.leftMargin: 1
            anchors.verticalCenter: parent.verticalCenter
            width: 2
            height: 6
            radius: 1
            color: batteryBody.border.color
        }
    }
}
