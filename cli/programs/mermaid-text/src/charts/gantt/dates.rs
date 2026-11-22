use rustc_hash::FxHashSet;

use chrono::{
    DateTime, Datelike, Months, NaiveDate, Utc, Weekday,
    format::{Parsed, StrftimeItems, parse, parse_and_remainder},
};

pub(super) const DAY: i64 = 86_400_000;
pub(super) fn datetime(ms: i64) -> Result<DateTime<Utc>, String> {
    DateTime::from_timestamp_millis(ms).ok_or_else(|| "date outside supported range".into())
}

pub(super) struct DateFormat {
    source: String,
    pattern: String,
    ordinal: bool,
    short_fraction: Option<(usize, usize)>,
}

impl DateFormat {
    pub fn new(source: &str) -> Result<Self, String> {
        let mut pattern = String::new();
        let mut ordinal = false;
        let mut short_fraction = None;
        let mut rest = if source == "yyyy-mm-dd" {
            "YYYY-MM-DD"
        } else {
            source
        };
        while !rest.is_empty() {
            if let Some(literal) = rest.strip_prefix('[') {
                let (literal, tail) = literal
                    .split_once(']')
                    .ok_or("unclosed dateFormat literal")?;
                pattern.push_str(&literal.replace('%', "%%"));
                rest = tail;
                continue;
            }
            let tokens = [
                ("YYYY", "%Y"),
                ("YY", "%y"),
                ("MMMM", "%B"),
                ("MMM", "%b"),
                ("MM", "%m"),
                ("M", "%m"),
                ("dddd", "%A"),
                ("ddd", "%a"),
                ("DDDD", "%j"),
                ("DDD", "%j"),
                ("DD", "%d"),
                ("Do", "%d"),
                ("D", "%d"),
                ("HH", "%H"),
                ("H", "%H"),
                ("hh", "%I"),
                ("h", "%I"),
                ("mm", "%M"),
                ("m", "%M"),
                ("ss", "%S"),
                ("s", "%S"),
                ("SSS", "%3f"),
                ("SS", ""),
                ("S", ""),
                ("ZZ", "%z"),
                ("Z", "%#z"),
                ("a", "%p"),
                ("A", "%p"),
                ("Q", "%q"),
                ("X", "%s"),
                ("x", "%s"),
            ];
            if let Some(&(token, replacement)) = tokens.iter().find(|(t, _)| rest.starts_with(t)) {
                if matches!(token, "X" | "x") && token != source {
                    return Err("X and x must be standalone date formats".into());
                }
                ordinal |= token == "Do";
                if matches!(token, "S" | "SS") {
                    if short_fraction.is_some() {
                        return Err("repeated gantt fractional seconds token".into());
                    }
                    short_fraction = Some((pattern.len(), token.len()));
                }
                pattern.push_str(replacement);
                rest = &rest[token.len()..];
            } else {
                let c = rest.chars().next().unwrap();
                if c.is_ascii_alphabetic() && c != 'T' {
                    return Err(format!("unsupported gantt dateFormat token at `{rest}`"));
                }
                if c == '%' {
                    pattern.push('%');
                }
                pattern.push(c);
                rest = &rest[c.len_utf8()..];
            }
        }
        Ok(Self {
            source: source.into(),
            pattern,
            ordinal,
            short_fraction,
        })
    }

    pub fn parse(&self, text: &str, today: NaiveDate) -> Result<i64, String> {
        let invalid = || format!("`{text}` does not match gantt dateFormat `{}`", self.source);
        if self.source == "x" || self.source == "X" {
            let ms = if self.source == "x" {
                text.parse().map_err(|_| invalid())?
            } else {
                scaled_number(text, 1000.0, true).ok_or_else(invalid)?
            };
            return Ok(datetime(ms)?.timestamp_millis());
        }
        let mut p = Parsed::new();
        let mut normalized = text.to_owned();
        if self.ordinal {
            for suffix in ["st", "nd", "rd", "th"] {
                if let Some((i, _)) = normalized
                    .match_indices(suffix)
                    .find(|(i, _)| *i > 0 && normalized.as_bytes()[i - 1].is_ascii_digit())
                {
                    normalized.replace_range(i..i + 2, "");
                    break;
                }
            }
        }
        let mut text = normalized.as_str();
        let mut pattern = self.pattern.as_str();
        if let Some((i, count)) = self.short_fraction {
            text = parse_and_remainder(&mut p, text, StrftimeItems::new(&pattern[..i]))
                .map_err(|_| invalid())?;
            let digits = text
                .get(..count)
                .filter(|s| s.bytes().all(|b| b.is_ascii_digit()))
                .ok_or_else(invalid)?;
            p.nanosecond = Some(digits.parse::<u32>().unwrap() * 10u32.pow(9 - count as u32));
            text = &text[count..];
            pattern = &pattern[i..];
        }
        parse(&mut p, text, StrftimeItems::new(pattern)).map_err(|_| invalid())?;
        let has_year = p.year.is_some() || p.year_mod_100.is_some();
        let has_month = p.month.is_some() || p.quarter.is_some();
        if !has_year {
            p.year = Some(today.year());
        } else if p.year.is_none()
            && let Some(y) = p.year_mod_100
        {
            p.year = Some(if y <= 68 { 2000 } else { 1900 } + y);
        }
        if p.ordinal.is_none() {
            p.month = Some(
                p.month
                    .or_else(|| p.quarter.map(|q| 3 * q - 2))
                    .unwrap_or(if has_year { 1 } else { today.month() }),
            );
            p.day.get_or_insert(if has_year || has_month {
                1
            } else {
                today.day()
            });
        }
        p.hour_div_12.get_or_insert(0);
        p.hour_mod_12.get_or_insert(0);
        p.minute.get_or_insert(0);
        p.second.get_or_insert(0);
        p.offset.get_or_insert(0);
        p.to_datetime()
            .map(|d| d.timestamp_millis())
            .map_err(|_| invalid())
    }
}

