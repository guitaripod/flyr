use comfy_table::{Table, ContentArrangement, presets::UTF8_FULL};

use crate::model::SearchResult;

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

pub fn render(result: &SearchResult, currency: &str) -> String {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            "Airlines", "Route", "Depart", "Arrive", "Duration", "Stops", "Aircraft", "Price",
        ]);

    for flight in &result.flights {
        let airlines = flight.airlines.join(", ");

        let route: Vec<String> = flight
            .segments
            .iter()
            .map(|s| format!("{} → {}", s.from_airport.code, s.to_airport.code))
            .collect();
        let route_str = route.join("\n");

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

        let duration = if flight.segments.is_empty() {
            "—".to_string()
        } else {
            let total_duration: u32 = flight.segments.iter().map(|s| s.duration_minutes).sum();
            let hours = total_duration / 60;
            let mins = total_duration % 60;
            format!("{hours}h {mins:02}m")
        };

        let stops = if flight.segments.is_empty() {
            "—".to_string()
        } else if flight.segments.len() == 1 {
            "Nonstop".to_string()
        } else {
            let n = flight.segments.len() - 1;
            let stopovers: Vec<&str> = flight.segments[..flight.segments.len() - 1]
                .iter()
                .map(|s| s.to_airport.code.as_str())
                .collect();
            format!("{n} ({})", stopovers.join(", "))
        };

        let aircraft: Vec<String> = flight
            .segments
            .iter()
            .filter_map(|s| s.aircraft.clone())
            .collect();
        let aircraft_str = aircraft.join(", ");

        let price = format_price(flight.price, currency);

        table.add_row(vec![
            &airlines,
            &route_str,
            &depart,
            &arrive,
            &duration,
            &stops,
            &aircraft_str,
            &price,
        ]);
    }

    table.to_string()
}
