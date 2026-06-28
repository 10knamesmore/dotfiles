import "../theme"
import QtQuick
import QtQuick.Controls

// 轻量下拉框：一个字段 + 点击弹出的选项列表。匹配仓库配色，避免 ComboBox 原生样式。
Item {
    id: root

    property var model: []        // 字符串数组
    property string currentText: ""
    property bool enabled: true
    signal activated(string value)

    implicitHeight: 32
    implicitWidth: 120

    Rectangle {
        id: field
        anchors.fill: parent
        radius: Tokens.radiusS
        color: root.enabled ? (fieldArea.containsMouse ? Colors.surface1 : Colors.surface0) : Colors.surface0
        opacity: root.enabled ? 1 : 0.5
        border.width: 1
        border.color: popup.visible ? Colors.blue : Qt.rgba(1, 1, 1, 0.08)

        Text {
            anchors.left: parent.left
            anchors.right: arrow.left
            anchors.verticalCenter: parent.verticalCenter
            anchors.leftMargin: Tokens.spaceS
            elide: Text.ElideRight
            text: root.currentText
            color: Colors.text
            font.family: Fonts.family
            font.pixelSize: Fonts.small
        }

        Text {
            id: arrow
            anchors.right: parent.right
            anchors.rightMargin: Tokens.spaceS
            anchors.verticalCenter: parent.verticalCenter
            text: "▾"
            color: Colors.subtext0
            font.pixelSize: Fonts.xs
        }

        MouseArea {
            id: fieldArea
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: root.enabled ? Qt.PointingHandCursor : Qt.ArrowCursor
            onClicked: {
                if (!root.enabled)
                    return;
                if (popup.visible)
                    popup.close();
                else
                    popup.open();
            }
        }

        Behavior on color { ColorAnimation { duration: Tokens.animFast } }
        Behavior on border.color { ColorAnimation { duration: Tokens.animFast } }
    }

    Popup {
        id: popup
        y: field.height + 4
        width: field.width
        padding: 4
        implicitHeight: Math.min(contentItem.implicitHeight + 8, 220)

        background: Rectangle {
            radius: Tokens.radiusS
            color: Colors.surface0
            border.width: 1
            border.color: Qt.rgba(1, 1, 1, 0.1)
        }

        contentItem: ListView {
            implicitHeight: contentHeight
            clip: true
            model: root.model
            delegate: Rectangle {
                width: ListView.view.width
                height: 28
                radius: Tokens.radiusXS
                color: itemArea.containsMouse ? Colors.surface1 : "transparent"
                Text {
                    anchors.left: parent.left
                    anchors.leftMargin: Tokens.spaceS
                    anchors.verticalCenter: parent.verticalCenter
                    text: modelData
                    color: modelData === root.currentText ? Colors.blue : Colors.text
                    font.family: Fonts.family
                    font.pixelSize: Fonts.small
                }
                MouseArea {
                    id: itemArea
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: {
                        root.activated(modelData);
                        popup.close();
                    }
                }
            }
        }
    }
}
