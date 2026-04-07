import "../theme"
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Wayland

// 桌面浮动天气卡片 — 当前天气 + Canvas 温度折线图
PanelWindow {
    id: root

    required property var modelData
    screen: modelData

    // ── 布局配置（改这里调整位置和大小）──
    property int widgetMarginRight: 20  // 右边距
    property int widgetY: 60            // 上边距

    // ── 配置 ──
    property string cityName: "北京"
    property real latitude: 39.9
    property real longitude: 116.4

    // ── 数据 ──
    property real temperature: 0
    property int weatherCode: 0
    property int humidity: 0
    property real windSpeed: 0
    property bool loaded: false
    property var hourlyData: [] // [{time, temp, code}]
    property string _buf: ""

    function fetchWeather() {
        root._buf = "";
        weatherProc.running = true;
    }

    function parseData() {
        try {
            let obj = JSON.parse(_buf);
            root.temperature = obj.current.temperature_2m;
            root.weatherCode = obj.current.weather_code;
            root.humidity = obj.current.relative_humidity_2m;
            root.windSpeed = obj.current.wind_speed_10m;
            let h = [];
            for (let i = 0; i < obj.hourly.time.length; i++) {
                let t = obj.hourly.time[i];
                h.push({
                    "time": t.substring(11, 16),
                    "temp": Math.round(obj.hourly.temperature_2m[i]),
                    "code": obj.hourly.weather_code[i]
                });
            }
            root.hourlyData = h;
            root.loaded = true;
            sparkline.requestPaint();
        } catch (e) {}
    }

    function wmoIcon(code) {
        if (code === 0) return "\udb80\ude19"; // 晴
        if (code <= 3) return "\udb80\ude10"; // 多云
        if (code <= 49) return "\udb80\ude11"; // 雾
        if (code <= 59) return "\udb80\ude17"; // 毛毛雨
        if (code <= 69) return "\udb80\ude17"; // 雨
        if (code <= 79) return "\udb80\ude18"; // 雪
        if (code <= 84) return "\udb80\ude17"; // 阵雨
        if (code <= 86) return "\udb80\ude18"; // 阵雪
        if (code <= 99) return "\udb80\ude16"; // 雷暴
        return "\udb80\ude10";
    }

    function wmoDesc(code) {
        if (code === 0) return "晴";
        if (code === 1) return "大部晴";
        if (code === 2) return "多云";
        if (code === 3) return "阴";
        if (code <= 49) return "雾";
        if (code <= 55) return "毛毛雨";
        if (code <= 59) return "冻雨";
        if (code <= 65) return "雨";
        if (code <= 69) return "冻雨";
        if (code <= 75) return "雪";
        if (code <= 79) return "冰粒";
        if (code <= 82) return "阵雨";
        if (code <= 86) return "阵雪";
        if (code === 95) return "雷暴";
        if (code <= 99) return "雷暴+冰雹";
        return "未知";
    }

    aboveWindows: false
    anchors.top: true
    anchors.right: true
    implicitWidth: 300
    implicitHeight: 200
    margins.top: widgetY
    margins.right: widgetMarginRight
    visible: PanelState.weatherWidgetVisible && root.loaded
    focusable: false
    exclusionMode: ExclusionMode.Ignore
    color: "transparent"

    Component.onCompleted: fetchWeather()

    Timer {
        interval: 1.8e+06 // 30 分钟
        running: true
        repeat: true
        onTriggered: fetchWeather()
    }

    Process {
        id: weatherProc
        command: ["curl", "-s", "--max-time", "8",
            "https://api.open-meteo.com/v1/forecast?latitude=" + root.latitude
            + "&longitude=" + root.longitude
            + "&current=temperature_2m,weather_code,relative_humidity_2m,wind_speed_10m"
            + "&hourly=temperature_2m,weather_code&forecast_hours=24"
            + "&timezone=Asia/Shanghai"]
        onExited: root.parseData()
        stdout: SplitParser {
            onRead: data => { root._buf += data; }
        }
    }

    // ── 主体 ──
    Rectangle {
        id: card
        anchors.centerIn: parent
        width: 280
        height: 180
        radius: Tokens.radiusXL
        color: Qt.rgba(Colors.base.r, Colors.base.g, Colors.base.b, Tokens.panelAlpha)
        border.color: Qt.rgba(Tokens.borderBase.r, Tokens.borderBase.g, Tokens.borderBase.b, Tokens.borderAlpha)
        border.width: 1

        SoftShadow {
            anchors.fill: parent
            radius: parent.radius
        }
        InnerGlow {}

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 14
            spacing: 6

            // ── 当前天气 ──
            RowLayout {
                Layout.fillWidth: true
                spacing: 10

                Text {
                    text: wmoIcon(root.weatherCode)
                    color: Colors.yellow
                    font.family: Fonts.family
                    font.pixelSize: Fonts.h1
                }

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 2

                    RowLayout {
                        spacing: 6
                        Text {
                            text: Math.round(root.temperature) + "°C"
                            color: Colors.text
                            font.family: Fonts.family
                            font.pixelSize: Fonts.h2
                            font.weight: Font.Bold
                        }
                        Text {
                            text: wmoDesc(root.weatherCode)
                            color: Colors.subtext0
                            font.family: Fonts.family
                            font.pixelSize: Fonts.body
                        }
                    }

                    RowLayout {
                        spacing: 10
                        Text {
                            text: "\udb81\ud35d " + root.humidity + "%"
                            color: Colors.overlay1
                            font.family: Fonts.family
                            font.pixelSize: Fonts.caption
                        }
                        Text {
                            text: "\udb80\ude10 " + Math.round(root.windSpeed) + "km/h"
                            color: Colors.overlay1
                            font.family: Fonts.family
                            font.pixelSize: Fonts.caption
                        }
                        Text {
                            text: root.cityName
                            color: Colors.overlay0
                            font.family: Fonts.family
                            font.pixelSize: Fonts.caption
                        }
                    }
                }
            }

            // ── 温度折线图 ──
            Canvas {
                id: sparkline
                Layout.fillWidth: true
                Layout.fillHeight: true
                Layout.minimumHeight: 50

                onPaint: {
                    let ctx = getContext("2d");
                    let w = width;
                    let h = height;
                    ctx.clearRect(0, 0, w, h);

                    if (root.hourlyData.length < 2) return;

                    let data = root.hourlyData;
                    let minT = 100, maxT = -100;
                    for (let d of data) {
                        minT = Math.min(minT, d.temp);
                        maxT = Math.max(maxT, d.temp);
                    }
                    let range = maxT - minT || 1;
                    let padY = 12;

                    function tx(i) { return i / (data.length - 1) * (w - 10) + 5; }
                    function ty(temp) { return padY + (1 - (temp - minT) / range) * (h - padY * 2 - 10); }

                    // 面积填充
                    ctx.beginPath();
                    ctx.moveTo(tx(0), ty(data[0].temp));
                    for (let i = 1; i < data.length; i++)
                        ctx.lineTo(tx(i), ty(data[i].temp));
                    ctx.lineTo(tx(data.length - 1), h);
                    ctx.lineTo(tx(0), h);
                    ctx.closePath();
                    ctx.fillStyle = Qt.rgba(Colors.blue.r, Colors.blue.g, Colors.blue.b, 0.12).toString();
                    ctx.fill();

                    // 折线
                    ctx.beginPath();
                    ctx.moveTo(tx(0), ty(data[0].temp));
                    for (let i = 1; i < data.length; i++)
                        ctx.lineTo(tx(i), ty(data[i].temp));
                    ctx.strokeStyle = Colors.blue.toString();
                    ctx.lineWidth = 1.5;
                    ctx.lineJoin = "round";
                    ctx.stroke();

                    // 数据点 + 标签
                    ctx.font = "9px 'Hack Nerd Font'";
                    ctx.textAlign = "center";
                    for (let i = 0; i < data.length; i += 4) {
                        let x = tx(i), y = ty(data[i].temp);
                        // 圆点
                        ctx.beginPath();
                        ctx.arc(x, y, 2.5, 0, 2 * Math.PI);
                        ctx.fillStyle = (i === 0) ? Colors.yellow.toString() : Colors.blue.toString();
                        ctx.fill();
                        // 时间标签
                        ctx.fillStyle = Colors.overlay0.toString();
                        ctx.fillText(data[i].time, x, h - 2);
                        // 温度标签
                        ctx.fillStyle = Colors.subtext0.toString();
                        ctx.fillText(data[i].temp + "°", x, y - 6);
                    }
                }
            }
        }
    }

}
