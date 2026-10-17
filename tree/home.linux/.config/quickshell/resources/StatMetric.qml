import "../theme"
import QtQuick

Column {
    id: root

    required property string label
    property string unit: ""
    required property string value
    property color valueColor: Colors.text
    property int valueSize: Fonts.display2

    Accessible.name: label + "，" + value + " " + unit
    Accessible.role: Accessible.StaticText
    spacing: Tokens.spaceXS

    Text {
        Accessible.ignored: true
        color: Colors.subtext0
        font.family: Fonts.family
        font.pixelSize: Fonts.caption
        text: root.label
    }
    Row {
        spacing: Tokens.spaceXS

        Text {
            id: number

            Accessible.ignored: true
            color: root.valueColor
            font.family: Fonts.family
            font.pixelSize: root.valueSize
            text: root.value
        }
        Text {
            Accessible.ignored: true
            anchors.baseline: number.baseline
            color: Colors.subtext0
            font.family: Fonts.family
            font.pixelSize: Fonts.body
            text: root.unit
            visible: text !== ""
        }
    }
}
