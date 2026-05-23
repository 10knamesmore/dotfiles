import "../theme"
import QtQuick
import Quickshell
import Quickshell.Wayland

// 桌面浮动仿真时钟
//
// 性能要点：完全用 Rectangle + Rotation transform，不用 Canvas。
//   - 60 根刻度 / 4 个数字静态排列一次，永不重绘
//   - 3 根针只更新 rotation.angle，scene graph 走 GPU 矩阵合成
//   - 1Hz 更新仅触发 3 个 binding，无 requestPaint
PanelWindow {
    id: root

    required property var modelData
    screen: modelData

    // ── 布局配置（改这里调整位置和大小）──
    property int widgetX: 20
    property int widgetY: 60
    property int clockSize: 500

    property real _hours: 0
    property real _minutes: 0
    property real _seconds: 0

    readonly property real _radius: clockSize / 2

    aboveWindows: false
    anchors.top: true
    anchors.left: true
    implicitWidth: clockSize + 40
    implicitHeight: clockSize + 40
    margins.top: widgetY
    margins.left: widgetX
    visible: PanelState.analogClockVisible
    focusable: false
    exclusionMode: ExclusionMode.Ignore
    color: "transparent"

    Timer {
        interval: 1000
        running: root.visible
        repeat: true
        triggeredOnStart: true
        onTriggered: {
            let now = new Date();
            root._seconds = now.getSeconds() + now.getMilliseconds() / 1000;
            root._minutes = now.getMinutes() + root._seconds / 60;
            root._hours = (now.getHours() % 12) + root._minutes / 60;
        }
    }

    // 表盘背景
    Rectangle {
        id: clockBg
        anchors.centerIn: parent
        width: root.clockSize
        height: root.clockSize
        radius: root._radius
        color: Qt.rgba(Colors.base.r, Colors.base.g, Colors.base.b, Tokens.panelAlpha)
        border.color: Qt.rgba(1, 1, 1, Tokens.borderAlpha)
        border.width: 1

        SoftShadow {
            anchors.fill: parent
            radius: parent.radius
        }

        InnerGlow {}
    }

    // 表盘 dial：所有指针/刻度/数字都以它的中心为锚
    Item {
        id: dial
        anchors.centerIn: parent
        width: root.clockSize
        height: root.clockSize

        // ── 60 根刻度（每个都是 dial 高度的 Item，绕中心旋转，顶端贴 Rectangle）──
        Repeater {
            model: 60

            delegate: Item {
                required property int index
                readonly property bool isHour: index % 5 === 0

                anchors.horizontalCenter: dial.horizontalCenter
                anchors.verticalCenter: dial.verticalCenter
                width: 2
                height: root.clockSize - 8
                rotation: index * 6

                Rectangle {
                    anchors.top: parent.top
                    anchors.horizontalCenter: parent.horizontalCenter
                    width: parent.isHour ? 2 : 1
                    height: parent.isHour ? 10 : 6
                    color: parent.isHour ? Colors.text : Colors.overlay0
                    radius: width / 2
                }
            }
        }

        // ── 数字 12 / 3 / 6 / 9 ──
        Text {
            anchors.horizontalCenter: dial.horizontalCenter
            anchors.top: dial.top
            anchors.topMargin: 18
            text: "12"
            color: Colors.subtext0
            font.family: "Hack Nerd Font"
            font.pixelSize: 11
            font.bold: true
        }
        Text {
            anchors.verticalCenter: dial.verticalCenter
            anchors.right: dial.right
            anchors.rightMargin: 22
            text: "3"
            color: Colors.subtext0
            font.family: "Hack Nerd Font"
            font.pixelSize: 11
            font.bold: true
        }
        Text {
            anchors.horizontalCenter: dial.horizontalCenter
            anchors.bottom: dial.bottom
            anchors.bottomMargin: 18
            text: "6"
            color: Colors.subtext0
            font.family: "Hack Nerd Font"
            font.pixelSize: 11
            font.bold: true
        }
        Text {
            anchors.verticalCenter: dial.verticalCenter
            anchors.left: dial.left
            anchors.leftMargin: 22
            text: "9"
            color: Colors.subtext0
            font.family: "Hack Nerd Font"
            font.pixelSize: 11
            font.bold: true
        }

        // ── 指针 pivot：0x0 Item，centerIn dial，rotation 绕 (0,0)；子 Rectangle 在 pivot 上方延伸 ──
        component Hand: Item {
            id: handRoot
            property real length: 100   // 针从中心向上伸出的长度
            property real tail: 8       // 针超过中心向下的尾部
            property real thickness: 3
            property color tint: Colors.text

            anchors.centerIn: parent
            width: 0
            height: 0

            Rectangle {
                x: -handRoot.thickness / 2
                y: -handRoot.length
                width: handRoot.thickness
                height: handRoot.length + handRoot.tail
                radius: handRoot.thickness / 2
                color: handRoot.tint
            }
        }

        Hand {
            length: root._radius * 0.5
            thickness: 3.5
            tint: Colors.text
            rotation: root._hours * 30
        }

        Hand {
            length: root._radius * 0.7
            thickness: 2.5
            tint: Colors.text
            rotation: root._minutes * 6
        }

        Hand {
            length: root._radius * 0.78
            tail: 12
            thickness: 1.2
            tint: Colors.red
            rotation: root._seconds * 6
        }

        // ── 中心圆点 ──
        Rectangle {
            anchors.centerIn: dial
            width: 8
            height: 8
            radius: 4
            color: Colors.mauve
        }
        Rectangle {
            anchors.centerIn: dial
            width: 4
            height: 4
            radius: 2
            color: Colors.base
        }
    }
}
