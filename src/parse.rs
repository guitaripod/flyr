use scraper::{Html, Selector};
use serde_json::Value;

use crate::error::FlightError;
use crate::model::*;

/// Payload sections holding flights: Google's "top flights", then "other flights".
const FLIGHT_SECTIONS: [usize; 2] = [2, 3];

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

/// Reads a `[year, month, day]` date and an `[hour, minute]` time. Google leaves zero
/// components out of times: 00:20 arrives as `[null, 20]` and 09:00 as `[9]`.
fn parse_datetime(date_val: &Value, time_val: &Value) -> Option<FlightDateTime> {
    time_val.as_array()?;
    Some(FlightDateTime {
        year: get_u32(date_val, 0)?,
        month: get_u32(date_val, 1)?,
        day: get_u32(date_val, 2)?,
        hour: get_u32(time_val, 0).unwrap_or(0),
        minute: get_u32(time_val, 1).unwrap_or(0),
    })
}

/// Joins a `[airline_code, number, ..]` entry into a designator like `BA175`.
fn flight_designator(val: &Value) -> Option<String> {
    Some(format!("{}{}", get_str(val, 0)?, get_str(val, 1)?))
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
    let flight_number = get_val(sf, 22).and_then(flight_designator);
    let codeshares = get_val(sf, 15)
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(flight_designator).collect())
        .unwrap_or_default();

    Some(Segment {
        from_airport,
        to_airport,
        departure,
        arrival,
        duration_minutes,
        aircraft,
        flight_number,
        codeshares,
    })
}

/// Describes the connection between each two segments, taking the wait from Google's
/// per-layover details and falling back to the local-time gap when they're missing.
fn parse_layovers(flight: &Value, segments: &[Segment]) -> Vec<Layover> {
    let details = get_val(flight, 13);

    segments
        .windows(2)
        .enumerate()
        .map(|(i, pair)| {
            let (inbound, outbound) = (&pair[0], &pair[1]);
            let detail = details.and_then(|d| get_val(d, i));
            let duration_minutes = detail.and_then(|d| get_u32(d, 0)).unwrap_or_else(|| {
                inbound.arrival.minutes_until(&outbound.departure).max(0) as u32
            });
            Layover {
                airport: inbound.to_airport.code.clone(),
                duration_minutes,
                overnight: inbound.arrival.date() != outbound.departure.date(),
                change_of_airport: inbound.to_airport.code != outbound.from_airport.code,
            }
        })
        .collect()
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

    let layovers = parse_layovers(flight, &segments);

    let duration_minutes = get_u32(flight, 9).unwrap_or_else(|| {
        segments.iter().map(|s| s.duration_minutes).sum::<u32>()
            + layovers.iter().map(|l| l.duration_minutes).sum::<u32>()
    });

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
        layovers,
        duration_minutes,
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

/// Reads a `[date, return_date, [[null, price], token], ..]` entry, shared by the
/// calendar response and the results page's nearby-date suggestion.
fn parse_date_price(entry: &Value) -> Option<DatePrice> {
    Some(DatePrice {
        date: get_str(entry, 0)?,
        return_date: get_str(entry, 1),
        price: get_val(entry, 2)
            .and_then(|v| get_val(v, 0))
            .and_then(|v| get_i64(v, 1))?,
    })
}

fn section_flights(payload: &Value, idx: usize) -> Result<Vec<FlightResult>, FlightError> {
    match get_val(payload, idx).and_then(|v| get_val(v, 0)) {
        Some(root) if !root.is_null() => root
            .as_array()
            .map(|arr| arr.iter().filter_map(parse_flight).collect())
            .ok_or_else(|| FlightError::JsParse(format!("payload[{idx}][0] is not an array"))),
        _ => Ok(Vec::new()),
    }
}

pub fn parse_payload(payload: &Value) -> Result<SearchResult, FlightError> {
    let metadata = parse_metadata(payload);

    let mut flights = Vec::new();
    for idx in FLIGHT_SECTIONS {
        flights.extend(section_flights(payload, idx)?);
    }

    let cheapest = flights.iter().filter_map(|f| f.price).min();
    let cheaper_date = get_val(payload, 6)
        .and_then(|v| get_val(v, 0))
        .and_then(|v| get_val(v, 0))
        .and_then(parse_date_price)
        .filter(|hint| cheapest.is_none_or(|c| hint.price < c));

    Ok(SearchResult {
        flights,
        cheaper_date,
        metadata,
    })
}

pub fn parse_html(html: &str) -> Result<SearchResult, FlightError> {
    let js = extract_script(html)?;
    let payload = parse_js(&js)?;
    parse_payload(&payload)
}

/// Decodes a `GetCalendarGraph` response: `)]}'`-prefixed lines, one of which is a
/// `wrb.fr` envelope whose third element is the JSON-encoded result, or null plus an
/// error code when Google rejects the request.
pub fn parse_calendar(body: &str) -> Result<Vec<DatePrice>, FlightError> {
    let envelope: Value = body
        .lines()
        .filter(|line| line.starts_with(r#"[["wrb.fr""#))
        .find_map(|line| serde_json::from_str(line).ok())
        .ok_or_else(|| FlightError::JsParse("no result envelope in date search response".into()))?;
    let row = get_val(&envelope, 0)
        .ok_or_else(|| FlightError::JsParse("empty date search envelope".into()))?;

    let Some(inner) = get_val(row, 2).and_then(|v| v.as_str()) else {
        return Err(FlightError::Rejected(get_val(row, 5).and_then(|v| get_i64(v, 0))));
    };
    let data: Value =
        serde_json::from_str(inner).map_err(|e| FlightError::JsParse(e.to_string()))?;

    let mut dates: Vec<DatePrice> = data
        .as_array()
        .and_then(|arr| arr.last())
        .and_then(|v| v.as_array())
        .map(|items| items.iter().filter_map(parse_date_price).collect())
        .unwrap_or_default();
    dates.sort_by(|a, b| a.date.cmp(&b.date));
    Ok(dates)
}
