import "../../theme"
import QtQuick
import QtQuick.Layouts

// 天气分区卡片；内容按列排列，标题右侧可放操作按钮。
Rectangle {
    id: root

    property string title: ""
    property string note: ""
    default property alias content: contentColumn.data
    property alias actions: actionsRow.data

    implicitHeight: sectionColumn.implicitHeight + 24
    color: Colors.withAlpha(Colors.crust, 0.3)
    radius: Tokens.radiusMS
    border.width: 1
    border.color: Colors.overlay(0.04)

    data: ColumnLayout {
        id: sectionColumn

        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.margins: 12
        spacing: 8

        RowLayout {
            Layout.fillWidth: true
            spacing: 6
            visible: root.title !== ""

            Text {
                Layout.fillWidth: true
                text: root.title
                color: Colors.subtext1
                font.family: Fonts.family
                font.pixelSize: Fonts.small
                font.weight: Font.DemiBold
            }

            Text {
                visible: root.note !== ""
                text: root.note
                color: Colors.subtext0
                font.family: Fonts.family
                font.pixelSize: Fonts.xs
            }

            RowLayout {
                id: actionsRow

                spacing: 2
            }

        }

        ColumnLayout {
            id: contentColumn

            Layout.fillWidth: true
            spacing: 8
        }

    }

}