fn scaled_number(text: &str, scale: f64, signed: bool) -> Option<i64> {
    let digits = if signed {
        text.strip_prefix('-').unwrap_or(text)
    } else {
        text
    };
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit() || b == b'.') {
        return None;
    }
    let n = (text.parse::<f64>().ok()? * scale).round();
    (n.is_finite() && n.abs() < i64::MAX as f64).then_some(n as i64)
}

#[derive(Clone, Copy)]
pub(super) enum Span {
    Millis(i64),
    Months(u32),
}

impl Span {
    pub fn parse(text: &str) -> Option<Self> {
        let i = text.find(|c: char| c.is_ascii_alphabetic())?;
        let (number, unit) = text.split_at(i);
        let scale = match unit {
            "ms" => 1,
            "s" => 1000,
            "m" => 60_000,
            "h" => 3_600_000,
            "d" => DAY,
            "w" => 7 * DAY,
            "M" | "y" => {
                let months = scaled_number(number, if unit == "y" { 12.0 } else { 1.0 }, false)?;
                return Some(Self::Months(months.try_into().ok()?));
            }
            _ => return None,
        };
        Some(Self::Millis(scaled_number(number, scale as f64, false)?))
    }
}

#[derive(Default)]
pub(super) struct DaySet {
    weekdays: [bool; 7],
    dates: FxHashSet<NaiveDate>,
}

impl DaySet {
    pub fn add(
        &mut self,
        value: &str,
        weekend: Weekday,
        dates: &DateFormat,
        today: NaiveDate,
    ) -> Result<(), String> {
        match value.to_ascii_lowercase().as_str() {
            // Mermaid accepts this calendar entry without matching any dates.
            "weekdays" => (),
            "weekends" => {
                self.weekdays[weekend.num_days_from_monday() as usize] = true;
                self.weekdays[weekend.succ().num_days_from_monday() as usize] = true;
            }
            other => {
                if let Ok(day) = other.parse::<Weekday>() {
                    self.weekdays[day.num_days_from_monday() as usize] = true;
                } else {
                    let day = NaiveDate::parse_from_str(value, "%Y-%m-%d")
                        .ok()
                        .or_else(|| {
                            datetime(dates.parse(value, today).ok()?)
                                .ok()
                                .map(|d| d.date_naive())
                        })
                        .ok_or_else(|| format!("unknown gantt calendar date or day `{value}`"))?;
                    self.dates.insert(day);
                }
            }
        }
        Ok(())
    }
    fn contains(&self, date: NaiveDate) -> bool {
        self.weekdays[date.weekday().num_days_from_monday() as usize] || self.dates.contains(&date)
    }
}

#[derive(Default)]
pub(super) struct Calendar {
    pub excludes: DaySet,
    pub includes: DaySet,
    days_examined: usize,
}

impl Calendar {
    pub fn has_exclusions(&self) -> bool {
        !self.excludes.dates.is_empty() || self.excludes.weekdays.contains(&true)
    }

    pub fn excluded(&self, ms: i64) -> Result<bool, String> {
        let date = datetime(ms)?.date_naive();
        Ok(self.excludes.contains(date) && !self.includes.contains(date))
    }
    fn advance_day(&mut self, ms: i64) -> Result<i64, String> {
        self.days_examined += 1;
        if self.days_examined > 1_000_000 {
            return Err("gantt exceeds one million calendar-day steps".into());
        }
        Ok(datetime(ms - ms.rem_euclid(DAY) + DAY)?.timestamp_millis())
    }
    pub fn next_work_time(&mut self, mut ms: i64) -> Result<i64, String> {
        while self.excluded(ms)? {
            ms = self.advance_day(ms)?;
        }
        Ok(ms)
    }
    pub fn finish(&mut self, mut start: i64, span: Span) -> Result<i64, String> {
        let delta = match span {
            Span::Millis(d) => d,
            Span::Months(n) => {
                datetime(start)?
                    .checked_add_months(Months::new(n))
                    .ok_or("gantt duration outside supported date range")?
                    .timestamp_millis()
                    - start
            }
        };
        let nominal = start.checked_add(delta).ok_or("gantt duration overflow")?;
        datetime(nominal)?;
        if !self.has_exclusions() {
            return Ok(nominal);
        }
        let mut remaining = delta;
        while remaining > 0 {
            start = self.next_work_time(start)?;
            let midnight = self.advance_day(start)?;
            let work = remaining.min(midnight - start);
            start += work;
            remaining -= work;
        }
        Ok(start)
    }
}
