import "../../theme"
import QtQuick
import QtQuick.Layouts
import "WeatherFormat.js" as Format

// 当前环境和今日光照；逐时指标缺失时显示空值，不以零替代。
WeatherSection {
    id: root

    required property var current
    required property var currentHour
    required property var today
    required property double now
    readonly property real sunProgress: today ? Math.max(0, Math.min(1, (now - today.sunrise) / (today.sunset - today.sunrise))) : 0
    readonly property color sunColor: Colors.yellow
    readonly property string sunStatus: {
        if (!today)
            return "今日数据暂缺";

        const beforeRise = now < today.sunrise;
        const afterSet = now > today.sunset;
        const minutes = Math.floor(Math.abs(now - (beforeRise ? today.sunrise : today.sunset)) / 60000);
        const duration = (minutes >= 60 ? Math.floor(minutes / 60) + " 小时 " : "") + minutes % 60 + " 分";
        return (beforeRise ? "距日出 " : afterSet ? "日落已过 " : "距日落 ") + duration;
    }

    title: "当前环境"
    onSunProgressChanged: sunArc.requestPaint()
    onSunColorChanged: sunArc.requestPaint()

    GridLayout {
        Layout.fillWidth: true
        columns: 2
        columnSpacing: 7
        rowSpacing: 7

        Metric {
            label: Format.windDirection(root.current.windDirection)
            icon: "󰈐"
            value: Format.metersPerSecond(root.current.windSpeed)
            unit: "m/s"
            detail: "阵风 " + Format.metersPerSecond(root.current.windGust) + " m/s"
        }

        Metric {
            label: "相对湿度"
            icon: "󰍝"
            value: Format.number(root.current.humidity)
            unit: "%"
            detail: root.current.humidity === null ? "暂无数据" : root.current.humidity < 40 ? "空气偏干" : root.current.humidity > 70 ? "空气湿润" : "湿度适中"
        }

        Metric {
            label: "紫外线"
            icon: "󰖙"
            value: Format.number(root.currentHour ? root.currentHour.uvIndex : null, 1)
            detail: root.currentHour ? Format.uvLevel(root.currentHour.uvIndex) + (root.today ? " · 今日最高 " + Format.number(root.today.uvMax, 1) : "") : "暂无数据"
        }

        Metric {
            label: "能见度"
            icon: "󰈈"
            value: Format.number(root.currentHour && root.currentHour.visibility !== null ? root.currentHour.visibility / 1000 : null, 1)
            unit: "km"
            detail: root.currentHour ? Format.timeOfDay(root.currentHour.time) + " 预报" : "暂无数据"
        }

    }

    Rectangle {
        Layout.fillWidth: true
        Layout.topMargin: 2
        height: 1
        color: Colors.overlay(0.06)
    }

    Text {
        text: "日出与日落"
        color: Colors.subtext1
        font.family: Fonts.family
        font.pixelSize: Fonts.small
    }

    Text {
        text: root.sunStatus
        color: Colors.subtext0
        font.family: Fonts.family
        font.pixelSize: Fonts.xs
    }

    Canvas {
        id: sunArc

        Layout.fillWidth: true
        Layout.preferredHeight: 42
        antialiasing: true
        Accessible.ignored: true
        onWidthChanged: requestPaint()
        onVisibleChanged: {
            if (visible) {
                requestPaint();
            }
        }
        onPaint: {
            const ctx = getContext("2d");
            ctx.clearRect(0, 0, width, height);
            if (!root.today)
                return ;

            const x = (t) => {
                return 12 + (width - 24) * t;
            };
            const y = (t) => {
                return height - 6 - Math.sin(t * Math.PI) * (height - 12);
            };
            ctx.lineWidth = 1.5;
            ctx.setLineDash([3, 4]);
            ctx.strokeStyle = Colors.withAlpha(root.sunColor, 0.25);
            ctx.beginPath();
            ctx.moveTo(x(0), y(0));
            for (let t = 0.025; t <= 1.001; t += 0.025) ctx.lineTo(x(t), y(t))
            ctx.stroke();
            ctx.setLineDash([]);
            if (root.now >= root.today.sunrise && root.now <= root.today.sunset) {
                ctx.strokeStyle = root.sunColor;
                ctx.beginPath();
                ctx.moveTo(x(0), y(0));
                for (let t = 0.025; t < root.sunProgress; t += 0.025) ctx.lineTo(x(t), y(t))
                ctx.lineTo(x(root.sunProgress), y(root.sunProgress));
                ctx.stroke();
            }
            ctx.beginPath();
            ctx.arc(x(root.sunProgress), y(root.sunProgress), 3, 0, Math.PI * 2);
            ctx.fillStyle = root.sunColor;
            ctx.fill();
        }
    }

    RowLayout {
        Layout.fillWidth: true

        Text {
            text: "日出 " + Format.timeOfDay(root.today ? root.today.sunrise : null)
            color: Colors.subtext0
            font.family: Fonts.family
            font.pixelSize: Fonts.caption
        }

        Item {
            Layout.fillWidth: true
        }

        Text {
            text: "日落 " + Format.timeOfDay(root.today ? root.today.sunset : null)
            color: Colors.subtext0
            font.family: Fonts.family
            font.pixelSize: Fonts.caption
        }

    }

    component Metric: Rectangle {
        id: metric

        required property string label
        required property string icon
        required property string value
        property string unit: ""
        required property string detail

        Layout.fillWidth: true
        Layout.preferredWidth: 1
        implicitHeight: metricColumn.implicitHeight + 16
        radius: Tokens.radiusS
        color: Colors.withAlpha(Colors.crust, 0.25)
        Accessible.role: Accessible.StaticText
        Accessible.name: label + "，" + value + " " + unit + "，" + detail

        ColumnLayout {
            id: metricColumn

            anchors.left: parent.left
            anchors.right: parent.right
            anchors.top: parent.top
            anchors.margins: 8
            spacing: 4
            Accessible.ignored: true

            Text {
                Layout.fillWidth: true
                text: metric.icon + " " + metric.label
                color: Colors.subtext0
                font.family: Fonts.family
                font.pixelSize: Fonts.caption
            }

            RowLayout {
                spacing: 3

                Text {
                    text: metric.value
                    color: Colors.text
                    font.family: Fonts.family
                    font.pixelSize: Fonts.h3
                }

                Text {
                    Layout.alignment: Qt.AlignBottom
                    Layout.bottomMargin: 3
                    text: metric.unit
                    color: Colors.subtext0
                    font.family: Fonts.family
                    font.pixelSize: Fonts.xs
                }

            }

            Text {
                Layout.fillWidth: true
                text: metric.detail
                color: Colors.subtext0
                font.family: Fonts.family
                font.pixelSize: Fonts.xs
                wrapMode: Text.Wrap
            }

        }

    }

}
