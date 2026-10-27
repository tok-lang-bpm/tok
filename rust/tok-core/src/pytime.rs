//! The parts of Python's `datetime` module that FEEL values use: `date`, `timedelta` and
//! `datetime` (with an optional fixed UTC offset), with Python's arithmetic, comparison,
//! `str`, `repr` and `isoformat`, and the same exceptions.

use std::cmp::Ordering;

use crate::pyutil::PyError;

const MAX_ORDINAL: i64 = 3_652_059;
const MAX_DELTA_DAYS: i64 = 999_999_999;
const US_PER_DAY: i128 = 86_400_000_000;

fn is_leap(y: i64) -> bool {
    y % 4 == 0 && (y % 100 != 0 || y % 400 == 0)
}

fn days_in_month(y: i64, m: u32) -> u32 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ if is_leap(y) => 29,
        _ => 28,
    }
}

fn days_before_year(y: i64) -> i64 {
    let y = y - 1;
    y * 365 + y / 4 - y / 100 + y / 400
}

fn days_before_month(y: i64, m: u32) -> i64 {
    (1..m).map(|k| days_in_month(y, k) as i64).sum()
}

fn date_overflow() -> PyError {
    PyError::new("OverflowError", "date value out of range")
}

/// `datetime.date`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Date {
    pub year: i64,
    pub month: u32,
    pub day: u32,
}

impl Date {
    /// `datetime.date(year, month, day)`.
    pub fn new(year: i128, month: i128, day: i128) -> Result<Date, PyError> {
        if !(1..=9999).contains(&year) {
            return Err(PyError::new(
                "ValueError",
                format!("year {year} is out of range"),
            ));
        }
        if !(1..=12).contains(&month) {
            return Err(PyError::new("ValueError", "month must be in 1..12"));
        }
        let dim = days_in_month(year as i64, month as u32) as i128;
        if !(1..=dim).contains(&day) {
            return Err(PyError::new("ValueError", "day is out of range for month"));
        }
        Ok(Date {
            year: year as i64,
            month: month as u32,
            day: day as u32,
        })
    }

    /// `toordinal()`: 1 for 0001-01-01.
    pub fn ordinal(&self) -> i64 {
        days_before_year(self.year) + days_before_month(self.year, self.month) + self.day as i64
    }

    /// `date.fromordinal(n)`; `OverflowError` outside 1..=3652059.
    pub fn from_ordinal(n: i64) -> Result<Date, PyError> {
        if !(1..=MAX_ORDINAL).contains(&n) {
            return Err(date_overflow());
        }
        let mut year = (n - 1) / 366 + 1;
        while days_before_year(year + 1) < n {
            year += 1;
        }
        let mut rest = n - days_before_year(year);
        let mut month = 1;
        while rest > days_in_month(year, month) as i64 {
            rest -= days_in_month(year, month) as i64;
            month += 1;
        }
        Ok(Date {
            year,
            month,
            day: rest as u32,
        })
    }

    /// The date of a number of days since 1970-01-01.
    pub fn from_unix_days(days: i64) -> Result<Date, PyError> {
        Date::from_ordinal(days + 719_163)
    }

    pub fn isoformat(&self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }

    pub fn repr(&self) -> String {
        format!("datetime.date({}, {}, {})", self.year, self.month, self.day)
    }

    /// `self + delta` (only the days of the delta count).
    pub fn add(&self, delta: &Duration) -> Result<Date, PyError> {
        Date::from_ordinal(self.ordinal() + delta.days)
    }

    /// `self - delta`.
    pub fn sub(&self, delta: &Duration) -> Result<Date, PyError> {
        Date::from_ordinal(self.ordinal() - delta.days)
    }
}

/// `datetime.timedelta`, normalized like Python: `0 <= seconds < 86400`,
/// `0 <= microseconds < 1000000`, `|days| <= 999999999`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Duration {
    pub days: i64,
    pub seconds: i64,
    pub microseconds: i64,
}

impl Duration {
    /// A duration of a number of microseconds; `OverflowError` beyond the day limit.
    pub fn from_micros(us: i128) -> Result<Duration, PyError> {
        let days = us.div_euclid(US_PER_DAY);
        let rest = us.rem_euclid(US_PER_DAY);
        if days.abs() > MAX_DELTA_DAYS as i128 {
            if days.abs() > i32::MAX as i128 {
                return Err(PyError::new(
                    "OverflowError",
                    "Python int too large to convert to C int",
                ));
            }
            return Err(PyError::new(
                "OverflowError",
                format!("days={days}; must have magnitude <= {MAX_DELTA_DAYS}"),
            ));
        }
        Ok(Duration {
            days: days as i64,
            seconds: (rest / 1_000_000) as i64,
            microseconds: (rest % 1_000_000) as i64,
        })
    }

