import QtQuick
import QtQuick.Layouts
import Quickshell.Io
import "../theme"

// 天气卡片 — Open-Meteo API，可展开显示逐时 + 3 天预报
Rectangle {
    id: root

    // ── 配置 ──
    // 修改坐标切换城市（VPN 会导致自动定位不准）
    property string cityName: "北京"
    property real latitude: 39.90
    property real longitude: 116.40

    // ── 数据 ──
    property real temperature: 0
    property int weatherCode: 0
    property int humidity: 0
    property real windSpeed: 0
    property bool loaded: false
    property bool expanded: false

    property var hourlyData: []   // [{time, temp, code}]
    property var dailyData: []    // [{date, maxTemp, minTemp, code}]

    Layout.fillWidth: true
    visible: loaded
    implicitHeight: contentCol.implicitHeight + 20
    radius: 10
    color: cardHover.containsMouse ? Colors.surface1 : Colors.surface0
    border.color: cardHover.containsMouse ? Qt.rgba(Colors.blue.r, Colors.blue.g, Colors.blue.b, 0.2) : Qt.rgba(1, 1, 1, 0.04)
    border.width: 1
    clip: true

    Behavior on color {
        ColorAnimation {
            duration: 200
            easing.type: Easing.OutCubic
        }
    }
    Behavior on border.color {
        ColorAnimation {
            duration: 200
            easing.type: Easing.OutCubic
        }
    }
    Behavior on implicitHeight {
        NumberAnimation {
            duration: 200
            easing.type: Easing.OutCubic
        }
    }

    MouseArea {
        id: cardHover
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        onClicked: root.expanded = !root.expanded
    }

    Component.onCompleted: fetchWeather()

    Timer {
        interval: 1800000  // 30 分钟
        running: true
        repeat: true
        onTriggered: fetchWeather()
    }

    // ── 数据获取 ──
    property string _buf: ""

    function fetchWeather() {
        root._buf = "";
        weatherProc.running = true;
    }

    Process {
        id: weatherProc
        command: ["curl", "-s", "--max-time", "8", "https://api.open-meteo.com/v1/forecast?latitude=" + root.latitude + "&longitude=" + root.longitude + "&current=temperature_2m,weather_code,relative_humidity_2m,wind_speed_10m" + "&hourly=temperature_2m,weather_code&forecast_hours=24" + "&daily=temperature_2m_max,temperature_2m_min,weather_code&forecast_days=7" + "&timezone=Asia/Shanghai"]
        stdout: SplitParser {
            onRead: data => root._buf += data
        }
        onExited: root.parseData()
    }

    function parseData() {
        try {
            let obj = JSON.parse(_buf);

            // 当前天气
            root.temperature = obj.current.temperature_2m;
            root.weatherCode = obj.current.weather_code;
            root.humidity = obj.current.relative_humidity_2m;
            root.windSpeed = obj.current.wind_speed_10m;

            // 逐时
            let h = [];
            for (let i = 0; i < obj.hourly.time.length; i++) {
                let t = obj.hourly.time[i];
                h.push({
                    time: t.substring(11, 16)  // "HH:MM"
                    ,
                    temp: Math.round(obj.hourly.temperature_2m[i]),
                    code: obj.hourly.weather_code[i]
                });
            }
            root.hourlyData = h;

            // 3 天
            let d = [];
            let weekdays = ["周日", "周一", "周二", "周三", "周四", "周五", "周六"];
            for (let i = 0; i < obj.daily.time.length; i++) {
                let date = new Date(obj.daily.time[i]);
                d.push({
                    day: i === 0 ? "今天" : weekdays[date.getDay()],
                    date: obj.daily.time[i].substring(5)  // "MM-DD"
                    ,
                    maxTemp: Math.round(obj.daily.temperature_2m_max[i]),
                    minTemp: Math.round(obj.daily.temperature_2m_min[i]),
                    code: obj.daily.weather_code[i]
                });
            }
            root.dailyData = d;
            root.loaded = true;
        } catch (e) {}
    }

    // ── WMO 天气码 → 图标/描述 ──
    function wmoIcon(code) {
        if (code === 0)
            return "󰖙";           // 晴
        if (code <= 3)
            return "󰖐";            // 多云
        if (code <= 49)
            return "󰖑";           // 雾
        if (code <= 59)
            return "󰖗";           // 毛毛雨
        if (code <= 69)
            return "󰖗";           // 雨
        if (code <= 79)
            return "󰖘";           // 雪
        if (code <= 84)
            return "󰖗";           // 阵雨
        if (code <= 86)
            return "󰖘";           // 阵雪
        if (code <= 99)
            return "󰖖";           // 雷暴
        return "󰖐";
    }

    function wmoDesc(code) {
        if (code === 0)
            return "晴";
        if (code === 1)
            return "大部晴";
        if (code === 2)
            return "多云";
        if (code === 3)
            return "阴";
        if (code <= 49)
            return "雾";
        if (code <= 55)
            return "毛毛雨";
        if (code <= 59)
            return "冻雨";
        if (code <= 65)
            return "雨";
        if (code <= 69)
            return "冻雨";
        if (code <= 75)
            return "雪";
        if (code <= 79)
            return "冰粒";
        if (code <= 82)
            return "阵雨";
        if (code <= 86)
            return "阵雪";
        if (code === 95)
            return "雷暴";
        if (code <= 99)
            return "雷暴+冰雹";
        return "未知";
    }

    // ── UI ──
    ColumnLayout {
        id: contentCol
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.margins: 10
        spacing: 8

        // ── 当前天气（始终显示）──
        RowLayout {
            Layout.fillWidth: true
            spacing: 10

            Text {
                text: wmoIcon(root.weatherCode)
                color: Colors.yellow
                font.family: "Hack Nerd Font"
                font.pixelSize: 24
            }
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 2

                RowLayout {
                    spacing: 8
                    Text {
                        text: Math.round(root.temperature) + "°C"
                        color: Colors.text
                        font.family: "Hack Nerd Font"
                        font.pixelSize: 13
                        font.weight: Font.Bold
                    }
                    Text {
                        text: wmoDesc(root.weatherCode)
                        color: Colors.subtext0
                        font.family: "Hack Nerd Font"
                        font.pixelSize: 11
                    }
                    Item {
                        Layout.fillWidth: true
                    }
                    Text {
                        text: root.expanded ? "󰅃" : "󰅀"
                        color: Colors.overlay1
                        font.family: "Hack Nerd Font"
                        font.pixelSize: 14
                    }
                }
                RowLayout {
                    spacing: 12
                    Text {
                        text: "󰍝 " + root.humidity + "%"
                        color: Colors.overlay1
                        font.family: "Hack Nerd Font"
                        font.pixelSize: 10
                    }
                    Text {
                        text: "󰈐 " + Math.round(root.windSpeed) + "km/h"
                        color: Colors.overlay1
                        font.family: "Hack Nerd Font"
                        font.pixelSize: 10
                    }
                    Text {
                        text: "󰆍 " + root.cityName
                        color: Colors.overlay1
                        font.family: "Hack Nerd Font"
                        font.pixelSize: 10
                    }
                }
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

            // 逐时预报（横向滚动）
            Text {
                text: "未来 24 小时"
                color: Colors.subtext0
                font.family: "Hack Nerd Font"
                font.pixelSize: 10
                font.weight: Font.DemiBold
            }
            Flickable {
                Layout.fillWidth: true
                Layout.preferredHeight: 52
                contentWidth: hourlyRow.implicitWidth
                clip: true
                flickableDirection: Flickable.HorizontalFlick

                Row {
                    id: hourlyRow
                    spacing: 2

                    Repeater {
                        model: root.hourlyData
                        delegate: Column {
                            required property var modelData
                            width: 42
                            spacing: 2
                            Text {
                                text: modelData.time
                                color: Colors.overlay1
                                font.family: "Hack Nerd Font"
                                font.pixelSize: 9
                                anchors.horizontalCenter: parent.horizontalCenter
                            }
                            Text {
                                text: wmoIcon(modelData.code)
                                color: Colors.yellow
                                font.family: "Hack Nerd Font"
                                font.pixelSize: 14
                                anchors.horizontalCenter: parent.horizontalCenter
                            }
                            Text {
                                text: modelData.temp + "°"
                                color: Colors.text
                                font.family: "Hack Nerd Font"
                                font.pixelSize: 10
                                font.weight: Font.DemiBold
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

            // 3 天预报
            Text {
                text: "7 天预报"
                color: Colors.subtext0
                font.family: "Hack Nerd Font"
                font.pixelSize: 10
                font.weight: Font.DemiBold
            }
            Repeater {
                model: root.dailyData
                delegate: RowLayout {
                    required property var modelData
                    Layout.fillWidth: true
                    spacing: 8

                    Text {
                        text: modelData.day
                        color: Colors.text
                        font.family: "Hack Nerd Font"
                        font.pixelSize: 11
                        font.weight: Font.DemiBold
                        Layout.preferredWidth: 32
                    }
                    Text {
                        text: modelData.date
                        color: Colors.overlay1
                        font.family: "Hack Nerd Font"
                        font.pixelSize: 10
                        Layout.preferredWidth: 40
                    }
                    Text {
                        text: wmoIcon(modelData.code)
                        color: Colors.yellow
                        font.family: "Hack Nerd Font"
                        font.pixelSize: 14
                    }
                    Text {
                        text: wmoDesc(modelData.code)
                        color: Colors.subtext0
                        font.family: "Hack Nerd Font"
                        font.pixelSize: 10
                        Layout.preferredWidth: 50
                    }
                    Item {
                        Layout.fillWidth: true
                    }
                    Text {
                        text: modelData.minTemp + "°"
                        color: Colors.blue
                        font.family: "Hack Nerd Font"
                        font.pixelSize: 11
                    }
                    // 温度条
                    Rectangle {
                        Layout.preferredWidth: 50
                        height: 4
                        radius: 2
                        color: Colors.surface1
                        Rectangle {
                            property real range: {
                                let allMax = -100, allMin = 100;
                                for (let d of root.dailyData) {
                                    allMax = Math.max(allMax, d.maxTemp);
                                    allMin = Math.min(allMin, d.minTemp);
                                }
                                return allMax - allMin || 1;
                            }
                            property real allMin: {
                                let m = 100;
                                for (let d of root.dailyData)
                                    m = Math.min(m, d.minTemp);
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
                        font.family: "Hack Nerd Font"
                        font.pixelSize: 11
                    }
                }
            }
        }
    }
}
