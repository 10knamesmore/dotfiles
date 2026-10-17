pragma ComponentBehavior: Bound

import "../theme"
import QtQuick

// 保留名称、PID 和数值，以整机容量为分母绘制占用条。
// 采集器负责排名，固定行实例只更新内容，不随每次采样重建。
Column {
    id: root

    property color barColor: Colors.blue
    required property string metricLabel
    readonly property int pidWidth: 66
    property var rows: [] // [{pid, name, value, fraction}]，fraction 为整机 CPU 或物理内存占比。
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

            fraction: root.rows[index].fraction
            processId: String(root.rows[index].pid)
            processName: root.rows[index].name
            value: root.rows[index].value
        }
    }

    component RowItem: Item {
        id: row

        property real fraction: 0
        property bool header: false
        required property string processId
        required property string processName
        required property string value

        Accessible.name: processName + "，PID " + processId + "，" + root.metricLabel + " " + value
        Accessible.role: Accessible.Row
        height: header ? 24 : 44
        width: root.width

        Text {
            Accessible.ignored: true
            color: row.header ? Colors.subtext0 : Colors.text
            elide: Text.ElideRight
            font.family: Fonts.family
            font.pixelSize: row.header ? Fonts.small : Fonts.body
            height: 28
            text: row.processName
            verticalAlignment: Text.AlignVCenter
            width: row.width - root.pidWidth - root.valueWidth - Tokens.spaceS
        }
        Text {
            Accessible.ignored: true
            color: Colors.subtext0
            font.family: Fonts.family
            font.pixelSize: row.header ? Fonts.small : Fonts.body
            height: 28
            horizontalAlignment: Text.AlignRight
            text: row.processId
            verticalAlignment: Text.AlignVCenter
            width: root.pidWidth
            x: row.width - root.pidWidth - root.valueWidth
        }
        Text {
            Accessible.ignored: true
            anchors.right: parent.right
            color: row.header ? Colors.subtext0 : Colors.text
            font.family: Fonts.family
            font.pixelSize: row.header ? Fonts.small : Fonts.body
            height: 28
            text: row.value
            verticalAlignment: Text.AlignVCenter
        }
        Rectangle {
            Accessible.ignored: true
            clip: true
            color: Colors.overlay(0.05)
            height: 4
            radius: 2
            visible: !row.header
            width: parent.width
            y: 30

            Rectangle {
                color: root.barColor
                height: parent.height
                radius: parent.radius
                width: parent.width * row.fraction
            }
        }
    }
}
