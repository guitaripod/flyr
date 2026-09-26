use flyr::error::FlightError;
use flyr::model::{DatePrice, SearchResult};
use flyr::parse::parse_payload;
use flyr::table::{compact, compact_dates, render, render_dates, sections};

fn fixture(name: &str) -> SearchResult {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    let payload: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    parse_payload(&payload).unwrap()
}

#[test]
fn compact_shows_layovers_and_next_day_arrival() {
    let text = compact(&fixture("lhr_jfk_2026-11-01.json"), "USD", false);
    assert_eq!(text.lines().count(), 21);
    assert_eq!(
        text.lines().next().unwrap(),
        "$450 | LHR>JFK | 8h15m | nonstop | JetBlue | Nov01 07:45>11:00 | B62220"
    );
    assert!(text.lines().any(|l| l
        == "$664 | LHR>KEF>JFK | 26h25m | 1 stop KEF 17h00m overnight | Icelandair \
            | Nov01 20:35>18:00+1 | FI455,FI615"));
}

#[test]
fn compact_lists_every_stop() {
    let text = compact(&fixture("hel_bkk_2026-11-02.json"), "USD", false);
    assert!(text.lines().any(|l| l.starts_with(
        "$507 | HEL>CPH>AUH>BKK | 30h30m | 2 stops CPH 3h10m, AUH 13h00m overnight |"
    )));
    assert_eq!(text.lines().last().unwrap(), "Cheaper on 2026-11-03 Tue: $354");
}

#[test]
fn compact_marks_round_trip_prices() {
    let text = compact(&fixture("hel_bcn_2026-11-02_2026-11-09.json"), "EUR", true);
    assert!(text
        .lines()
        .next()
        .unwrap()
        .starts_with("€297 round trip | HEL>CDG>BCN | 6h20m | 1 stop CDG 1h00m | Air France |"));
    assert_eq!(
        text.lines().last().unwrap(),
        "Cheaper on 2026-11-03 Tue returning 2026-11-10 Tue: €215"
    );
}

#[test]
fn table_labels_round_trip_prices_and_flight_numbers() {
    let text = render(&fixture("hel_bcn_2026-11-02_2026-11-09.json"), "USD", true);
    assert!(text.contains("Price (round trip)"));
    assert!(text.contains("AF1071"));
    assert!(text.contains("CDG 1h00m"));
}

#[test]
fn empty_results_say_so() {
    assert_eq!(compact(&SearchResult::default(), "USD", false), "No flights found.");
    assert_eq!(render(&SearchResult::default(), "USD", false), "No flights found.");
    assert_eq!(compact_dates(&[], "USD"), "No prices found for these dates.");
}

#[test]
fn sections_show_failures_inline() {
    let outcomes = vec![
        ("BCN".to_string(), Ok(fixture("hel_bcn_2026-11-02_2026-11-09.json"))),
        ("ATH".to_string(), Err(FlightError::RateLimited)),
    ];
    let text = sections(&outcomes, "\n", |r| compact(r, "USD", true));
    assert!(text.starts_with("=== BCN ===\n$297 round trip |"));
    assert!(text.contains("\n=== ATH ===\nerror: rate limited by Google"));
}

#[test]
fn dates_render_with_weekdays() {
    let one_way = [DatePrice { date: "2026-11-03".into(), return_date: None, price: 354 }];
    assert_eq!(compact_dates(&one_way, "USD"), "2026-11-03 Tue | $354");

    let round_trip = [DatePrice {
        date: "2026-11-03".into(),
        return_date: Some("2026-11-10".into()),
        price: 215,
    }];
    assert_eq!(
        compact_dates(&round_trip, "USD"),
        "2026-11-03 Tue > 2026-11-10 Tue | $215 round trip"
    );

    let table = render_dates(&round_trip, "USD");
    assert!(table.contains("Return"));
    assert!(table.contains("Price (round trip)"));
    assert!(table.contains("2026-11-10 Tue"));
}
