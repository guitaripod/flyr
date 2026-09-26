use comfy_table::{Table, ContentArrangement, presets::UTF8_FULL};

use crate::date::Date;
use crate::error::FlightError;
use crate::model::{DatePrice, FlightResult, Layover, SearchResult, Segment};

pub fn format_price(price: Option<i64>, currency: &str) -> String {
    let p = match price {
        Some(p) => p,
        None => return "—".to_string(),
    };
    match currency {
        "USD" => format!("${p}"),
        "EUR" => format!("€{p}"),
        "GBP" => format!("£{p}"),
        "JPY" | "CNY" => format!("¥{p}"),
        "KRW" => format!("₩{p}"),
        "INR" => format!("₹{p}"),
        "THB" => format!("฿{p}"),
        _ => format!("{p} {currency}"),
    }
}

pub fn format_duration(minutes: u32) -> String {
    format!("{}h{:02}m", minutes / 60, minutes % 60)
}

fn describe_layover(layover: &Layover) -> String {
    let mut text = format!("{} {}", layover.airport, format_duration(layover.duration_minutes));
    if layover.overnight {
        text.push_str(" overnight");
    }
    if layover.change_of_airport {
        text.push_str(" airport change");
    }
    text
}

/// A date with its weekday, e.g. `2026-11-03 Tue`, or the raw text if it isn't a date.
fn with_weekday(date: &str) -> String {
    Date::parse(date).map_or_else(|| date.to_string(), |d| format!("{d} {}", d.weekday()))
}

/// The arrival's day offset from departure, like Google's `+1`, or empty on the same day.
fn day_offset(flight: &FlightResult) -> String {
    let (Some(first), Some(last)) = (flight.segments.first(), flight.segments.last()) else {
        return String::new();
    };
    match last.arrival.date().days() - first.departure.date().days() {
        0 => String::new(),
        n => format!("{n:+}"),
    }
}

fn cheaper_date_line(hint: &DatePrice, currency: &str) -> String {
    let returning = hint
        .return_date
        .as_ref()
        .map(|r| format!(" returning {}", with_weekday(r)))
        .unwrap_or_default();
    format!(
        "Cheaper on {}{returning}: {}",
        with_weekday(&hint.date),
        format_price(Some(hint.price), currency)
    )
}

/// One table cell with a line per segment, so it lines up with the route column.
fn per_segment(flight: &FlightResult, cell: impl Fn(&Segment) -> String) -> String {
    flight.segments.iter().map(cell).collect::<Vec<_>>().join("\n")
}

fn price_header(round_trip: bool) -> &'static str {
    if round_trip { "Price (round trip)" } else { "Price" }
}

pub fn render(result: &SearchResult, currency: &str, round_trip: bool) -> String {
    if result.flights.is_empty() {
        return with_hint("No flights found.".to_string(), result, currency);
    }

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            "Airlines", "Route", "Flight", "Depart", "Arrive", "Duration", "Stops", "Aircraft",
            price_header(round_trip),
        ]);

    for flight in &result.flights {
        let route =
            per_segment(flight, |s| format!("{} → {}", s.from_airport.code, s.to_airport.code));
        let flight_numbers =
            per_segment(flight, |s| s.flight_number.clone().unwrap_or_else(|| "—".into()));
        let aircraft = per_segment(flight, |s| s.aircraft.clone().unwrap_or_default());

        let depart = flight
            .segments
            .first()
            .map(|s| s.departure.to_string())
            .unwrap_or_else(|| "—".to_string());

        let arrive = flight
            .segments
            .last()
            .map(|s| s.arrival.to_string())
            .unwrap_or_else(|| "—".to_string());

        let stops = if flight.layovers.is_empty() {
            "Nonstop".to_string()
        } else {
            flight.layovers.iter().map(describe_layover).collect::<Vec<_>>().join("\n")
        };

        table.add_row(vec![
            flight.airlines.join(", "),
            route,
            flight_numbers,
            depart,
            arrive,
            format_duration(flight.duration_minutes),
            stops,
            aircraft,
            format_price(flight.price, currency),
        ]);
    }

    with_hint(table.to_string(), result, currency)
}

fn with_hint(mut text: String, result: &SearchResult, currency: &str) -> String {
    if let Some(hint) = &result.cheaper_date {
        text.push('\n');
        text.push_str(&cheaper_date_line(hint, currency));
    }
    text
}

