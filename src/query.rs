use base64::Engine;
use base64::engine::general_purpose::STANDARD;

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
}

#[derive(Debug, Clone)]
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

fn validate_airport(code: &str) -> Result<(), FlightError> {
    if code.len() != 3 || !code.chars().all(|c| c.is_ascii_uppercase()) {
        return Err(FlightError::InvalidAirport(code.to_string()));
    }
    Ok(())
}

fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400) {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

fn validate_date(date: &str) -> Result<(), FlightError> {
    let parts: Vec<&str> = date.split('-').collect();
    if parts.len() != 3 {
        return Err(FlightError::InvalidDate(date.to_string()));
    }
    let year: u32 = parts[0]
        .parse()
        .map_err(|_| FlightError::InvalidDate(date.to_string()))?;
    let month: u32 = parts[1]
        .parse()
        .map_err(|_| FlightError::InvalidDate(date.to_string()))?;
    let day: u32 = parts[2]
        .parse()
        .map_err(|_| FlightError::InvalidDate(date.to_string()))?;

    if year < 2000 || !(1..=12).contains(&month) {
        return Err(FlightError::InvalidDate(date.to_string()));
    }

    if day < 1 || day > days_in_month(year, month) {
        return Err(FlightError::InvalidDate(date.to_string()));
    }

    Ok(())
}

impl QueryParams {
    pub fn validate(&self) -> Result<(), FlightError> {
        if self.legs.is_empty() {
            return Err(FlightError::Validation(
                "at least one flight leg required".into(),
            ));
        }

        for leg in &self.legs {
            validate_airport(&leg.from_airport)?;
            validate_airport(&leg.to_airport)?;
            validate_date(&leg.date)?;
        }

        let total = self.passengers.adults
            + self.passengers.children
            + self.passengers.infants_in_seat
            + self.passengers.infants_on_lap;

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

        if self.passengers.infants_on_lap > self.passengers.adults {
            return Err(FlightError::Validation(
                "infants on lap cannot exceed number of adults".into(),
            ));
        }

        Ok(())
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

pub enum SearchQuery {
    Structured(QueryParams),
    NaturalLanguage(String),
}

impl SearchQuery {
    pub fn to_url_params(&self) -> Vec<(String, String)> {
        match self {
            Self::Structured(q) => q.to_url_params(),
            Self::NaturalLanguage(text) => vec![("q".to_string(), text.clone())],
        }
    }
}
