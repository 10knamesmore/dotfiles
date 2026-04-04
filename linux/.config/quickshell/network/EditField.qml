import "../theme"
import QtQuick
import QtQuick.Layouts

// 可复用的带标签文本输入字段
ColumnLayout {
    id: root

    property string label: ""
    property string placeholder: ""
    property string text: ""

    signal edited(string text)

    Layout.fillWidth: true
    spacing: 2

    Text {
        text: root.label
        color: Colors.subtext0
        font.family: Fonts.family
        font.pixelSize: Fonts.caption
        Layout.leftMargin: 4
    }

    Rectangle {
        Layout.fillWidth: true
        height: 32
        radius: 8
        color: Colors.surface1

        TextInput {
            id: input

            anchors.fill: parent
            anchors.leftMargin: 8
            anchors.rightMargin: 8
            verticalAlignment: TextInput.AlignVCenter
            color: Colors.text
            font.family: Fonts.family
            font.pixelSize: Fonts.body
            clip: true
            selectByMouse: true
            text: root.text
            onTextChanged: {
                if (text !== root.text)
                    root.edited(text);

            }

            Text {
                anchors.fill: parent
                verticalAlignment: Text.AlignVCenter
                text: root.placeholder
                color: Colors.overlay0
                font: parent.font
                visible: !parent.text && !parent.activeFocus
            }

        }

    }

}
