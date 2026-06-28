import "../theme"
import "../state"
import QtQuick
import QtQuick.Layouts

// apply 后的回滚确认条：倒计时内不「保留」则自动恢复，防错误配置黑屏锁死。
Rectangle {
    id: root

    Layout.fillWidth: true
    visible: MonitorState.revertSecs > 0
    implicitHeight: visible ? row.implicitHeight + Tokens.spaceM * 2 : 0
    radius: Tokens.radiusM
    color: Qt.rgba(Colors.red.r, Colors.red.g, Colors.red.b, 0.14)
    border.width: 1
    border.color: Qt.rgba(Colors.red.r, Colors.red.g, Colors.red.b, 0.5)

    RowLayout {
        id: row
        anchors.fill: parent
        anchors.margins: Tokens.spaceM
        spacing: Tokens.spaceM

        Text {
            Layout.fillWidth: true
            wrapMode: Text.WordWrap
            color: Colors.red
            font.family: Fonts.family
            font.pixelSize: Fonts.small
            text: "保留这些显示设置？" + MonitorState.revertSecs + " 秒后自动恢复"
        }

        // 恢复（立即撤销）
        Rectangle {
            implicitWidth: revertLbl.implicitWidth + Tokens.spaceL
            implicitHeight: 28
            radius: Tokens.radiusS
            color: revertArea.containsMouse ? Colors.surface2 : Colors.surface1
            Text {
                id: revertLbl
                anchors.centerIn: parent
                text: "恢复"
                color: Colors.text
                font.family: Fonts.family
                font.pixelSize: Fonts.small
            }
            MouseArea {
                id: revertArea
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: MonitorState.revert()
            }
            Behavior on color { ColorAnimation { duration: Tokens.animFast } }
        }

        // 保留（提交并记入当前组合）
        Rectangle {
            implicitWidth: keepLbl.implicitWidth + Tokens.spaceL
            implicitHeight: 28
            radius: Tokens.radiusS
            color: keepArea.containsMouse ? Qt.lighter(Colors.green, 1.1) : Colors.green
            Text {
                id: keepLbl
                anchors.centerIn: parent
                text: "保留"
                color: Colors.base
                font.family: Fonts.family
                font.pixelSize: Fonts.small
                font.bold: true
            }
            MouseArea {
                id: keepArea
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: MonitorState.keep()
            }
            Behavior on color { ColorAnimation { duration: Tokens.animFast } }
        }
    }

    Behavior on implicitHeight { NumberAnimation { duration: Tokens.animNormal; easing.type: Easing.OutCubic } }
}