    /// `timedelta(days=, hours=, minutes=, seconds=)` from floats, with the rounding of
    /// CPython's constructor (integral parts exact, fractions rounded half to even).
    pub fn from_parts(
        days: f64,
        hours: f64,
        minutes: f64,
        seconds: f64,
    ) -> Result<Duration, PyError> {
        let mut total: i128 = 0;
        let mut leftover = 0.0f64;
        let too_big = || PyError::new("OverflowError", "Python int too large to convert to C int");
        let mut accum = |value: f64, factor: i128, total: &mut i128| -> Result<(), PyError> {
            if value == 0.0 {
                return Ok(());
            }
            let int = value.trunc();
            let frac = value - int;
            if int.abs() >= 1e36 {
                return Err(too_big());
            }
            *total += (int as i128) * factor;
            if frac == 0.0 {
                return Ok(());
            }
            let scaled = factor as f64 * frac;
            let int2 = scaled.trunc();
            *total += int2 as i128;
            leftover += scaled - int2;
            Ok(())
        };
        accum(seconds, 1_000_000, &mut total)?;
        accum(minutes, 60_000_000, &mut total)?;
        accum(hours, 3_600_000_000, &mut total)?;
        accum(days, US_PER_DAY, &mut total)?;
        if leftover != 0.0 {
            let mut whole = leftover.round();
            if (whole - leftover).abs() == 0.5 {
                let odd = (total.rem_euclid(2)) as f64;
                whole = 2.0 * ((leftover + odd) * 0.5).round() - odd;
            }
            total += whole as i128;
        }
        Duration::from_micros(total)
    }

    pub fn total_micros(&self) -> i128 {
        self.days as i128 * US_PER_DAY
            + self.seconds as i128 * 1_000_000
            + self.microseconds as i128
    }

    pub fn neg(&self) -> Result<Duration, PyError> {
        Duration::from_micros(-self.total_micros())
    }

    pub fn add(&self, other: &Duration) -> Result<Duration, PyError> {
        Duration::from_micros(self.total_micros() + other.total_micros())
    }

    pub fn sub(&self, other: &Duration) -> Result<Duration, PyError> {
        Duration::from_micros(self.total_micros() - other.total_micros())
    }

    /// `str(timedelta)`: `1 day, 2:00:00`, `-1 day, 23:59:59.500000`.
    pub fn to_str(&self) -> String {
        let (hh, rem) = (self.seconds / 3600, self.seconds % 3600);
        let (mm, ss) = (rem / 60, rem % 60);
        let mut s = format!("{hh}:{mm:02}:{ss:02}");
        if self.days != 0 {
            let plural = if self.days.abs() != 1 { "s" } else { "" };
            s = format!("{} day{plural}, {s}", self.days);
        }
        if self.microseconds != 0 {
            s.push_str(&format!(".{:06}", self.microseconds));
        }
        s
    }

    /// `repr(timedelta)`.
    pub fn repr(&self) -> String {
        let mut args = Vec::new();
        if self.days != 0 {
            args.push(format!("days={}", self.days));
        }
        if self.seconds != 0 {
            args.push(format!("seconds={}", self.seconds));
        }
        if self.microseconds != 0 {
            args.push(format!("microseconds={}", self.microseconds));
        }
        if args.is_empty() {
            args.push("0".into());
        }
        format!("datetime.timedelta({})", args.join(", "))
    }
}

/// `datetime.datetime` with an optional fixed UTC offset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DateTime {
    pub date: Date,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
    pub microsecond: u32,
    pub offset: Option<Duration>,
}

