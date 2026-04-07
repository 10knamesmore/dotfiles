import "../theme"
import QtQuick
import Quickshell
import Quickshell.Wayland

// 桌面浮动仿真时钟 — Canvas 绘制
PanelWindow {
    id: root

    required property var modelData
    screen: modelData

    // ── 布局配置（改这里调整位置和大小）──
    property int widgetX: 20        // 左边距
    property int widgetY: 60        // 上边距
    property int clockSize: 500     // 时钟直径

    property real _hours: 0
    property real _minutes: 0
    property real _seconds: 0

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
            canvas.requestPaint();
        }
    }

    // 表盘背景（用于 SoftShadow）
    Rectangle {
        id: clockBg
        anchors.centerIn: parent
        width: root.clockSize
        height: root.clockSize
        radius: root.clockSize / 2
        color: Qt.rgba(Colors.base.r, Colors.base.g, Colors.base.b, Tokens.panelAlpha)
        border.color: Qt.rgba(1, 1, 1, Tokens.borderAlpha)
        border.width: 1

        SoftShadow {
            anchors.fill: parent
            radius: parent.radius
        }

        InnerGlow {}
    }

    Canvas {
        id: canvas

        anchors.centerIn: parent
        width: root.clockSize
        height: root.clockSize

        onPaint: {
            let ctx = getContext("2d");
            let w = width;
            let h = height;
            let cx = w / 2;
            let cy = h / 2;
            let r = Math.min(cx, cy) - 4;

            ctx.clearRect(0, 0, w, h);

            // ── 刻度 ──
            for (let i = 0; i < 60; i++) {
                let angle = (i * 6 - 90) * Math.PI / 180;
                let isHour = (i % 5 === 0);
                let innerR = isHour ? r - 12 : r - 6;
                let outerR = r - 2;

                ctx.beginPath();
                ctx.moveTo(cx + innerR * Math.cos(angle), cy + innerR * Math.sin(angle));
                ctx.lineTo(cx + outerR * Math.cos(angle), cy + outerR * Math.sin(angle));
                ctx.strokeStyle = isHour ? Colors.text.toString() : Colors.overlay0.toString();
                ctx.lineWidth = isHour ? 2 : 1;
                ctx.lineCap = "round";
                ctx.stroke();
            }

            // ── 数字（12/3/6/9）──
            ctx.font = "bold 11px 'Hack Nerd Font'";
            ctx.fillStyle = Colors.subtext0.toString();
            ctx.textAlign = "center";
            ctx.textBaseline = "middle";
            let nums = [
                {
                    n: "12",
                    a: -90
                },
                {
                    n: "3",
                    a: 0
                },
                {
                    n: "6",
                    a: 90
                },
                {
                    n: "9",
                    a: 180
                }
            ];
            for (let item of nums) {
                let a = item.a * Math.PI / 180;
                let nr = r - 22;
                ctx.fillText(item.n, cx + nr * Math.cos(a), cy + nr * Math.sin(a));
            }

            // ── 时针 ──
            let hourAngle = (root._hours * 30 - 90) * Math.PI / 180;
            ctx.beginPath();
            ctx.moveTo(cx - 8 * Math.cos(hourAngle), cy - 8 * Math.sin(hourAngle));
            ctx.lineTo(cx + (r * 0.5) * Math.cos(hourAngle), cy + (r * 0.5) * Math.sin(hourAngle));
            ctx.strokeStyle = Colors.text.toString();
            ctx.lineWidth = 3.5;
            ctx.lineCap = "round";
            ctx.stroke();

            // ── 分针 ──
            let minAngle = (root._minutes * 6 - 90) * Math.PI / 180;
            ctx.beginPath();
            ctx.moveTo(cx - 8 * Math.cos(minAngle), cy - 8 * Math.sin(minAngle));
            ctx.lineTo(cx + (r * 0.7) * Math.cos(minAngle), cy + (r * 0.7) * Math.sin(minAngle));
            ctx.strokeStyle = Colors.text.toString();
            ctx.lineWidth = 2.5;
            ctx.lineCap = "round";
            ctx.stroke();

            // ── 秒针 ──
            let secAngle = (root._seconds * 6 - 90) * Math.PI / 180;
            ctx.beginPath();
            ctx.moveTo(cx - 12 * Math.cos(secAngle), cy - 12 * Math.sin(secAngle));
            ctx.lineTo(cx + (r * 0.78) * Math.cos(secAngle), cy + (r * 0.78) * Math.sin(secAngle));
            ctx.strokeStyle = Colors.red.toString();
            ctx.lineWidth = 1.2;
            ctx.lineCap = "round";
            ctx.stroke();

            // ── 中心圆点 ──
            ctx.beginPath();
            ctx.arc(cx, cy, 4, 0, 2 * Math.PI);
            ctx.fillStyle = Colors.mauve.toString();
            ctx.fill();

            // 内圆
            ctx.beginPath();
            ctx.arc(cx, cy, 2, 0, 2 * Math.PI);
            ctx.fillStyle = Colors.base.toString();
            ctx.fill();
        }
    }

}
