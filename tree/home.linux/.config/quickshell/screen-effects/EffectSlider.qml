import "../theme"
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

// 单个效果滑块：标签 + 滑条 + 数值
//
// moved 信号经 150ms 节流：每次应用都要让 Hyprland 重新编译 shader，而编译发生在
// 渲染线程的 begin() 里、会阻塞那一帧。0→100 拖一趟原本触发 20 次，节流后 2-3 次。
//
// ⚠ 是 throttle（leading + trailing）不是纯 debounce，别"简化"成后者：纯 debounce
// 会让拖动全程画面不动、松手才跳一次，调色滑块就没法拖着看效果了。throttle 的触发
// 次数一样，但拖动中每 150ms 更新一次画面，手感是连续的。
RowLayout {
    id: root

    property string label: ""
    property int value: 0

    signal moved(int val)

    // 节流窗口内的最新值；-1 = 无待发值（滑块值域是 0-100，不会撞上）
    property int _pending: -1

    function _emit(v) {
        if (throttle.running) {
            root._pending = v;
            return;
        }
        root.moved(v);      // leading：首次立即发，保证起手就有反馈
        throttle.start();
    }

    Layout.fillWidth: true
    spacing: 8

    Timer {
        id: throttle

        interval: 150
        onTriggered: {
            if (root._pending < 0)
                return;     // 窗口内无新值 → 自然停下，下次移动又是 leading
            root.moved(root._pending);
            root._pending = -1;
            throttle.restart();
        }
    }

    Text {
        text: root.label
        font.family: Fonts.family
        font.pixelSize: Fonts.bodyLarge
        color: Colors.subtext1
        Layout.preferredWidth: 90
    }

    Slider {
        id: slider

        Layout.fillWidth: true
        from: 0
        to: 100
        stepSize: 5
        value: root.value
        live: true
        onMoved: root._emit(Math.round(value))
        // 松手立刻补发待发值，免得最终值还要再等一个节流窗口才落地
        onPressedChanged: {
            if (!pressed && root._pending >= 0) {
                root.moved(root._pending);
                root._pending = -1;
            }
        }

        // 和外部属性同步（避免绑定环路）
        Connections {
            function onValueChanged() {
                if (!slider.pressed)
                    slider.value = root.value;
            }

            target: root
        }

        background: Rectangle {
            x: slider.leftPadding
            y: slider.topPadding + slider.availableHeight / 2 - height / 2
            implicitWidth: 120
            implicitHeight: 10
            width: slider.availableWidth
            height: 6
            radius: 3
            color: Colors.surface2

            Rectangle {
                width: slider.visualPosition * parent.width
                height: parent.height
                radius: parent.radius
                color: Colors.mauve
            }
        }

        handle: Rectangle {
            x: slider.leftPadding + slider.visualPosition * (slider.availableWidth - width)
            y: slider.topPadding + slider.availableHeight / 2 - height / 2
            implicitWidth: 30
            implicitHeight: 30
            width: 30
            height: 30
            radius: 15
            color: "transparent"

            Rectangle {
                anchors.centerIn: parent
                width: 18
                height: 18
                radius: 9
                color: slider.pressed ? Colors.pink : Colors.text
                border.color: Colors.mauve
                border.width: 2
            }
        }

        MouseArea {
            anchors.fill: parent
            acceptedButtons: Qt.NoButton
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
        }
    }

    Text {
        text: Math.round(slider.value)
        font.family: Fonts.family
        font.pixelSize: Fonts.body
        color: Colors.subtext0
        Layout.preferredWidth: 28
        horizontalAlignment: Text.AlignRight
    }
}
