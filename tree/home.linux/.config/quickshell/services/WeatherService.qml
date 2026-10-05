// 天气服务单例 — 抓取 Open-Meteo 当前天气、24 小时与 7 天预报、空气质量，收口 WMO 码与 US AQI 展示映射。
// 对外契约：
//   - weather / airQuality: 最近一次成功抓取的结构化数据；加载或失败时保持旧值（初始 null）
//   - weatherStatus / airQualityStatus: "loading" | "ready" | "error"
//   - weatherUpdatedAt / airQualityUpdatedAt: 最近一次成功抓取完成的 epoch 毫秒
//   - refresh(): 立即刷新两个端点；refreshIfStale(): 面板打开时对未就绪或过期端点按需刷新
//   - wmoIcon(code, day) / wmoDesc(code) / aqiDesc(usAqi): 供 UI 渲染的映射函数
// 副作用：两个 curl 子进程，各自独立失败重试（2 分钟）并每 30 分钟刷新一次。

import QtQuick
import Quickshell
import Quickshell.Io
pragma Singleton

// curl 必须 --noproxy：本地代理对 api.open-meteo.com TLS 握手超时导致刷新静默失败，
// 数据会一直停留在最后一次成功抓取；直连反而稳定。
Singleton {
    // ── 刷新 ──
    // ── 解析 ──
    // ── 抓取 ──
    // ── 展示映射 ──
    // ── 定时 ──

    id: root

    // ── 配置 ──
    // 修改坐标切换城市（VPN 会导致自动定位不准，故硬编码）
    readonly property string cityName: "北京"
    readonly property real latitude: 39.9
    readonly property real longitude: 116.4
    // ── 数据 ──
    // current: {observedAt(epoch ms), temp(°C), apparentTemp(°C), code, humidity(%), windSpeed(km/h),
    //           windDirection(°), windGust(km/h), isDay}
    // hourly[24]: {time(epoch ms), temp, code, precipProb(%), precipitation(mm), visibility(m),
    //              uvIndex, windSpeed(km/h), isDay}
    // daily[7]: {date("YYYY-MM-DD"), minTemp, maxTemp, code, precipProb(%), precipitation(mm),
    //            uvMax, sunrise(epoch ms), sunset(epoch ms)}
    // 缺测数值为 null，UI 按不可用渲染；不得把 null 当作 0 或晴。
    property var weather: null
    // {observedAt(epoch ms), usAqi, pm25(µg/m³)}；缺测数值为 null。
    property var airQuality: null
    property string weatherStatus: "loading"
    property string airQualityStatus: "loading"
    property double weatherUpdatedAt: 0
    property double airQualityUpdatedAt: 0
    readonly property bool refreshing: weatherStatus === "loading" || airQualityStatus === "loading"
    readonly property string _forecastUrl: "https://api.open-meteo.com/v1/forecast" + "?latitude=" + root.latitude + "&longitude=" + root.longitude + "&current=temperature_2m,apparent_temperature,weather_code,relative_humidity_2m,wind_speed_10m,wind_direction_10m,wind_gusts_10m,is_day" + "&hourly=temperature_2m,weather_code,precipitation_probability,precipitation,visibility,uv_index,wind_speed_10m,is_day&forecast_hours=24" + "&daily=temperature_2m_max,temperature_2m_min,weather_code,precipitation_probability_max,precipitation_sum,uv_index_max,sunrise,sunset&forecast_days=7" + "&timezone=Asia/Shanghai"
    readonly property string _airQualityUrl: "https://air-quality-api.open-meteo.com/v1/air-quality" + "?latitude=" + root.latitude + "&longitude=" + root.longitude + "&current=pm2_5,us_aqi&timezone=Asia/Shanghai"

    // 立即刷新两个端点；已在途的抓取不打断，避免手动刷新或定时刷新把本次结果丢弃。
    function refresh() {
        _startWeatherFetch();
        _startAirQualityFetch();
    }

    // 面板打开时调用：每个端点独立判断，从未成功、上次失败或距上次成功超过 30 分钟才重新抓取。
    function refreshIfStale() {
        const staleAfterMs = 30 * 60 * 1000;
        const now = Date.now();
        if (weatherStatus !== "ready" || now - weatherUpdatedAt > staleAfterMs)
            _startWeatherFetch();

        if (airQualityStatus !== "ready" || now - airQualityUpdatedAt > staleAfterMs)
            _startAirQualityFetch();

    }

    function _startWeatherFetch() {
        if (weatherProc.running)
            return ;

        weatherStatus = "loading";
        console.info("[weather] refresh started");
        weatherProc.running = true;
    }

    function _startAirQualityFetch() {
        if (airQualityProc.running)
            return ;

        airQualityStatus = "loading";
        console.info("[air-quality] refresh started");
        airQualityProc.running = true;
    }

    // Open-Meteo 配 timezone=Asia/Shanghai 后返回不带偏移的本地时间 "YYYY-MM-DDTHH:MM"；
    // 显式按 +08:00 解析成 epoch 毫秒，使时间戳不随主机时区漂移。
    function _epochMs(localIso) {
        const ms = Date.parse(localIso + ":00+08:00");
        return isNaN(ms) ? null : ms;
    }

    // 必需字段必须存在；provider 用 null 表示缺测，不会整个键消失。
    function _hasFields(section, fields) {
        if (!section)
            return false;

        for (const field of fields) {
            if (!(field in section))
                return false;

        }
        return true;
    }

    // 请求的每个数组字段必须存在、非空且与 time 等长，否则视为不完整响应。
    function _hasCompleteSeries(section, fields) {
        if (!section || !Array.isArray(section.time) || section.time.length === 0)
            return false;

        for (const field of fields) {
            const series = section[field];
            if (!Array.isArray(series) || series.length !== section.time.length)
                return false;

        }
        return true;
    }

    // 解析预报响应；provider 报错或结构不完整时抛出，由调用方转入 error 状态。
    function _parseWeather(text) {
        const data = JSON.parse(text);
        const currentFields = ["time", "temperature_2m", "apparent_temperature", "weather_code", "relative_humidity_2m", "wind_speed_10m", "wind_direction_10m", "wind_gusts_10m", "is_day"];
        const hourlyFields = ["time", "temperature_2m", "weather_code", "precipitation_probability", "precipitation", "visibility", "uv_index", "wind_speed_10m", "is_day"];
        const dailyFields = ["time", "temperature_2m_max", "temperature_2m_min", "weather_code", "precipitation_probability_max", "precipitation_sum", "uv_index_max", "sunrise", "sunset"];
        if (data.error)
            throw new Error("provider error: " + (data.reason || "unknown"));

        if (!_hasFields(data.current, currentFields) || typeof data.current.time !== "string")
            throw new Error("incomplete current section");

        if (!_hasCompleteSeries(data.hourly, hourlyFields) || !_hasCompleteSeries(data.daily, dailyFields))
            throw new Error("incomplete forecast series");

        const hourly = [];
        for (let i = 0; i < data.hourly.time.length; i++) {
            hourly.push({
                "time": _epochMs(data.hourly.time[i]),
                "temp": data.hourly.temperature_2m[i],
                "code": data.hourly.weather_code[i],
                "precipProb": data.hourly.precipitation_probability[i],
                "precipitation": data.hourly.precipitation[i],
                "visibility": data.hourly.visibility[i],
                "uvIndex": data.hourly.uv_index[i],
                "windSpeed": data.hourly.wind_speed_10m[i],
                "isDay": data.hourly.is_day[i] === 1
            });
        }
        const daily = [];
        for (let i = 0; i < data.daily.time.length; i++) {
            daily.push({
                "date": data.daily.time[i],
                "minTemp": data.daily.temperature_2m_min[i],
                "maxTemp": data.daily.temperature_2m_max[i],
                "code": data.daily.weather_code[i],
                "precipProb": data.daily.precipitation_probability_max[i],
                "precipitation": data.daily.precipitation_sum[i],
                "uvMax": data.daily.uv_index_max[i],
                "sunrise": _epochMs(data.daily.sunrise[i]),
                "sunset": _epochMs(data.daily.sunset[i])
            });
        }
        return {
            "current": {
                "observedAt": _epochMs(data.current.time),
                "temp": data.current.temperature_2m,
                "apparentTemp": data.current.apparent_temperature,
                "code": data.current.weather_code,
                "humidity": data.current.relative_humidity_2m,
                "windSpeed": data.current.wind_speed_10m,
                "windDirection": data.current.wind_direction_10m,
                "windGust": data.current.wind_gusts_10m,
                "isDay": data.current.is_day === 1
            },
            "hourly": hourly,
            "daily": daily
        };
    }

    // 解析空气质量响应；provider 报错或结构不完整时抛出。
    function _parseAirQuality(text) {
        const data = JSON.parse(text);
        if (data.error)
            throw new Error("provider error: " + (data.reason || "unknown"));

        if (!_hasFields(data.current, ["time", "us_aqi", "pm2_5"]) || typeof data.current.time !== "string")
            throw new Error("incomplete air quality payload");

        return {
            "observedAt": _epochMs(data.current.time),
            "usAqi": data.current.us_aqi,
            "pm25": data.current.pm2_5
        };
    }

    // 唯一的提交点：非零退出（网络失败或 curl 的 HTTP 错误）一律计失败，不会被空正文当成成功。
    function _handleWeatherExit(exitCode, exitStatus) {
        if (exitCode !== 0) {
            root.weatherStatus = "error";
            console.warn("[weather] refresh failed; exit=" + exitCode + " status=" + exitStatus, weatherStderr.text.trim());
            weatherRetryTimer.restart();
            return ;
        }
        try {
            const parsed = _parseWeather(weatherStdout.text);
            root.weather = parsed;
            root.weatherUpdatedAt = Date.now();
            root.weatherStatus = "ready";
            weatherRetryTimer.stop();
            console.info("[weather] refresh succeeded; observedAt=" + parsed.current.observedAt);
        } catch (error) {
            root.weatherStatus = "error";
            console.warn("[weather] response rejected:", error);
            weatherRetryTimer.restart();
        }
    }

    function _handleAirQualityExit(exitCode, exitStatus) {
        if (exitCode !== 0) {
            root.airQualityStatus = "error";
            console.warn("[air-quality] refresh failed; exit=" + exitCode + " status=" + exitStatus, airQualityStderr.text.trim());
            airQualityRetryTimer.restart();
            return ;
        }
        try {
            const parsed = _parseAirQuality(airQualityStdout.text);
            root.airQuality = parsed;
            root.airQualityUpdatedAt = Date.now();
            root.airQualityStatus = "ready";
            airQualityRetryTimer.stop();
            console.info("[air-quality] refresh succeeded; observedAt=" + parsed.observedAt);
        } catch (error) {
            root.airQualityStatus = "error";
            console.warn("[air-quality] response rejected:", error);
            airQualityRetryTimer.restart();
        }
    }

    // WMO 天气码 → nerd font 图标；day 为 false 时晴/多云用夜间字形，null/undefined 按白天。
    function wmoIcon(code, day) {
        if (code === null || code === undefined)
            return "󰖐";

        const isDay = day !== false;
        if (code === 0)
            return isDay ? "󰖙" : "󰖔";
 // 晴
        if (code >= 1 && code <= 3)
            return isDay ? "󰖕" : "󰼱";
 // 多云
        if (code === 45 || code === 48)
            return "󰖑";
 // 雾
        if (code >= 51 && code <= 57)
            return "󰖗";
 // 毛毛雨 / 冻毛毛雨
        if (code >= 61 && code <= 67)
            return "󰖗";
 // 雨 / 冻雨
        if (code >= 71 && code <= 77)
            return "󰖘";
 // 雪 / 雪粒
        if (code >= 80 && code <= 82)
            return "󰖗";
 // 阵雨
        if (code === 85 || code === 86)
            return "󰖘";
 // 阵雪
        if (code >= 95 && code <= 99)
            return "󰖖";
 // 雷暴 / 雷暴伴冰雹
        return "󰖐"; // 未知
    }

    // WMO 天气码 → 中文描述；未覆盖的码与 null 返回 "未知"。
    function wmoDesc(code) {
        if (code === null || code === undefined)
            return "未知";

        if (code === 0)
            return "晴";

        if (code === 1)
            return "大部晴";

        if (code === 2)
            return "多云";

        if (code === 3)
            return "阴";

        if (code === 45)
            return "雾";

        if (code === 48)
            return "雾凇";

        if (code === 51)
            return "小毛毛雨";

        if (code === 53)
            return "毛毛雨";

        if (code === 55)
            return "大毛毛雨";

        if (code === 56)
            return "轻度冻毛毛雨";

        if (code === 57)
            return "强冻毛毛雨";

        if (code === 61)
            return "小雨";

        if (code === 63)
            return "中雨";

        if (code === 65)
            return "大雨";

        if (code === 66)
            return "轻度冻雨";

        if (code === 67)
            return "强冻雨";

        if (code === 71)
            return "小雪";

        if (code === 73)
            return "中雪";

        if (code === 75)
            return "大雪";

        if (code === 77)
            return "雪粒";

        if (code === 80)
            return "小阵雨";

        if (code === 81)
            return "中阵雨";

        if (code === 82)
            return "强阵雨";

        if (code === 85)
            return "小阵雪";

        if (code === 86)
            return "大阵雪";

        if (code === 95)
            return "雷暴";

        if (code === 96)
            return "雷暴伴小冰雹";

        if (code === 99)
            return "雷暴伴大冰雹";

        return "未知";
    }

    // US AQI → 中文等级；null 或非数值返回 "未知"。
    function aqiDesc(usAqi) {
        if (usAqi === null || usAqi === undefined || isNaN(usAqi))
            return "未知";

        if (usAqi <= 50)
            return "优";

        if (usAqi <= 100)
            return "中等";

        if (usAqi <= 150)
            return "敏感人群不健康";

        if (usAqi <= 200)
            return "不健康";

        if (usAqi <= 300)
            return "非常不健康";

        return "危险";
    }

    // Quickshell 在 emit exited 之前先结束 stdout/stderr 流，因此 _handle...Exit 里读到的
    // text 已是本次运行的完整输出；解析与状态提交也只在那里发生一次。
    Process {
        id: weatherProc

        command: ["curl", "-fsS", "--noproxy", "*", "--max-time", "8", root._forecastUrl]
        onExited: (exitCode, exitStatus) => {
            return root._handleWeatherExit(exitCode, exitStatus);
        }

        stdout: StdioCollector {
            id: weatherStdout
        }

        stderr: StdioCollector {
            id: weatherStderr
        }

    }

    Process {
        id: airQualityProc

        command: ["curl", "-fsS", "--noproxy", "*", "--max-time", "8", root._airQualityUrl]
        onExited: (exitCode, exitStatus) => {
            return root._handleAirQualityExit(exitCode, exitStatus);
        }

        stdout: StdioCollector {
            id: airQualityStdout
        }

        stderr: StdioCollector {
            id: airQualityStderr
        }

    }

    // 启动即抓取，之后每 30 分钟一次；失败由各自的 2 分钟重试补上。
    Timer {
        interval: 30 * 60 * 1000
        running: true
        repeat: true
        triggeredOnStart: true
        onTriggered: root.refresh()
    }

    // 天气抓取失败后 2 分钟重试；成功时停止。
    Timer {
        id: weatherRetryTimer

        interval: 2 * 60 * 1000
        onTriggered: root._startWeatherFetch()
    }

    // 空气质量抓取失败后 2 分钟重试；成功时停止。
    Timer {
        id: airQualityRetryTimer

        interval: 2 * 60 * 1000
        onTriggered: root._startAirQualityFetch()
    }

}
