use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use base64::Engine;
use serde_json::{json, Value};

use crate::date::Date;
use crate::error::FlightError;
use crate::proto;

#[derive(Debug, Clone)]
pub struct FlightLeg {
    pub date: String,
    pub from_airport: String,
    pub to_airport: String,
    pub max_stops: Option<u32>,
    pub airlines: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct Passengers {
    pub adults: u32,
    pub children: u32,
    pub infants_in_seat: u32,
    pub infants_on_lap: u32,
}

impl Default for Passengers {
    fn default() -> Self {
        Self {
            adults: 1,
            children: 0,
            infants_in_seat: 0,
            infants_on_lap: 0,
        }
    }
}

impl Passengers {
    pub fn validate(&self) -> Result<(), FlightError> {
        let total = self.adults + self.children + self.infants_in_seat + self.infants_on_lap;

        if total > 9 {
            return Err(FlightError::Validation(format!(
                "total passengers ({total}) exceeds maximum of 9"
            )));
        }

        if total == 0 {
            return Err(FlightError::Validation(
                "at least one passenger required".into(),
            ));
        }

        if self.infants_on_lap > self.adults {
            return Err(FlightError::Validation(
                "infants on lap cannot exceed number of adults".into(),
            ));
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum Seat {
    Economy,
    PremiumEconomy,
    Business,
    First,
}

impl Seat {
    pub fn from_str_loose(s: &str) -> Result<Self, FlightError> {
        match s {
            "economy" => Ok(Self::Economy),
            "premium-economy" => Ok(Self::PremiumEconomy),
            "business" => Ok(Self::Business),
            "first" => Ok(Self::First),
            _ => Err(FlightError::Validation(format!("invalid seat class: {s}"))),
        }
    }

    pub fn code(&self) -> u64 {
        match self {
            Self::Economy => 1,
            Self::PremiumEconomy => 2,
            Self::Business => 3,
            Self::First => 4,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TripType {
    RoundTrip,
    OneWay,
    MultiCity,
}

impl TripType {
    pub fn from_str_loose(s: &str) -> Result<Self, FlightError> {
        match s {
            "round-trip" => Ok(Self::RoundTrip),
            "one-way" => Ok(Self::OneWay),
            "multi-city" => Ok(Self::MultiCity),
            _ => Err(FlightError::Validation(format!("invalid trip type: {s}"))),
        }
    }

    pub fn code(&self) -> u64 {
        match self {
            Self::RoundTrip => 1,
            Self::OneWay => 2,
            Self::MultiCity => 3,
        }
    }
}

/// Everything about a search except the route: filters, passengers, cabin and locale.
#[derive(Debug, Clone)]
pub struct Filters {
    pub max_stops: Option<u32>,
    pub airlines: Option<Vec<String>>,
    pub passengers: Passengers,
    pub seat: Seat,
    pub language: String,
    pub currency: String,
}

impl Default for Filters {
    fn default() -> Self {
        Self {
            max_stops: None,
            airlines: None,
            passengers: Passengers::default(),
            seat: Seat::Economy,
            language: "en".into(),
            currency: "USD".into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct QueryParams {
    pub legs: Vec<FlightLeg>,
    pub passengers: Passengers,
    pub seat: Seat,
    pub trip: TripType,
    pub language: String,
    pub currency: String,
}

/// Splits a comma-separated list of codes, uppercased, without blanks or repeats.
pub fn parse_codes(list: &str) -> Vec<String> {
    let mut codes: Vec<String> = Vec::new();
    for code in list.split(',').map(|c| c.trim().to_uppercase()) {
        if !code.is_empty() && !codes.contains(&code) {
            codes.push(code);
        }
    }
    codes
}

fn validate_airport(code: &str) -> Result<(), FlightError> {
    if code.len() != 3 || !code.chars().all(|c| c.is_ascii_uppercase()) {
        return Err(FlightError::InvalidAirport(code.to_string()));
    }
    Ok(())
}

fn validate_date(date: &str) -> Result<Date, FlightError> {
    Date::parse(date).ok_or_else(|| FlightError::InvalidDate(date.to_string()))
}

impl QueryParams {
    /// Builds a query from `(date, from, to)` legs, applying the same filters to every leg.
    pub fn new(legs: &[(&str, &str, &str)], trip: TripType, filters: &Filters) -> Self {
        Self {
            legs: legs
                .iter()
                .map(|&(date, from, to)| FlightLeg {
                    date: date.to_string(),
                    from_airport: from.to_string(),
                    to_airport: to.to_string(),
                    max_stops: filters.max_stops,
                    airlines: filters.airlines.clone(),
                })
                .collect(),
            passengers: filters.passengers.clone(),
            seat: filters.seat.clone(),
            trip,
            language: filters.language.clone(),
            currency: filters.currency.clone(),
        }
    }

    /// A one-way query, or a round trip when `return_date` is given.
    pub fn route(
        from: &str,
        to: &str,
        date: &str,
        return_date: Option<&str>,
        filters: &Filters,
    ) -> Self {
        match return_date {
            Some(ret) => {
                Self::new(&[(date, from, to), (ret, to, from)], TripType::RoundTrip, filters)
            }
            None => Self::new(&[(date, from, to)], TripType::OneWay, filters),
        }
    }

    pub fn validate(&self) -> Result<(), FlightError> {
        if self.legs.is_empty() {
            return Err(FlightError::Validation(
                "at least one flight leg required".into(),
            ));
        }

        let mut previous: Option<Date> = None;
        for leg in &self.legs {
            validate_airport(&leg.from_airport)?;
            validate_airport(&leg.to_airport)?;
            let date = validate_date(&leg.date)?;
            if previous.is_some_and(|p| date < p) {
                return Err(FlightError::Validation(format!(
                    "leg on {date} departs before the previous leg"
                )));
            }
            previous = Some(date);
        }

        match (&self.trip, self.legs.len()) {
            (TripType::OneWay, 1) | (TripType::RoundTrip, 2) | (TripType::MultiCity, 2..) => {}
            (TripType::RoundTrip, _) => {
                return Err(FlightError::Validation(
                    "round-trip needs exactly one return leg (use --return-date)".into(),
                ))
            }
            (TripType::MultiCity, _) => {
                return Err(FlightError::Validation(
                    "multi-city needs at least two legs (use --leg)".into(),
                ))
            }
            (TripType::OneWay, _) => {
                return Err(FlightError::Validation(
                    "one-way takes a single leg (use --trip multi-city for more)".into(),
                ))
            }
        }

        self.passengers.validate()
    }

    pub fn to_url_params(&self) -> Vec<(String, String)> {
        let encoded = proto::encode(&self.legs, &self.passengers, &self.seat, &self.trip);
        let b64 = STANDARD.encode(&encoded);

        let mut params = vec![("tfs".to_string(), b64)];

        if !self.language.is_empty() {
            params.push(("hl".to_string(), self.language.clone()));
        }
        if !self.currency.is_empty() {
            params.push(("curr".to_string(), self.currency.clone()));
        }

        params
    }
}

/// One query per destination, all from the same origin on the same dates.
pub fn route_queries(
    from: &str,
    destinations: &[String],
    date: &str,
    return_date: Option<&str>,
    filters: &Filters,
) -> Vec<(String, QueryParams)> {
    destinations
        .iter()
        .map(|to| (to.clone(), QueryParams::route(from, to, date, return_date, filters)))
        .collect()
}

pub fn to_google_flights_url(params: &QueryParams) -> String {
    let encoded = proto::encode(&params.legs, &params.passengers, &params.seat, &params.trip);
    let tfs = URL_SAFE_NO_PAD.encode(&encoded);

    let mut url = format!(
        "https://www.google.com/travel/flights/search?tfs={tfs}&tfu=EgYIABAAGAA"
    );

    if !params.currency.is_empty() {
        url.push_str(&format!("&curr={}", params.currency));
    }
    if !params.language.is_empty() {
        url.push_str(&format!("&hl={}", params.language));
    }

    url
}

/// Cheapest fare per departure date across a range, for one-way trips or for round
/// trips of a fixed length.
#[derive(Debug, Clone)]
pub struct DateQuery {
    pub from_airport: String,
    pub to_airport: String,
    pub start: Date,
    pub end: Date,
    pub stay_days: Option<u32>,
    pub filters: Filters,
}

/// Parses `YYYY-MM` (a whole month), `YYYY-MM-DD..YYYY-MM-DD`, or a single `YYYY-MM-DD`.
pub fn parse_date_range(spec: &str) -> Result<(Date, Date), FlightError> {
    let invalid = || {
        FlightError::Validation(format!(
            "invalid date range \"{spec}\" — use YYYY-MM for a month or YYYY-MM-DD..YYYY-MM-DD \
             (e.g. 2026-03 or 2026-03-01..2026-03-15)"
        ))
    };

    if let Some((start, end)) = spec.split_once("..") {
        let start = Date::parse(start).ok_or_else(invalid)?;
        let end = Date::parse(end).ok_or_else(invalid)?;
        if start > end {
            return Err(FlightError::Validation(format!(
                "date range \"{spec}\" ends before it starts"
            )));
        }
        return Ok((start, end));
    }

    if let Some(date) = Date::parse(spec) {
        return Ok((date, date));
    }

    let first = Date::parse(&format!("{spec}-01")).ok_or_else(invalid)?;
    Ok((first, first.last_of_month()))
}

impl DateQuery {
    /// The first date to price: past dates are skipped, since Google rejects them.
    pub fn first_date(&self) -> Date {
        self.start.max(Date::today())
    }

    pub fn validate(&self) -> Result<(), FlightError> {
        validate_airport(&self.from_airport)?;
        validate_airport(&self.to_airport)?;

        if self.start > self.end {
            return Err(FlightError::Validation(
                "date range ends before it starts".into(),
            ));
        }
        if self.end < Date::today() {
            return Err(FlightError::Validation(format!(
                "date range ending {} is in the past",
                self.end
            )));
        }

        self.filters.passengers.validate()
    }

    /// The form body for Google's `GetCalendarGraph` RPC, mirroring what the Google
    /// Flights date grid sends: `f.req=[null, "<filters as JSON>"]`.
    pub fn to_request_body(&self) -> String {
        let first = self.first_date();
        let (trip, segments) = match self.stay_days {
            Some(stay) => (
                TripType::RoundTrip,
                vec![
                    self.calendar_segment(&self.from_airport, &self.to_airport, first),
                    self.calendar_segment(
                        &self.to_airport,
                        &self.from_airport,
                        first.add_days(stay as i64),
                    ),
                ],
            ),
            None => (
                TripType::OneWay,
                vec![self.calendar_segment(&self.from_airport, &self.to_airport, first)],
            ),
        };

        let p = &self.filters.passengers;
        let settings = json!([
            null, null, trip.code(), null, [], self.filters.seat.code(),
            [p.adults, p.children, p.infants_on_lap, p.infants_in_seat],
            null, null, null, null, null, null, segments,
            null, null, null, 1
        ]);

        let mut filters = vec![
            Value::Null,
            settings,
            json!([first.to_string(), self.end.to_string()]),
        ];
        if let Some(stay) = self.stay_days {
            filters.extend([Value::Null, json!([stay, stay])]);
        }

        let wrapped = json!([null, Value::Array(filters).to_string()]);
        format!("f.req={}", urlencoding::encode(&wrapped.to_string()))
    }

    /// One leg of the calendar filter. Stops are encoded with 0 meaning any number,
    /// then 1 for nonstop, 2 for up to one stop and 3 for up to two.
    fn calendar_segment(&self, from: &str, to: &str, date: Date) -> Value {
        let stops = match self.filters.max_stops {
            Some(n @ 0..=2) => n + 1,
            _ => 0,
        };
        json!([
            [[[from, 0]]], [[[to, 0]]], null, stops, self.filters.airlines, null,
            date.to_string(), null, null, null, null, null, null, null, 3
        ])
    }
}
