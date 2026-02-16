use scraper::{Html, Selector};
use serde_json::Value;

use crate::error::FlightError;
use crate::model::*;

fn get_val(val: &Value, idx: usize) -> Option<&Value> {
    val.as_array().and_then(|arr| arr.get(idx))
}

fn get_str(val: &Value, idx: usize) -> Option<String> {
    get_val(val, idx).and_then(|v| v.as_str()).map(String::from)
}

fn get_i64(val: &Value, idx: usize) -> Option<i64> {
    get_val(val, idx).and_then(|v| v.as_i64())
}

fn get_u32(val: &Value, idx: usize) -> Option<u32> {
    get_val(val, idx).and_then(|v| v.as_u64()).map(|v| v as u32)
}

pub fn extract_script(html: &str) -> Result<String, FlightError> {
    let document = Html::parse_document(html);
    let selector =
        Selector::parse(r#"script[class="ds:1"]"#).expect("valid selector");

    document
        .select(&selector)
        .next()
        .map(|el| el.inner_html())
        .ok_or(FlightError::ScriptTagNotFound)
}

pub fn parse_js(js: &str) -> Result<Value, FlightError> {
    let data = js
        .split_once("data:")
        .map(|(_, rest)| rest)
        .ok_or_else(|| FlightError::JsParse("no 'data:' marker found".into()))?;

    let data = data
        .rsplit_once(',')
        .map(|(left, _)| left)
        .ok_or_else(|| FlightError::JsParse("no trailing comma found".into()))?;

    serde_json::from_str(data).map_err(|e| FlightError::JsParse(e.to_string()))
}

fn parse_datetime(date_val: &Value, time_val: &Value) -> Option<FlightDateTime> {
    Some(FlightDateTime {
        year: get_u32(date_val, 0)?,
        month: get_u32(date_val, 1)?,
        day: get_u32(date_val, 2)?,
        hour: get_u32(time_val, 0)?,
        minute: get_u32(time_val, 1).unwrap_or(0),
    })
}

fn parse_segment(sf: &Value) -> Option<Segment> {
    let from_airport = Airport {
        code: get_str(sf, 3)?,
        name: get_str(sf, 4).unwrap_or_default(),
    };

    let to_airport = Airport {
        code: get_str(sf, 6)?,
        name: get_str(sf, 5).unwrap_or_default(),
    };

    let departure_date = get_val(sf, 20)?;
    let departure_time = get_val(sf, 8)?;
    let departure = parse_datetime(departure_date, departure_time)?;

    let arrival_date = get_val(sf, 21)?;
    let arrival_time = get_val(sf, 10)?;
    let arrival = parse_datetime(arrival_date, arrival_time)?;

    let duration_minutes = get_u32(sf, 11).unwrap_or(0);
    let aircraft = get_str(sf, 17);

    Some(Segment {
        from_airport,
        to_airport,
        departure,
        arrival,
        duration_minutes,
        aircraft,
    })
}

fn parse_flight(k: &Value) -> Option<FlightResult> {
    let flight = get_val(k, 0)?;

    let flight_type = get_str(flight, 0).unwrap_or_default();

    let airlines: Vec<String> = get_val(flight, 1)
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();

    let segments_arr = get_val(flight, 2).and_then(|v| v.as_array());
    let segments: Vec<Segment> = segments_arr
        .map(|arr| arr.iter().filter_map(parse_segment).collect())
        .unwrap_or_default();

    let price = get_val(k, 1)
        .and_then(|v| get_val(v, 0))
        .and_then(|v| get_i64(v, 1));

    let extras = get_val(flight, 22);
    let carbon = CarbonEmission {
        emission_grams: extras.and_then(|e| get_i64(e, 7)),
        typical_grams: extras.and_then(|e| get_i64(e, 8)),
    };

    Some(FlightResult {
        flight_type,
        airlines,
        segments,
        price,
        carbon,
    })
}

fn parse_metadata(payload: &Value) -> SearchMetadata {
    let mut alliances = Vec::new();
    let mut airlines = Vec::new();

    if let Some(meta_root) = get_val(payload, 7)
        .and_then(|v| get_val(v, 1))
    {
        if let Some(alliances_data) = get_val(meta_root, 0).and_then(|v| v.as_array()) {
            for item in alliances_data {
                if let (Some(code), Some(name)) = (get_str(item, 0), get_str(item, 1)) {
                    alliances.push(Alliance { code, name });
                }
            }
        }

        if let Some(airlines_data) = get_val(meta_root, 1).and_then(|v| v.as_array()) {
            for item in airlines_data {
                if let (Some(code), Some(name)) = (get_str(item, 0), get_str(item, 1)) {
                    airlines.push(Airline { code, name });
                }
            }
        }
    }

    SearchMetadata {
        airlines,
        alliances,
    }
}

pub fn parse_payload(payload: &Value) -> Result<SearchResult, FlightError> {
    let metadata = parse_metadata(payload);

    let flights_root = get_val(payload, 3).and_then(|v| get_val(v, 0));

    let flights = match flights_root {
        Some(root) if !root.is_null() => {
            let arr = root
                .as_array()
                .ok_or_else(|| FlightError::JsParse("payload[3][0] is not an array".into()))?;
            arr.iter().filter_map(parse_flight).collect()
        }
        _ => Vec::new(),
    };

    Ok(SearchResult { flights, metadata })
}

pub fn parse_html(html: &str) -> Result<SearchResult, FlightError> {
    let js = extract_script(html)?;
    let payload = parse_js(&js)?;
    parse_payload(&payload)
}
