import "../theme"
import QtQuick
import QtQuick.Layouts

// 紧凑连接入口；摘要和 IP 直接可读，hover 只做小幅高亮与缩放。
Rectangle {
    id: root

    required property string icon
    required property string label
    required property string status
    property string detail: ""
    property bool connected: false
    signal clicked

    Layout.fillWidth: true
    Layout.fillHeight: true
    implicitHeight: 88
    radius: Tokens.radiusM
    color: connected ? Colors.withAlpha(Colors.blue, hover.containsMouse ? 0.23 : 0.13) : Colors.withAlpha(Colors.surface1, hover.containsMouse ? 0.8 : 0.45)
    border.width: 1
    border.color: hover.containsMouse ? Colors.withAlpha(Colors.blue, 0.4) : Colors.overlay(0.05)
    scale: hover.containsMouse ? 1.015 : 1
    activeFocusOnTab: true
    Accessible.role: Accessible.Button
    Accessible.name: label
    Accessible.description: status + (detail !== "" ? "，" + detail : "")
    Accessible.onPressAction: clicked()
    Keys.onReturnPressed: clicked()
    Keys.onSpacePressed: clicked()

    RowLayout {
        anchors.fill: parent
        anchors.margins: 12
        spacing: 9

        Text {
            text: root.icon
            color: root.connected ? Colors.blue : Colors.overlay1
            font.family: Fonts.family
            font.pixelSize: Fonts.iconLarge
        }

        ColumnLayout {
            Layout.fillWidth: true
            spacing: 3
            Text {
                text: root.label
                color: Colors.text
                font.family: Fonts.family
                font.pixelSize: Fonts.body
                font.weight: Font.DemiBold
            }
            Text {
                Layout.fillWidth: true
                text: root.status
                color: Colors.subtext0
                font.family: Fonts.family
                font.pixelSize: Fonts.small
                elide: Text.ElideRight
            }
            Text {
                Layout.fillWidth: true
                visible: root.detail !== ""
                text: root.detail
                color: Colors.subtext0
                font.family: Fonts.family
                font.pixelSize: Fonts.xs
                elide: Text.ElideRight
            }
        }

        Text {
            text: "›"
            color: Colors.overlay1
            font.family: Fonts.family
            font.pixelSize: Fonts.title
        }
    }

    MouseArea {
        id: hover
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        onClicked: root.clicked()
    }

    Behavior on color {
        ColorAnimation {
            duration: Tokens.animFast
        }
    }
    Behavior on border.color {
        ColorAnimation {
            duration: Tokens.animFast
        }
    }
    Behavior on scale {
        NumberAnimation {
            duration: 150
            easing.type: Easing.OutCubic
        }
    }
}
