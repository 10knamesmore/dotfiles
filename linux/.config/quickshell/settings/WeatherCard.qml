import "../services"
import "../theme"
import QtQuick
import QtQuick.Layouts
import Quickshell

// 天气卡片 — 数据来自 WeatherService，可展开显示日出日落弧 + 逐时 + 7 天预报。
// 视觉：卡面带随天气状况变色的氛围渐变，图标/弧线同色呼应。
Rectangle {
    id: root

    property bool expanded: false
    // 当前天气状况的主题色（雨→蓝 晴昼→黄 晴夜→淡紫 雪→sky 雷→mauve 雾→overlay）
    readonly property color cond: condColor(WeatherService.weatherCode, WeatherService.isDay)

    function condColor(code, day) {
        if (code <= 3) return day ? Colors.yellow : Colors.lavender;
        if (code <= 49) return Colors.overlay1; // 雾
        if (code <= 69) return Colors.blue; // 雨
        if (code <= 79) return Colors.sky; // 雪
        if (code <= 84) return Colors.blue; // 阵雨
        if (code <= 86) return Colors.sky; // 阵雪
        return Colors.mauve; // 雷暴
    }

    function aqiColor(v) {
        if (v <= 50) return Colors.green;
        if (v <= 100) return Colors.yellow;
        if (v <= 150) return Colors.peach;
        if (v <= 200) return Colors.red;
        if (v <= 300) return Colors.maroon;
        return Colors.mauve;
    }

    Layout.fillWidth: true
    visible: WeatherService.loaded
    implicitHeight: contentCol.implicitHeight + 24
    radius: Tokens.radiusMS
    color: cardHover.containsMouse ? Colors.surface1 : Colors.surface0
    border.color: cardHover.containsMouse ? Qt.rgba(root.cond.r, root.cond.g, root.cond.b, Tokens.borderHoverAlpha) : Qt.rgba(1, 1, 1, 0.04)
    border.width: 1
    clip: true

    // 仅展开时走表（驱动日出日落弧上的太阳位置）
    SystemClock {
        id: clock

        enabled: root.expanded
        precision: SystemClock.Minutes
    }

    // ── 氛围渐变：状况色从顶部渗入 ──
    Rectangle {
        anchors.fill: parent
        radius: parent.radius

        gradient: Gradient {
            GradientStop { position: 0; color: Qt.rgba(root.cond.r, root.cond.g, root.cond.b, 0.1) }
            GradientStop { position: 0.55; color: "transparent" }
        }

    }

    MouseArea {
        id: cardHover

        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        onClicked: root.expanded = !root.expanded
    }

    // ── UI ──
    ColumnLayout {
        id: contentCol

        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.margins: 12
        spacing: 8

        // ── Hero：大温度 + 状况 + AQI ──
        RowLayout {
            Layout.fillWidth: true
            spacing: 10

            Text {
                text: WeatherService.wmoIcon(WeatherService.weatherCode, WeatherService.isDay)
                color: root.cond
                font.family: Fonts.family
                font.pixelSize: Fonts.h1
            }

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 0

                RowLayout {
                    spacing: 2

                    Text {
                        text: Math.round(WeatherService.temperature) + "°"
                        color: Colors.text
                        font.family: Fonts.family
                        font.pixelSize: Fonts.display3
                        font.weight: Fonts.weightLight
                    }

                    Text {
                        Layout.alignment: Qt.AlignTop
                        Layout.topMargin: 6
                        text: WeatherService.wmoDesc(WeatherService.weatherCode)
                        color: root.cond
                        font.family: Fonts.family
                        font.pixelSize: Fonts.small
                        font.weight: Font.DemiBold
                    }

                }

                Text {
                    Layout.topMargin: -4
                    text: "体感 " + Math.round(WeatherService.apparentTemp) + "° · " + WeatherService.cityName
                    color: Colors.subtext0
                    font.family: Fonts.family
                    font.pixelSize: Fonts.caption
                }

            }

            ColumnLayout {
                Layout.alignment: Qt.AlignTop
                spacing: 6

                // AQI 胶囊徽标
                Rectangle {
                    Layout.alignment: Qt.AlignRight
                    visible: WeatherService.aqiLoaded
                    implicitWidth: aqiRow.implicitWidth + 14
                    implicitHeight: 20
                    radius: Tokens.radiusFull
                    color: Qt.rgba(aqiColor(WeatherService.usAqi).r, aqiColor(WeatherService.usAqi).g, aqiColor(WeatherService.usAqi).b, 0.15)

                    RowLayout {
                        id: aqiRow

                        anchors.centerIn: parent
                        spacing: 4

                        Rectangle {
                            width: 6
                            height: 6
                            radius: 3
                            color: root.aqiColor(WeatherService.usAqi)
                        }

                        Text {
                            text: "AQI " + WeatherService.usAqi + " " + WeatherService.aqiDesc(WeatherService.usAqi)
                            color: root.aqiColor(WeatherService.usAqi)
                            font.family: Fonts.family
                            font.pixelSize: Fonts.xs
                            font.weight: Font.DemiBold
                        }

                    }

                }

                Text {
                    Layout.alignment: Qt.AlignRight
                    text: "󰅀"
                    color: Colors.overlay1
                    font.family: Fonts.family
                    font.pixelSize: Fonts.icon
                    rotation: root.expanded ? 180 : 0

                    Behavior on rotation {
                        NumberAnimation {
                            duration: Tokens.animFast
                            easing.type: Easing.OutCubic
                        }

                    }

                }

            }

        }

        // ── 指标 chips：湿度 / 风速 / 当前降水概率 ──
        RowLayout {
            spacing: 12

            Text {
                text: "󰍝 " + WeatherService.humidity + "%"
                color: Colors.overlay1
                font.family: Fonts.family
                font.pixelSize: Fonts.caption
            }

            Text {
                text: "󰈐 " + Math.round(WeatherService.windSpeed) + "km/h"
                color: Colors.overlay1
                font.family: Fonts.family
                font.pixelSize: Fonts.caption
            }

            Text {
                visible: WeatherService.hourlyData.length > 0
                text: "󰖌 " + (WeatherService.hourlyData.length > 0 ? WeatherService.hourlyData[0].precipProb : 0) + "%"
                color: WeatherService.hourlyData.length > 0 && WeatherService.hourlyData[0].precipProb >= 50 ? Colors.blue : Colors.overlay1
                font.family: Fonts.family
                font.pixelSize: Fonts.caption
            }

        }

        // ── 展开区域 ──
        ColumnLayout {
            Layout.fillWidth: true
            visible: root.expanded
            spacing: 8

            Rectangle {
                Layout.fillWidth: true
                height: 1
                color: Colors.surface1
                opacity: 0.5
            }

            // ── 日出日落弧 ──
            Item {
                Layout.fillWidth: true
                Layout.preferredHeight: 56

                Canvas {
                    id: sunArc

                    // 当前时刻在昼间的进度 0~1（夜间钳到端点）
                    readonly property real frac: {
                        if (!WeatherService.sunrise || !WeatherService.sunset) return 0;
                        let toMin = s => parseInt(s.substring(0, 2)) * 60 + parseInt(s.substring(3, 5));
                        let rise = toMin(WeatherService.sunrise);
                        let set = toMin(WeatherService.sunset);
                        let now = clock.date.getHours() * 60 + clock.date.getMinutes();
                        return Math.max(0, Math.min(1, (now - rise) / (set - rise || 1)));
                    }

                    anchors.fill: parent
                    onFracChanged: requestPaint()
                    onVisibleChanged: if (visible) requestPaint()

                    onPaint: {
                        let ctx = getContext("2d");
                        let w = width, h = height;
                        ctx.clearRect(0, 0, w, h);

                        let pad = 24;
                        let baseY = h - 16;
                        let topY = 6;
                        // 二次贝塞尔弧：P0(左) C(顶) P1(右)
                        let p0x = pad, p1x = w - pad, cx = w / 2;
                        function bx(t) { return (1 - t) * (1 - t) * p0x + 2 * (1 - t) * t * cx + t * t * p1x; }
                        function by(t) { return (1 - t) * (1 - t) * baseY + 2 * (1 - t) * t * topY + t * t * baseY; }

                        // 地平线
                        ctx.beginPath();
                        ctx.moveTo(8, baseY);
                        ctx.lineTo(w - 8, baseY);
                        ctx.strokeStyle = Qt.rgba(Colors.surface2.r, Colors.surface2.g, Colors.surface2.b, 0.6).toString();
                        ctx.lineWidth = 1;
                        ctx.stroke();

                        // 全弧（暗）
                        ctx.beginPath();
                        ctx.moveTo(bx(0), by(0));
                        for (let t = 0.04; t <= 1.001; t += 0.04)
                            ctx.lineTo(bx(t), by(t));
                        ctx.strokeStyle = Colors.surface2.toString();
                        ctx.lineWidth = 1.5;
                        ctx.setLineDash([3, 4]);
                        ctx.stroke();
                        ctx.setLineDash([]);

                        // 已走过的弧（亮）
                        if (frac > 0) {
                            ctx.beginPath();
                            ctx.moveTo(bx(0), by(0));
                            for (let t = 0.02; t <= frac; t += 0.02)
                                ctx.lineTo(bx(t), by(t));
                            ctx.lineTo(bx(frac), by(frac));
                            ctx.strokeStyle = Colors.yellow.toString();
                            ctx.lineWidth = 1.5;
                            ctx.stroke();
                        }

                        // 太阳：光晕 + 实心点（夜间变月亮色）
                        let sx = bx(frac), sy = by(frac);
                        let day = WeatherService.isDay;
                        let c = day ? Colors.yellow : Colors.lavender;
                        ctx.beginPath();
                        ctx.arc(sx, sy, 7, 0, 2 * Math.PI);
                        ctx.fillStyle = Qt.rgba(c.r, c.g, c.b, 0.25).toString();
                        ctx.fill();
                        ctx.beginPath();
                        ctx.arc(sx, sy, 3.5, 0, 2 * Math.PI);
                        ctx.fillStyle = c.toString();
                        ctx.fill();
                    }
                }

                Text {
                    anchors.left: parent.left
                    anchors.bottom: parent.bottom
                    text: "󰖜 " + WeatherService.sunrise
                    color: Colors.subtext0
                    font.family: Fonts.family
                    font.pixelSize: Fonts.xs
                }

                Text {
                    anchors.right: parent.right
                    anchors.bottom: parent.bottom
                    text: "󰖛 " + WeatherService.sunset
                    color: Colors.subtext0
                    font.family: Fonts.family
                    font.pixelSize: Fonts.xs
                }

            }

            Rectangle {
                Layout.fillWidth: true
                height: 1
                color: Colors.surface1
                opacity: 0.5
            }

            // ── 逐时预报（横向滚动）──
            Text {
                text: "未来 24 小时"
                color: Colors.subtext0
                font.family: Fonts.family
                font.pixelSize: Fonts.caption
                font.weight: Font.DemiBold
            }

            Flickable {
                Layout.fillWidth: true
                Layout.preferredHeight: 66
                contentWidth: hourlyRow.implicitWidth
                clip: true
                flickableDirection: Flickable.HorizontalFlick

                Row {
                    id: hourlyRow

                    spacing: 2

                    Repeater {
                        model: WeatherService.hourlyData

                        delegate: Column {
                            required property var modelData

                            width: 42
                            spacing: 2

                            Text {
                                text: modelData.time
                                color: Colors.overlay1
                                font.family: Fonts.family
                                font.pixelSize: Fonts.xs
                                anchors.horizontalCenter: parent.horizontalCenter
                            }

                            Text {
                                text: WeatherService.wmoIcon(modelData.code, modelData.isDay)
                                color: root.condColor(modelData.code, modelData.isDay)
                                font.family: Fonts.family
                                font.pixelSize: Fonts.icon
                                anchors.horizontalCenter: parent.horizontalCenter
                            }

                            Text {
                                text: modelData.temp + "°"
                                color: Colors.text
                                font.family: Fonts.family
                                font.pixelSize: Fonts.caption
                                font.weight: Font.DemiBold
                                anchors.horizontalCenter: parent.horizontalCenter
                            }

                            Text {
                                text: "󰖌" + modelData.precipProb + "%"
                                color: modelData.precipProb >= 50 ? Colors.blue : Colors.overlay0
                                opacity: modelData.precipProb >= 20 ? 1 : 0
                                font.family: Fonts.family
                                font.pixelSize: Fonts.xs
                                anchors.horizontalCenter: parent.horizontalCenter
                            }

                        }

                    }

                }

            }

            Rectangle {
                Layout.fillWidth: true
                height: 1
                color: Colors.surface1
                opacity: 0.5
            }

            // ── 7 天预报 ──
            Text {
                text: "7 天预报"
                color: Colors.subtext0
                font.family: Fonts.family
                font.pixelSize: Fonts.caption
                font.weight: Font.DemiBold
            }

            Repeater {
                model: WeatherService.dailyData

                delegate: RowLayout {
                    required property var modelData

                    Layout.fillWidth: true
                    spacing: 8

                    Text {
                        text: modelData.day
                        color: modelData.day === "今天" ? root.cond : Colors.text
                        font.family: Fonts.family
                        font.pixelSize: Fonts.small
                        font.weight: Font.DemiBold
                        Layout.preferredWidth: 32
                    }

                    Text {
                        text: modelData.date
                        color: Colors.overlay1
                        font.family: Fonts.family
                        font.pixelSize: Fonts.caption
                        Layout.preferredWidth: 38
                    }

                    Text {
                        text: WeatherService.wmoIcon(modelData.code)
                        color: root.condColor(modelData.code, true)
                        font.family: Fonts.family
                        font.pixelSize: Fonts.icon
                    }

                    Text {
                        text: modelData.precipProb >= 20 ? "󰖌" + modelData.precipProb + "%" : ""
                        color: modelData.precipProb >= 50 ? Colors.blue : Colors.overlay0
                        font.family: Fonts.family
                        font.pixelSize: Fonts.xs
                        Layout.preferredWidth: 38
                    }

                    Item {
                        Layout.fillWidth: true
                    }

                    Text {
                        text: modelData.minTemp + "°"
                        color: Colors.blue
                        font.family: Fonts.family
                        font.pixelSize: Fonts.small
                    }

                    // 温度区间条
                    Rectangle {
                        Layout.preferredWidth: 50
                        height: 4
                        radius: 2
                        color: Colors.surface1

                        Rectangle {
                            property real range: {
                                let allMax = -100, allMin = 100;
                                for (let d of WeatherService.dailyData) {
                                    allMax = Math.max(allMax, d.maxTemp);
                                    allMin = Math.min(allMin, d.minTemp);
                                }
                                return allMax - allMin || 1;
                            }
                            property real allMin: {
                                let m = 100;
                                for (let d of WeatherService.dailyData) m = Math.min(m, d.minTemp)
                                return m;
                            }

                            x: parent.width * (modelData.minTemp - allMin) / range
                            width: parent.width * (modelData.maxTemp - modelData.minTemp) / range
                            height: parent.height
                            radius: 2
                            color: Colors.peach
                        }

                    }

                    Text {
                        text: modelData.maxTemp + "°"
                        color: Colors.peach
                        font.family: Fonts.family
                        font.pixelSize: Fonts.small
                    }

                }

            }

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

    Behavior on implicitHeight {
        NumberAnimation {
            duration: 200
            easing.type: Easing.OutCubic
        }

    }

}
