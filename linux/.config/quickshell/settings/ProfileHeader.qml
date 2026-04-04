import QtQuick
import QtQuick.Layouts
import Quickshell
import "../theme"

// 用户头像 + 用户名 + 签名
RowLayout {
    id: root

    property string motto: "Don't let me change for anything"

    spacing: 12

    // 圆形头像
    Rectangle {
        width: 48
        height: 48
        radius: 24
        color: Colors.surface1
        clip: true

        Image {
            id: avatar
            anchors.fill: parent
            source: "file://" + Quickshell.env("HOME") + "/Pictures/avatar.jpg"
            fillMode: Image.PreserveAspectCrop
            visible: status === Image.Ready
        }

        // fallback 图标
        Text {
            anchors.centerIn: parent
            text: "󰀄"
            color: Colors.overlay1
            font.family: "Hack Nerd Font"
            font.pixelSize: 22
            visible: avatar.status !== Image.Ready
        }
    }

    // 用户名 + 签名
    ColumnLayout {
        Layout.fillWidth: true
        spacing: 2

        Text {
            text: Quickshell.env("USER")
            color: Colors.text
            font.family: "Hack Nerd Font"
            font.pixelSize: 15
            font.weight: Font.Bold
        }

        Text {
            text: root.motto
            color: Colors.subtext0
            font.family: "Hack Nerd Font"
            font.pixelSize: 11
            elide: Text.ElideRight
            Layout.fillWidth: true
        }
    }
}
