pragma Singleton

import QtQuick
import Quickshell
import Quickshell.Io

// 统一天气服务 — 收口 Open-Meteo 数据获取与 WMO 码映射，消费者（WeatherWidget / WeatherCard）只做渲染。
// curl 必须 --noproxy：本地代理对 api.open-meteo.com TLS 握手超时导致刷新静默失败，
// 数据会一直停留在最后一次成功抓取（曾表现为日期显示昨天）；直连反而稳定。
Singleton {
    id: root

    // ── 配置 ──
    // 修改坐标切换城市（VPN 会导致自动定位不准，故硬编码）
    readonly property string cityName: "北京"
    readonly property real latitude: 39.9
    readonly property real longitude: 116.4

    // ── 数据 ──
    property real temperature: 0
    property real apparentTemp: 0
    property int weatherCode: 0
    property int humidity: 0
    property real windSpeed: 0
    property bool isDay: true
    property string sunrise: "" // 今日 "HH:MM"
    property string sunset: ""
    property bool loaded: false
    property var hourlyData: [] // [{time: "HH:MM", temp, code, precipProb, isDay}]
    property var dailyData: [] // [{day, date: "MM-DD", maxTemp, minTemp, code, precipProb}]
    // 空气质量（独立端点，单独 loaded 标志，失败不影响天气）
    property int usAqi: 0
    property real pm25: 0
    property bool aqiLoaded: false

    function refresh() {
        fetchProc.running = false;
        fetchProc.running = true;
        aqiProc.running = false;
        aqiProc.running = true;
    }

    function _parse(text) {
        try {
            let obj = JSON.parse(text);
            // 当前天气
            root.temperature = obj.current.temperature_2m;
            root.apparentTemp = obj.current.apparent_temperature;
            root.weatherCode = obj.current.weather_code;
            root.humidity = obj.current.relative_humidity_2m;
            root.windSpeed = obj.current.wind_speed_10m;
            root.isDay = obj.current.is_day === 1;
            root.sunrise = obj.daily.sunrise[0].substring(11, 16);
            root.sunset = obj.daily.sunset[0].substring(11, 16);
            // 逐时（昼夜按所属日期的日出/日落区间判定，ISO 串可直接字典序比较）
            let h = [];
            for (let i = 0; i < obj.hourly.time.length; i++) {
                let t = obj.hourly.time[i];
                let di = obj.daily.time.indexOf(t.substring(0, 10));
                let day = di < 0 || (t >= obj.daily.sunrise[di] && t <= obj.daily.sunset[di]);
                h.push({
                    "time": t.substring(11, 16),
                    "temp": Math.round(obj.hourly.temperature_2m[i]),
                    "code": obj.hourly.weather_code[i],
                    "precipProb": obj.hourly.precipitation_probability[i],
                    "isDay": day
                });
            }
            root.hourlyData = h;
            // 逐日
            let d = [];
            let weekdays = ["周日", "周一", "周二", "周三", "周四", "周五", "周六"];
            for (let i = 0; i < obj.daily.time.length; i++) {
                // 按本地时区构造日期，避免 ISO 字符串被解析为 UTC 后星期偏移
                let p = obj.daily.time[i].split("-");
                let date = new Date(parseInt(p[0]), parseInt(p[1]) - 1, parseInt(p[2]));
                d.push({
                    "day": i === 0 ? "今天" : weekdays[date.getDay()],
                    "date": obj.daily.time[i].substring(5),
                    "maxTemp": Math.round(obj.daily.temperature_2m_max[i]),
                    "minTemp": Math.round(obj.daily.temperature_2m_min[i]),
                    "code": obj.daily.weather_code[i],
                    "precipProb": obj.daily.precipitation_probability_max[i]
                });
            }
            root.dailyData = d;
            root.loaded = true;
        } catch (e) {
            // 抓取/解析失败（网络抖动、curl 超时输出为空）→ 短间隔重试，
            // 不等下一个 30 分钟周期，避免数据长时间停留在过期状态
            retryTimer.restart();
        }
    }

    // ── WMO 天气码 → nerd font 图标 / 中文描述 ──
    // day 为 false 时晴/多云用夜间字形
    function wmoIcon(code, day) {
        let isDay = day === undefined ? true : day;
        if (code === 0) return isDay ? "󰖙" : "󰖔"; // 晴 / 晴夜
        if (code <= 3) return isDay ? "󰖕" : "󰼱"; // 多云 / 夜云
        if (code <= 49) return "󰖑"; // 雾
        if (code <= 59) return "󰖗"; // 毛毛雨
        if (code <= 69) return "󰖗"; // 雨
        if (code <= 79) return "󰖘"; // 雪
        if (code <= 84) return "󰖗"; // 阵雨
        if (code <= 86) return "󰖘"; // 阵雪
        if (code <= 99) return "󰖖"; // 雷暴
        return "󰖐";
    }

    // US AQI → 等级描述
    function aqiDesc(v) {
        if (v <= 50) return "优";
        if (v <= 100) return "良";
        if (v <= 150) return "轻度";
        if (v <= 200) return "中度";
        if (v <= 300) return "重度";
        return "严重";
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

    Process {
        id: fetchProc

        command: ["curl", "-s", "--noproxy", "*", "--max-time", "8",
            "https://api.open-meteo.com/v1/forecast?latitude=" + root.latitude
            + "&longitude=" + root.longitude
            + "&current=temperature_2m,apparent_temperature,weather_code,relative_humidity_2m,wind_speed_10m,is_day"
            + "&hourly=temperature_2m,weather_code,precipitation_probability&forecast_hours=24"
            + "&daily=temperature_2m_max,temperature_2m_min,weather_code,precipitation_probability_max,sunrise,sunset&forecast_days=7"
            + "&timezone=Asia/Shanghai"]

        stdout: StdioCollector {
            onStreamFinished: root._parse(text)
        }
    }

    Process {
        id: aqiProc

        command: ["curl", "-s", "--noproxy", "*", "--max-time", "8",
            "https://air-quality-api.open-meteo.com/v1/air-quality?latitude=" + root.latitude
            + "&longitude=" + root.longitude
            + "&current=pm2_5,us_aqi&timezone=Asia/Shanghai"]

        stdout: StdioCollector {
            onStreamFinished: {
                try {
                    let obj = JSON.parse(text);
                    root.pm25 = obj.current.pm2_5;
                    root.usAqi = obj.current.us_aqi;
                    root.aqiLoaded = true;
                } catch (e) {
                    // 静默：AQI 缺失时卡片隐藏徽标即可，天气主数据已有自己的重试
                }
            }
        }
    }

    // 常规刷新：启动即拉一次，之后每 30 分钟
    Timer {
        interval: 1.8e+06
        running: true
        repeat: true
        triggeredOnStart: true
        onTriggered: root.refresh()
    }

    // 失败重试（单次，2 分钟后）
    Timer {
        id: retryTimer

        interval: 120000
        onTriggered: root.refresh()
    }
}
