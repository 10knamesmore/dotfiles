//! 本地日期换算。
//!
//! transcript 里的 `timestamp` 是 UTC，按天分桶必须换到本地时区——UTC+8 直接用 UTC
//! 日期的话，本地凌晨 0–8 点干的活会全部算进前一天。

use std::time::{Duration, SystemTime};

use jiff::civil::Date;
use jiff::tz::TimeZone;
use jiff::{Span, Timestamp, Zoned};

/// 本地日期：按天分桶的键，序列化成 `YYYY-MM-DD`。
pub type Day = Date;

/// 今天（本地时区）。
#[must_use]
pub fn today() -> Day {
    Zoned::now().date()
}

/// 解析 `YYYY-MM-DD`。
#[must_use]
pub fn parse_day(text: &str) -> Option<Day> {
    text.parse().ok()
}

/// UTC 时间戳字符串（`2026-07-29T01:23:45.678Z`）落在本地哪一天。
#[must_use]
pub fn day_of(timestamp: &str) -> Option<Day> {
    let stamp: Timestamp = timestamp.parse().ok()?;
    Some(stamp.to_zoned(TimeZone::system()).date())
}

/// 本地某天零点对应的系统时间（用来筛 mtime）。
#[must_use]
pub fn day_start(day: Day) -> Option<SystemTime> {
    let secs = day
        .to_zoned(TimeZone::system())
        .ok()?
        .timestamp()
        .as_second();
    let offset = Duration::from_secs(secs.unsigned_abs());
    if secs >= 0 {
        SystemTime::UNIX_EPOCH.checked_add(offset)
    } else {
        SystemTime::UNIX_EPOCH.checked_sub(offset)
    }
}

/// `day` 往前 `count` 天。
#[must_use]
pub fn days_before(day: Day, count: i64) -> Option<Day> {
    day.checked_sub(Span::new().days(count)).ok()
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::min_ident_chars,
        clippy::missing_docs_in_private_items
    )]
    use super::*;

    #[test]
    fn day_of_uses_local_timezone() {
        // TZ 在进程内由 jiff 读系统设置，这里只断言解析与跨日不 panic。
        let day = day_of("2026-07-29T01:23:45.678Z").expect("合法时间戳");
        let neighbours = [
            days_before(day, 1).expect("前一天"),
            day,
            days_before(day, -1).expect("后一天"),
        ];
        assert_eq!(neighbours[0].to_string().len(), 10);
        assert!(neighbours[0] < neighbours[1] && neighbours[1] < neighbours[2]);
    }

    #[test]
    fn bad_timestamp_is_none() {
        assert!(day_of("not-a-timestamp").is_none());
        assert!(parse_day("2026-13-99").is_none());
    }

    #[test]
    fn day_start_is_midnight_before_any_stamp_that_day() {
        let day = parse_day("2026-07-29").expect("合法日期");
        let start = day_start(day).expect("零点");
        let noon = day_of("2026-07-29T12:00:00Z").expect("合法时间戳");
        assert!(day_start(noon).expect("零点") >= start);
    }
}
