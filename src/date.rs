use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Date {
    pub year: u32,
    pub month: u32,
    pub day: u32,
}

impl Date {
    /// Parses a strict `YYYY-MM-DD` calendar date from the year 2000 on.
    pub fn parse(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split('-').collect();
        let [year, month, day] = parts[..] else {
            return None;
        };
        let widths_ok = year.len() == 4 && month.len() == 2 && day.len() == 2;
        if !widths_ok || !s.bytes().all(|b| b.is_ascii_digit() || b == b'-') {
            return None;
        }
        let date = Self {
            year: year.parse().ok()?,
            month: month.parse().ok()?,
            day: day.parse().ok()?,
        };
        let valid = date.year >= 2000
            && (1..=12).contains(&date.month)
            && (1..=days_in_month(date.year, date.month)).contains(&date.day);
        valid.then_some(date)
    }

    /// Today's date in UTC.
    pub fn today() -> Self {
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        Self::from_days((secs / 86_400) as i64)
    }

    /// Days since 1970-01-01, using Howard Hinnant's `days_from_civil`.
    pub fn days(self) -> i64 {
        let (month, day) = (self.month as i64, self.day as i64);
        let year = self.year as i64 - i64::from(month <= 2);
        let era = year.div_euclid(400);
        let year_of_era = year - era * 400;
        let day_of_year = (153 * ((month + 9) % 12) + 2) / 5 + day - 1;
        let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
        era * 146_097 + day_of_era - 719_468
    }

    /// Inverse of [`Date::days`], using Howard Hinnant's `civil_from_days`.
    pub fn from_days(days: i64) -> Self {
        let z = days + 719_468;
        let era = z.div_euclid(146_097);
        let day_of_era = z - era * 146_097;
        let year_of_era =
            (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
        let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
        let shifted_month = (5 * day_of_year + 2) / 153;
        let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
        let month = if shifted_month < 10 { shifted_month + 3 } else { shifted_month - 9 };
        let year = year_of_era + era * 400 + i64::from(month <= 2);
        Self {
            year: year as u32,
            month: month as u32,
            day: day as u32,
        }
    }

    pub fn add_days(self, days: i64) -> Self {
        Self::from_days(self.days() + days)
    }

    pub fn last_of_month(self) -> Self {
        Self {
            day: days_in_month(self.year, self.month),
            ..self
        }
    }

    pub fn weekday(self) -> &'static str {
        ["Thu", "Fri", "Sat", "Sun", "Mon", "Tue", "Wed"][self.days().rem_euclid(7) as usize]
    }
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

fn is_leap_year(year: u32) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}

pub fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}
