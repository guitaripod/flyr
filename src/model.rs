use serde::Serialize;

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

#[derive(Debug, Clone, Default, Serialize)]
pub struct SearchResult {
    pub flights: Vec<FlightResult>,
    pub metadata: SearchMetadata,
}
