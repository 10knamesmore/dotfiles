pragma ComponentBehavior: Bound

import "../theme"
import QtQuick

// 三列进程快照；采集器负责排名，固定行实例只更新内容，不随每次采样重建。
Column {
    id: root

    required property string metricLabel
    readonly property int pidWidth: 66
    property var rows: [] // [{pid, name, value}]
    readonly property int valueWidth: 96

    Accessible.role: Accessible.Table
    spacing: 0
    visible: rows.length > 0

    RowItem {
        header: true
        processId: "PID"
        processName: "进程"
        value: root.metricLabel
    }
    Repeater {
        model: root.rows.length

        delegate: RowItem {
            required property int index

            processId: String(root.rows[index].pid)
            processName: root.rows[index].name
            value: root.rows[index].value
        }
    }

    component RowItem: Item {
        id: row

        property bool header: false
        required property string processId
        required property string processName
        required property string value

        Accessible.name: processName + "，PID " + processId + "，" + root.metricLabel + " " + value
        Accessible.role: Accessible.Row
        height: header ? 24 : 30
        width: root.width

        Text {
            Accessible.ignored: true
            anchors.verticalCenter: parent.verticalCenter
            color: row.header ? Colors.subtext0 : Colors.text
            elide: Text.ElideRight
            font.family: Fonts.family
            font.pixelSize: row.header ? Fonts.small : Fonts.body
            text: row.processName
            width: row.width - root.pidWidth - root.valueWidth - Tokens.spaceS
        }
        Text {
            Accessible.ignored: true
            anchors.verticalCenter: parent.verticalCenter
            color: Colors.subtext0
            font.family: Fonts.family
            font.pixelSize: row.header ? Fonts.small : Fonts.body
            horizontalAlignment: Text.AlignRight
            text: row.processId
            width: root.pidWidth
            x: row.width - root.pidWidth - root.valueWidth
        }
        Text {
            Accessible.ignored: true
            anchors.right: parent.right
            anchors.verticalCenter: parent.verticalCenter
            color: row.header ? Colors.subtext0 : Colors.text
            font.family: Fonts.family
            font.pixelSize: row.header ? Fonts.small : Fonts.body
            text: row.value
        }
    }
}
