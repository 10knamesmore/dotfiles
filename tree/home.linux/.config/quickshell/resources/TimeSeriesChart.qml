import "../theme"
import QtQuick

// 最近一分钟的真实时间轴；100ms 与 1s 采样共享时间坐标，短历史不补零。
Item {
    id: root

    property color lineColor: Colors.blue
    property real maxValue: 100
    property int plotHeight: 80
    property var points: [] // [{time: Unix秒, value: 指标值}]

    Accessible.role: Accessible.Chart
    implicitHeight: plotHeight + Tokens.spaceS + leftTick.implicitHeight

    onLineColorChanged: plot.requestPaint()
    onMaxValueChanged: plot.requestPaint()
    onPointsChanged: plot.requestPaint()

    Canvas {
        id: plot

        antialiasing: true
        height: root.plotHeight
        width: parent.width

        onHeightChanged: requestPaint()
        onPaint: {
            const ctx = getContext("2d");
            ctx.clearRect(0, 0, width, height);
            if (!root.points.length || root.maxValue <= 0)
                return;
            const end = root.points[root.points.length - 1].time;
            const points = root.points.filter(point => point.time >= end - 60);
            const x = time => 2 + (width - 8) * (time - end + 60) / 60;
            const y = value => 4 + (height - 8) * (1 - Math.min(1, value / root.maxValue));

            ctx.setLineDash([3, 4]);
            ctx.strokeStyle = Colors.overlay(0.08);
            ctx.lineWidth = 1;
            for (let i = 0; i < 3; i++) {
                ctx.beginPath();
                ctx.moveTo(2, 4 + (height - 8) * i / 2);
                ctx.lineTo(width - 6, 4 + (height - 8) * i / 2);
                ctx.stroke();
            }
            ctx.setLineDash([]);
            ctx.beginPath();
            ctx.moveTo(x(points[0].time), y(points[0].value));
            for (let i = 1; i < points.length; i++)
                ctx.lineTo(x(points[i].time), y(points[i].value));
            ctx.strokeStyle = root.lineColor;
            ctx.lineWidth = 1.8;
            ctx.lineJoin = "round";
            ctx.stroke();
            const last = points[points.length - 1];
            ctx.lineTo(x(last.time), height - 4);
            ctx.lineTo(x(points[0].time), height - 4);
            ctx.closePath();
            const fill = ctx.createLinearGradient(0, 0, 0, height);
            fill.addColorStop(0, Colors.withAlpha(root.lineColor, 0.2));
            fill.addColorStop(1, Colors.withAlpha(root.lineColor, 0.02));
            ctx.fillStyle = fill;
            ctx.fill();
            ctx.beginPath();
            ctx.arc(x(last.time), y(last.value), 2.5, 0, 2 * Math.PI);
            ctx.fillStyle = root.lineColor;
            ctx.fill();
        }
        onVisibleChanged: if (visible)
            requestPaint()
        onWidthChanged: requestPaint()
    }
    Text {
        id: leftTick

        anchors.left: parent.left
        color: Colors.subtext0
        font.family: Fonts.family
        font.pixelSize: Fonts.caption
        text: "−60s"
        y: root.plotHeight + Tokens.spaceS
    }
    Text {
        anchors.horizontalCenter: parent.horizontalCenter
        color: Colors.subtext0
        font.family: Fonts.family
        font.pixelSize: Fonts.caption
        text: "−30s"
        y: leftTick.y
    }
    Text {
        anchors.right: parent.right
        color: Colors.subtext0
        font.family: Fonts.family
        font.pixelSize: Fonts.caption
        text: "现在"
        y: leftTick.y
    }
}
