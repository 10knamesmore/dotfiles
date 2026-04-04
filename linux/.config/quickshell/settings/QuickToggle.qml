import "../theme"
import QtQuick
import QtQuick.Layouts

// 可复用的快捷开关组件
Rectangle {
    id: root

    property string icon: ""
    property string label: ""
    property string status: ""
    property bool toggled: false

    signal clicked()
    signal rightClicked()

    Layout.fillWidth: true
    implicitHeight: toggleCol.implicitHeight + 16
    radius: Tokens.radiusM
    color: toggled ? Qt.rgba(Colors.blue.r, Colors.blue.g, Colors.blue.b, toggleHover.containsMouse ? 0.25 : 0.15) : toggleHover.containsMouse ? Colors.surface1 : Colors.surface0
    border.color: toggled ? Qt.rgba(Colors.blue.r, Colors.blue.g, Colors.blue.b, toggleHover.containsMouse ? 0.5 : Tokens.borderHoverAlpha) : toggleHover.containsMouse ? Qt.rgba(1, 1, 1, 0.08) : "transparent"
    border.width: 1
    scale: toggleHover.containsMouse ? 1.03 : 1

    ColumnLayout {
        id: toggleCol

        anchors.fill: parent
        anchors.margins: Tokens.spaceS
        spacing: 2

        Text {
            text: root.icon
            color: root.toggled ? Colors.blue : Colors.overlay1
            font.family: Fonts.family
            font.pixelSize: Fonts.iconLarge

            Behavior on color {
                ColorAnimation {
                    duration: Tokens.animFast
                    easing.type: Easing.BezierSpline
                    easing.bezierCurve: Anim.standard
                }

            }

        }

        Text {
            text: root.label
            color: Colors.text
            font.family: Fonts.family
            font.pixelSize: Fonts.small
            font.weight: Font.DemiBold
        }

        Text {
            text: root.status
            color: Colors.subtext0
            font.family: Fonts.family
            font.pixelSize: Fonts.xs
            elide: Text.ElideRight
            Layout.fillWidth: true
        }

    }

    MouseArea {
        id: toggleHover

        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        acceptedButtons: Qt.LeftButton | Qt.RightButton
        onClicked: (mouse) => {
            if (mouse.button === Qt.RightButton)
                root.rightClicked();
            else
                root.clicked();
        }
    }

    Behavior on color {
        ColorAnimation {
            duration: Tokens.animFast
            easing.type: Easing.BezierSpline
            easing.bezierCurve: Anim.standard
        }

    }

    Behavior on border.color {
        ColorAnimation {
            duration: Tokens.animFast
            easing.type: Easing.BezierSpline
            easing.bezierCurve: Anim.standard
        }

    }

    Behavior on scale {
        NumberAnimation {
            duration: Tokens.animNormal
            easing.type: Easing.BezierSpline
            easing.bezierCurve: Anim.elastic
        }

    }

}