impl DateTime {
    /// The current time in UTC (`datetime.now(timezone.utc)`).
    pub fn now_utc() -> DateTime {
        let us = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_micros() as i128)
            .unwrap_or(0);
        let days = us.div_euclid(US_PER_DAY) as i64;
        let rest = us.rem_euclid(US_PER_DAY) as i64;
        let date = Date::from_unix_days(days).unwrap_or(Date {
            year: 1970,
            month: 1,
            day: 1,
        });
        let secs = rest / 1_000_000;
        DateTime {
            date,
            hour: (secs / 3600) as u32,
            minute: (secs % 3600 / 60) as u32,
            second: (secs % 60) as u32,
            microsecond: (rest % 1_000_000) as u32,
            offset: Some(Duration::default()),
        }
    }

    fn local_micros(&self) -> i128 {
        self.date.ordinal() as i128 * US_PER_DAY
            + ((self.hour as i128 * 60 + self.minute as i128) * 60 + self.second as i128)
                * 1_000_000
            + self.microsecond as i128
    }

    fn utc_micros(&self) -> i128 {
        self.local_micros() - self.offset.map(|o| o.total_micros()).unwrap_or(0)
    }

    fn from_local_micros(us: i128, offset: Option<Duration>) -> Result<DateTime, PyError> {
        let days = us.div_euclid(US_PER_DAY);
        let rest = us.rem_euclid(US_PER_DAY) as i64;
        if !(1..=MAX_ORDINAL as i128).contains(&days) {
            return Err(date_overflow());
        }
        let secs = rest / 1_000_000;
        Ok(DateTime {
            date: Date::from_ordinal(days as i64)?,
            hour: (secs / 3600) as u32,
            minute: (secs % 3600 / 60) as u32,
            second: (secs % 60) as u32,
            microsecond: (rest % 1_000_000) as u32,
            offset,
        })
    }

    pub fn add(&self, delta: &Duration) -> Result<DateTime, PyError> {
        DateTime::from_local_micros(self.local_micros() + delta.total_micros(), self.offset)
    }

    pub fn sub(&self, delta: &Duration) -> Result<DateTime, PyError> {
        DateTime::from_local_micros(self.local_micros() - delta.total_micros(), self.offset)
    }

    /// `==`: naive and aware values are never equal.
    pub fn py_eq(&self, other: &DateTime) -> bool {
        match (self.offset, other.offset) {
            (None, None) => self.local_micros() == other.local_micros(),
            (Some(_), Some(_)) => self.utc_micros() == other.utc_micros(),
            _ => false,
        }
    }

    /// Ordering; mixing naive and aware values raises `TypeError`.
    pub fn py_cmp(&self, other: &DateTime) -> Result<Ordering, PyError> {
        match (self.offset, other.offset) {
            (None, None) => Ok(self.local_micros().cmp(&other.local_micros())),
            (Some(_), Some(_)) => Ok(self.utc_micros().cmp(&other.utc_micros())),
            _ => Err(PyError::new(
                "TypeError",
                "can't compare offset-naive and offset-aware datetimes",
            )),
        }
    }

    fn offset_text(&self, sep: &str) -> String {
        let Some(off) = self.offset else {
            return String::new();
        };
        let (sign, d) = if off.days < 0 {
            ("-", off.neg().unwrap_or(off))
        } else {
            ("+", off)
        };
        let secs = d.days * 86_400 + d.seconds;
        let mut s = format!("{sign}{:02}{sep}{:02}", secs / 3600, secs % 3600 / 60);
        if secs % 60 != 0 || d.microseconds != 0 {
            s.push_str(&format!("{sep}{:02}", secs % 60));
            if d.microseconds != 0 {
                s.push_str(&format!(".{:06}", d.microseconds));
            }
        }
        s
    }

    /// `isoformat()`: `2026-10-27T12:00:00.123456+00:00`.
    pub fn isoformat(&self) -> String {
        let mut s = format!(
            "{}T{:02}:{:02}:{:02}",
            self.date.isoformat(),
            self.hour,
            self.minute,
            self.second
        );
        if self.microsecond != 0 {
            s.push_str(&format!(".{:06}", self.microsecond));
        }
        s.push_str(&self.offset_text(":"));
        s
    }

    /// `repr(datetime)` for UTC and fixed offsets.
    pub fn repr(&self) -> String {
        let mut parts = vec![
            self.date.year.to_string(),
            self.date.month.to_string(),
            self.date.day.to_string(),
            self.hour.to_string(),
            self.minute.to_string(),
        ];
        if self.second != 0 || self.microsecond != 0 {
            parts.push(self.second.to_string());
        }
        if self.microsecond != 0 {
            parts.push(self.microsecond.to_string());
        }
        if let Some(off) = self.offset {
            if off == Duration::default() {
                parts.push("tzinfo=datetime.timezone.utc".into());
            } else {
                parts.push(format!("tzinfo=datetime.timezone({})", off.repr()));
            }
        }
        format!("datetime.datetime({})", parts.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_and_durations() {
        let d = Date::new(2020, 1, 1).expect("date");
        assert_eq!(Date::from_ordinal(d.ordinal()), Ok(d));
        assert_eq!(
            Date::from_unix_days(0).map(|d| d.isoformat()),
            Ok("1970-01-01".into())
        );
        let td = Duration::from_parts(1.0, 25.0, 0.0, 0.0).expect("duration");
        assert_eq!(td.to_str(), "2 days, 1:00:00");
        assert_eq!(d.add(&td).map(|x| x.isoformat()), Ok("2020-01-03".into()));
        let neg = Duration::from_micros(-86_400_000_000 + 5_000_000).expect("duration");
        assert_eq!(neg.to_str(), "-1 day, 0:00:05");
        assert_eq!(neg.repr(), "datetime.timedelta(days=-1, seconds=5)");
        assert_eq!(Duration::default().repr(), "datetime.timedelta(0)");
        assert_eq!(
            Duration::from_parts(0.0, 0.0, 0.0, 1.5).map(|t| t.to_str()),
            Ok("0:00:01.500000".into())
        );
        assert!(Date::new(2021, 2, 29).is_err());
    }
}
