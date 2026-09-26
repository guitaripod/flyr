use serde::Serialize;

use crate::date::Date;

#[derive(Debug, Clone, Serialize)]
pub struct Airport {
    pub code: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct FlightDateTime {
    pub year: u32,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
}

impl FlightDateTime {
    pub fn date(&self) -> Date {
        Date {
            year: self.year,
            month: self.month,
            day: self.day,
        }
    }

    /// Minutes from `self` to `later`, both read as local times in the same zone.
    pub fn minutes_until(&self, later: &FlightDateTime) -> i64 {
        (later.date().days() - self.date().days()) * 1440
            + (later.hour as i64 * 60 + later.minute as i64)
            - (self.hour as i64 * 60 + self.minute as i64)
    }
}

impl std::fmt::Display for FlightDateTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:04}-{:02}-{:02} {:02}:{:02}",
            self.year, self.month, self.day, self.hour, self.minute
        )
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Segment {
    pub from_airport: Airport,
    pub to_airport: Airport,
    pub departure: FlightDateTime,
    pub arrival: FlightDateTime,
    pub duration_minutes: u32,
    pub aircraft: Option<String>,
    pub flight_number: Option<String>,
    pub codeshares: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Layover {
    pub airport: String,
    pub duration_minutes: u32,
    /// The connection spans midnight: you land on one day and leave on the next.
    pub overnight: bool,
    /// The next flight leaves from a different airport than this one landed at.
    pub change_of_airport: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct CarbonEmission {
    pub emission_grams: Option<i64>,
    pub typical_grams: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FlightResult {
    pub flight_type: String,
    pub airlines: Vec<String>,
    pub segments: Vec<Segment>,
    pub layovers: Vec<Layover>,
    pub duration_minutes: u32,
    pub price: Option<i64>,
    pub carbon: CarbonEmission,
}

#[derive(Debug, Clone, Serialize)]
pub struct Airline {
    pub code: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Alliance {
    pub code: String,
    pub name: String,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct SearchMetadata {
    pub airlines: Vec<Airline>,
    pub alliances: Vec<Alliance>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DatePrice {
    pub date: String,
    pub return_date: Option<String>,
    pub price: i64,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct SearchResult {
    pub flights: Vec<FlightResult>,
    pub cheaper_date: Option<DatePrice>,
    pub metadata: SearchMetadata,
}

impl SearchResult {
    /// Keeps the `n` cheapest flights, cheapest first; flights without a price sort last.
    pub fn keep_cheapest(&mut self, n: usize) {
        self.flights.sort_by_key(|f| f.price.unwrap_or(i64::MAX));
        self.flights.truncate(n);
    }
}

/// Keeps the `n` cheapest dates, cheapest first, with earlier dates winning ties.
pub fn keep_cheapest_dates(dates: &mut Vec<DatePrice>, n: usize) {
    dates.sort_by(|a, b| a.price.cmp(&b.price).then_with(|| a.date.cmp(&b.date)));
    dates.truncate(n);
}