/// One line per flight:
/// `price | route | duration | stops | airlines | date depart>arrive | flight numbers`.
pub fn compact(result: &SearchResult, currency: &str, round_trip: bool) -> String {
    if result.flights.is_empty() {
        return with_hint("No flights found.".to_string(), result, currency);
    }

    let lines: Vec<String> = result
        .flights
        .iter()
        .map(|flight| compact_line(flight, currency, round_trip))
        .collect();
    with_hint(lines.join("\n"), result, currency)
}

fn compact_line(flight: &FlightResult, currency: &str, round_trip: bool) -> String {
    let mut price = format_price(flight.price, currency);
    if round_trip {
        price.push_str(" round trip");
    }

    let route: Vec<&str> = std::iter::once(
        flight
            .segments
            .first()
            .map(|s| s.from_airport.code.as_str())
            .unwrap_or("?"),
    )
    .chain(flight.segments.iter().map(|s| s.to_airport.code.as_str()))
    .collect();

    let stops = match flight.layovers.len() {
        0 => "nonstop".to_string(),
        n => format!(
            "{n} stop{} {}",
            if n == 1 { "" } else { "s" },
            flight.layovers.iter().map(describe_layover).collect::<Vec<_>>().join(", ")
        ),
    };

    let times = match (flight.segments.first(), flight.segments.last()) {
        (Some(d), Some(a)) => format!(
            "{}{:02} {:02}:{:02}>{:02}:{:02}{}",
            month_abbr(d.departure.month),
            d.departure.day,
            d.departure.hour,
            d.departure.minute,
            a.arrival.hour,
            a.arrival.minute,
            day_offset(flight),
        ),
        _ => "—".to_string(),
    };

    let flight_numbers: Vec<&str> = flight
        .segments
        .iter()
        .filter_map(|s| s.flight_number.as_deref())
        .collect();

    format!(
        "{price} | {} | {} | {stops} | {} | {times} | {}",
        route.join(">"),
        format_duration(flight.duration_minutes),
        flight.airlines.join(", "),
        if flight_numbers.is_empty() { "—".to_string() } else { flight_numbers.join(",") },
    )
}

fn month_abbr(m: u32) -> &'static str {
    match m {
        1 => "Jan",
        2 => "Feb",
        3 => "Mar",
        4 => "Apr",
        5 => "May",
        6 => "Jun",
        7 => "Jul",
        8 => "Aug",
        9 => "Sep",
        10 => "Oct",
        11 => "Nov",
        12 => "Dec",
        _ => "???",
    }
}

/// Renders multi-destination outcomes as `=== DEST ===` sections, with failures inline.
pub fn sections(
    outcomes: &[(String, Result<SearchResult, FlightError>)],
    separator: &str,
    render: impl Fn(&SearchResult) -> String,
) -> String {
    outcomes
        .iter()
        .map(|(label, outcome)| {
            let body = match outcome {
                Ok(result) => render(result),
                Err(e) => format!("error: {e}"),
            };
            format!("=== {label} ===\n{body}")
        })
        .collect::<Vec<_>>()
        .join(separator)
}

pub fn render_dates(dates: &[DatePrice], currency: &str) -> String {
    if dates.is_empty() {
        return "No prices found for these dates.".to_string();
    }

    let round_trip = dates.iter().any(|d| d.return_date.is_some());
    let mut header = vec!["Depart"];
    if round_trip {
        header.push("Return");
    }
    header.push(price_header(round_trip));

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(header);

    for date in dates {
        let mut row = vec![with_weekday(&date.date)];
        if round_trip {
            row.push(date.return_date.as_deref().map(with_weekday).unwrap_or_default());
        }
        row.push(format_price(Some(date.price), currency));
        table.add_row(row);
    }

    table.to_string()
}

/// One line per date: `2026-11-03 Tue | $354`, or `depart > return | price` for round trips.
pub fn compact_dates(dates: &[DatePrice], currency: &str) -> String {
    if dates.is_empty() {
        return "No prices found for these dates.".to_string();
    }

    dates
        .iter()
        .map(|d| {
            let returning = d
                .return_date
                .as_deref()
                .map(|r| format!(" > {}", with_weekday(r)))
                .unwrap_or_default();
            let mut price = format_price(Some(d.price), currency);
            if d.return_date.is_some() {
                price.push_str(" round trip");
            }
            format!("{}{returning} | {price}", with_weekday(&d.date))
        })
        .collect::<Vec<_>>()
        .join("\n")
}
