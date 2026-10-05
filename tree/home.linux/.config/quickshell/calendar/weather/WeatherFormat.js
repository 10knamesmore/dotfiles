.pragma library

// 天气时间固定按北京时区展示；输入时间为服务提供的 Unix 毫秒。
function dateKey(time) {
    return new Date(time + 8 * 3600000).toISOString().slice(0, 10);
}

function timeOfDay(time) {
    return time === null || time === undefined ? "—" : new Date(time + 8 * 3600000).toISOString().slice(11, 16);
}

function timestamp(time, now) {
    if (!time)
        return "尚未更新";
    return (dateKey(time) === dateKey(now) ? "" : dateKey(time).slice(5) + " ") + timeOfDay(time);
}

function dayLabel(date, now) {
    if (date === dateKey(now))
        return "今天";
    if (date === dateKey(now + 86400000))
        return "明天";
    return ["周日", "周一", "周二", "周三", "周四", "周五", "周六"][new Date(date + "T12:00:00+08:00").getUTCDay()];
}

function number(value, decimals) {
    return value === null || value === undefined || !Number.isFinite(value) ? "—" : value.toFixed(decimals === undefined ? 0 : decimals);
}

function temperature(value) {
    return number(value) + (value === null || value === undefined ? "" : "°");
}

function windDirection(degrees) {
    if (degrees === null || degrees === undefined)
        return "风向未知";
    return ["北风", "东北风", "东风", "东南风", "南风", "西南风", "西风", "西北风"][Math.round(degrees / 45) % 8];
}

function metersPerSecond(kilometersPerHour) {
    return kilometersPerHour === null || kilometersPerHour === undefined ? "—" : number(kilometersPerHour / 3.6, 1);
}

function uvLevel(value) {
    if (value === null || value === undefined)
        return "暂无数据";
    if (value < 3)
        return "低";
    if (value < 6)
        return "中等";
    if (value < 8)
        return "高";
    if (value < 11)
        return "很高";
    return "极高";
}

function aqiLevel(value) {
    if (value === null || value === undefined)
        return -1;
    return value <= 50 ? 0 : value <= 100 ? 1 : value <= 150 ? 2 : value <= 200 ? 3 : value <= 300 ? 4 : 5;
}

function upcomingHours(hours, now) {
    const currentHour = Math.floor(now / 3600000) * 3600000;
    return hours.filter(hour => hour.time >= currentHour);
}

function summary(hours) {
    if (!hours.length)
        return "逐时预报已过期，请刷新";
    const next = hours.slice(0, 6);
    const probabilities = next.map(hour => hour.precipProb).filter(value => value !== null);
    const rain = probabilities.length === next.length ? "未来 " + next.length + " 小时最高降雨概率 " + Math.max(...probabilities) + "%" : "未来几小时降雨概率暂缺";
    const last = next[next.length - 1];
    return rain + " · " + timeOfDay(last.time) + " " + temperature(last.temp);
}
