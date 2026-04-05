// 农历计算 — 1900-2100 年查表法
// 每个元素编码：高4位=闰月月份(0无), 接下来12位=各月大小(1=30天,0=29天), 低4位=闰月大小

var lunarInfo = [
    0x04bd8,0x04ae0,0x0a570,0x054d5,0x0d260,0x0d950,0x16554,0x056a0,0x09ad0,0x055d2,
    0x04ae0,0x0a5b6,0x0a4d0,0x0d250,0x1d255,0x0b540,0x0d6a0,0x0ada2,0x095b0,0x14977,
    0x04970,0x0a4b0,0x0b4b5,0x06a50,0x06d40,0x1ab54,0x02b60,0x09570,0x052f2,0x04970,
    0x06566,0x0d4a0,0x0ea50,0x06e95,0x05ad0,0x02b60,0x186e3,0x092e0,0x1c8d7,0x0c950,
    0x0d4a0,0x1d8a6,0x0b550,0x056a0,0x1a5b4,0x025d0,0x092d0,0x0d2b2,0x0a950,0x0b557,
    0x06ca0,0x0b550,0x15355,0x04da0,0x0a5b0,0x14573,0x052b0,0x0a9a8,0x0e950,0x06aa0,
    0x0aea6,0x0ab50,0x04b60,0x0aae4,0x0a570,0x05260,0x0f263,0x0d950,0x05b57,0x056a0,
    0x096d0,0x04dd5,0x04ad0,0x0a4d0,0x0d4d4,0x0d250,0x0d558,0x0b540,0x0b6a0,0x195a6,
    0x095b0,0x049b0,0x0a974,0x0a4b0,0x0b27a,0x06a50,0x06d40,0x0af46,0x0ab60,0x09570,
    0x04af5,0x04970,0x064b0,0x074a3,0x0ea50,0x06b58,0x05ac0,0x0ab60,0x096d5,0x092e0,
    0x0c960,0x0d954,0x0d4a0,0x0da50,0x07552,0x056a0,0x0abb7,0x025d0,0x092d0,0x0cab5,
    0x0a950,0x0b4a0,0x0baa4,0x0ad50,0x055d9,0x04ba0,0x0a5b0,0x15176,0x052b0,0x0a930,
    0x07954,0x06aa0,0x0ad50,0x05b52,0x04b60,0x0a6e6,0x0a4e0,0x0d260,0x0ea65,0x0d530,
    0x05aa0,0x076a3,0x096d0,0x04afb,0x04ad0,0x0a4d0,0x1d0b6,0x0d250,0x0d520,0x0dd45,
    0x0b5a0,0x056d0,0x055b2,0x049b0,0x0a577,0x0a4b0,0x0aa50,0x1b255,0x06d20,0x0ada0,
    0x14b63,0x09370,0x049f8,0x04970,0x064b0,0x168a6,0x0ea50,0x06b20,0x1a6c4,0x0aae0,
    0x092e0,0x0d2e3,0x0c960,0x0d557,0x0d4a0,0x0da50,0x05d55,0x056a0,0x0a6d0,0x055d4,
    0x052d0,0x0a9b8,0x0a950,0x0b4a0,0x0b6a6,0x0ad50,0x055a0,0x0aba4,0x0a5b0,0x052b0,
    0x0b273,0x06930,0x07337,0x06aa0,0x0ad50,0x14b55,0x04b60,0x0a570,0x054e4,0x0d160,
    0x0e968,0x0d520,0x0daa0,0x16aa6,0x056d0,0x04ae0,0x0a9d4,0x0a4d0,0x0d150,0x0f252,
    0x0d520
];

var tianGan = ["甲","乙","丙","丁","戊","己","庚","辛","壬","癸"];
var diZhi = ["子","丑","寅","卯","辰","巳","午","未","申","酉","戌","亥"];
var shengXiao = ["鼠","牛","虎","兔","龙","蛇","马","羊","猴","鸡","狗","猪"];
var lunarMonthName = ["正","二","三","四","五","六","七","八","九","十","冬","腊"];
var lunarDayName = [
    "初一","初二","初三","初四","初五","初六","初七","初八","初九","初十",
    "十一","十二","十三","十四","十五","十六","十七","十八","十九","二十",
    "廿一","廿二","廿三","廿四","廿五","廿六","廿七","廿八","廿九","三十"
];

// 农历节日
var lunarFestivals = {
    "1-1": "春节", "1-15": "元宵", "5-5": "端午",
    "7-7": "七夕", "7-15": "中元", "8-15": "中秋",
    "9-9": "重阳", "12-8": "腊八", "12-30": "除夕"
};

