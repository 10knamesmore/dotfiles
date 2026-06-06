import "../theme"
import QtQuick
import QtQuick.Layouts

// 配置选择弹出卡片
Rectangle {
    id: picker

    property var configs: []
    property string currentConfigId: ""

    signal configSelected(string configId, string configName)

    height: Math.min(configCol.implicitHeight + 2 * Tokens.spaceM, 300)
    radius: Tokens.radiusMS
    color: Qt.rgba(Colors.mantle.r, Colors.mantle.g, Colors.mantle.b, 0.95)
    border.color: Qt.rgba(1, 1, 1, Tokens.borderAlpha)
    border.width: 1
    z: 10

    // 阻止点击穿透
    MouseArea {
        anchors.fill: parent
        onClicked: mouse => mouse.accepted = true
    }

    Flickable {
        anchors.fill: parent
        anchors.margins: Tokens.spaceM
        contentHeight: configCol.implicitHeight
        clip: true

        Column {
            id: configCol

            width: parent.width
            spacing: 4

            Text {
                text: "选择配置"
                color: Colors.overlay0
                font.family: Fonts.family
                font.pixelSize: Fonts.small
                bottomPadding: 4
            }

            Repeater {
                model: picker.configs

                Rectangle {
                    required property var modelData
                    required property int index

                    width: configCol.width
                    height: 36
                    radius: Tokens.radiusMS
                    color: {
                        if (modelData.id === picker.currentConfigId)
                            return Qt.rgba(Colors.blue.r, Colors.blue.g, Colors.blue.b, 0.15);
                        return cfgArea.containsMouse ? Qt.rgba(1, 1, 1, 0.06) : "transparent";
                    }

                    RowLayout {
                        anchors.fill: parent
                        anchors.leftMargin: 10
                        anchors.rightMargin: 10
                        spacing: Tokens.spaceS

                        Text {
                            Layout.fillWidth: true
                            text: modelData.name
                            color: modelData.id === picker.currentConfigId ? Colors.blue : Colors.text
                            font.family: Fonts.family
                            font.pixelSize: Fonts.body
                            elide: Text.ElideRight
                        }

                        Text {
                            visible: modelData.is_default
                            text: "默认"
                            color: Colors.overlay0
                            font.family: Fonts.family
                            font.pixelSize: Fonts.xs
                        }

                        Text {
                            visible: modelData.id === picker.currentConfigId
                            text: "󰄬"
                            color: Colors.blue
                            font.family: Fonts.family
                            font.pixelSize: Fonts.body
                        }
                    }

                    MouseArea {
                        id: cfgArea
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: picker.configSelected(modelData.id, modelData.name)
                    }

                    Behavior on color {
                        ColorAnimation { duration: Tokens.animFast }
                    }
                }
            }
        }
    }
}
