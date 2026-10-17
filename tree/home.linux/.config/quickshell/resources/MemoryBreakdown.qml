pragma ComponentBehavior: Bound

import "../state/StatsFormat.js" as StatsFormat
import "../theme"
import QtQuick
import Quickshell.Widgets

Column {
    id: root

    required property var memory
    // Available 是可回收容量的估计，不能与 Buffer / Cached 并列相加。
    // 物理内存条按原始 meminfo 分区；剩余部分不等同于 total - available。
    readonly property var segments: [
        {
            label: "其他占用",
            bytes: memory.totalBytes - memory.breakdown.freeBytes - memory.breakdown.buffersBytes - memory.breakdown.cachedBytes,
            color: "#9dd866"
        },
        {
            label: "Buffer",
            bytes: memory.breakdown.buffersBytes,
            color: "#f4a261"
        },
        {
            label: "Cached",
            bytes: memory.breakdown.cachedBytes,
            color: "#64b5f6"
        },
        {
            label: "Free",
            bytes: memory.breakdown.freeBytes,
            color: "#5b6472"
        }
    ]

    spacing: Tokens.spaceM

    ClippingRectangle {
        id: capacityBar

        Accessible.name: "物理内存组成，" + root.segments.map(segment => segment.label + " " + StatsFormat.bytes(segment.bytes)).join("，")
        Accessible.role: Accessible.Chart
        color: "transparent"
        height: 24
        radius: Tokens.radiusXS
        width: parent.width

        Row {
            anchors.fill: parent

            Repeater {
                model: root.memory.totalBytes > 0 ? root.segments.length : 0

                delegate: Rectangle {
                    required property int index

                    Accessible.ignored: true
                    color: root.segments[index].color
                    height: capacityBar.height
                    width: capacityBar.width * root.segments[index].bytes / root.memory.totalBytes
                }
            }
        }
    }
    Grid {
        id: legend

        columnSpacing: Tokens.spaceL
        columns: 2
        rowSpacing: Tokens.spaceS
        width: parent.width

        Repeater {
            model: root.memory.totalBytes > 0 ? root.segments.length : 0

            delegate: Item {
                id: segmentLabel

                required property int index
                readonly property var segment: root.segments[index]

                Accessible.name: segment.label + "，" + StatsFormat.bytes(segment.bytes)
                Accessible.role: Accessible.StaticText
                height: label.implicitHeight
                width: (legend.width - legend.columnSpacing) / 2

                Rectangle {
                    Accessible.ignored: true
                    anchors.verticalCenter: parent.verticalCenter
                    color: segmentLabel.segment.color
                    height: 7
                    radius: 2
                    width: 7
                }
                Text {
                    id: label

                    Accessible.ignored: true
                    color: Colors.subtext0
                    font.family: Fonts.family
                    font.pixelSize: Fonts.small
                    text: segmentLabel.segment.label
                    x: 7 + Tokens.spaceS
                }
                Text {
                    Accessible.ignored: true
                    anchors.right: parent.right
                    color: Colors.text
                    font.family: Fonts.family
                    font.pixelSize: Fonts.small
                    text: StatsFormat.bytes(segmentLabel.segment.bytes)
                }
            }
        }
    }
}