// 公历节日
var solarFestivals = {
    "1-1": "元旦", "2-14": "情人节", "3-8": "妇女节", "4-1": "愚人节",
    "5-1": "劳动节", "6-1": "儿童节", "10-1": "国庆节", "12-25": "圣诞"
};

// 24 节气（简化近似算法）
var solarTermNames = [
    "小寒","大寒","立春","雨水","惊蛰","春分",
    "清明","谷雨","立夏","小满","芒种","夏至",
    "小暑","大暑","立秋","处暑","白露","秋分",
    "寒露","霜降","立冬","小雪","大雪","冬至"
];

// 节气近似日期（每月两个节气的日期，简化版）
var solarTermDays = [
    [5,20], [4,19], [6,21], [5,20], [6,21], [5,21],
    [7,23], [7,23], [8,23], [7,22], [7,22], [7,22]
];

function lYearDays(y) {
    var i, sum = 348;
    for (i = 0x8000; i > 0x8; i >>= 1)
        sum += (lunarInfo[y - 1900] & i) ? 1 : 0;
    return sum + leapDays(y);
}

function leapMonth(y) {
    return lunarInfo[y - 1900] & 0xf;
}

function leapDays(y) {
    if (leapMonth(y))
        return (lunarInfo[y - 1900] & 0x10000) ? 30 : 29;
    return 0;
}

function monthDays(y, m) {
    return (lunarInfo[y - 1900] & (0x10000 >> m)) ? 30 : 29;
}

function toLunar(year, month, day) {
    var baseDate = new Date(1900, 0, 31);
    var objDate = new Date(year, month - 1, day);
    var offset = Math.floor((objDate - baseDate) / 86400000);

    var lYear, lMonth, lDay, isLeap = false;
    var temp = 0;

    for (lYear = 1900; lYear < 2101 && offset > 0; lYear++) {
        temp = lYearDays(lYear);
        offset -= temp;
    }
    if (offset < 0) {
        offset += temp;
        lYear--;
    }

    var leap = leapMonth(lYear);
    var isLeapYear = false;

    for (lMonth = 1; lMonth < 13 && offset > 0; lMonth++) {
        if (leap > 0 && lMonth === (leap + 1) && !isLeapYear) {
            --lMonth;
            isLeapYear = true;
            temp = leapDays(lYear);
        } else {
            temp = monthDays(lYear, lMonth);
        }
        if (isLeapYear && lMonth === (leap + 1))
            isLeapYear = false;
        offset -= temp;
    }
    if (offset === 0 && leap > 0 && lMonth === leap + 1) {
        if (isLeapYear) {
            isLeapYear = false;
        } else {
            isLeapYear = true;
            --lMonth;
        }
    }
    if (offset < 0) {
        offset += temp;
        --lMonth;
    }
    lDay = offset + 1;
    isLeap = isLeapYear;

    // 天干地支年
    var ganIdx = (lYear - 4) % 10;
    var zhiIdx = (lYear - 4) % 12;
    var ganZhi = tianGan[ganIdx] + diZhi[zhiIdx];
    var animal = shengXiao[zhiIdx];

    // 农历日显示
    var dayStr = lunarDayName[lDay - 1];
    var monthStr = (isLeap ? "闰" : "") + lunarMonthName[lMonth - 1] + "月";

    // 节日检测
    var lunarKey = lMonth + "-" + lDay;
    var solarKey = month + "-" + day;
    var festival = lunarFestivals[lunarKey] || solarFestivals[solarKey] || "";

    // 除夕特殊处理（腊月最后一天）
    if (lMonth === 12 && !festival) {
        var lastDay = monthDays(lYear, 12);
        if (lDay === lastDay)
            festival = "除夕";
    }

    // 节气检测（简化）
    var term = "";
    if (month >= 1 && month <= 12) {
        var termIdx = (month - 1) * 2;
        if (day === solarTermDays[month - 1][0])
            term = solarTermNames[termIdx];
        else if (day === solarTermDays[month - 1][1])
            term = solarTermNames[termIdx + 1];
    }

    // 显示优先级：节日 > 节气 > 初一显示月份 > 日期
    var display = festival || term || (lDay === 1 ? monthStr : dayStr);

    return {
        year: lYear,
        month: lMonth,
        day: lDay,
        isLeap: isLeap,
        ganZhi: ganZhi,
        animal: animal,
        monthStr: monthStr,
        dayStr: dayStr,
        festival: festival,
        term: term,
        display: display,
        isFestival: festival !== "" || term !== ""
    };
}
