use flyr::model::{DatePrice, SearchResult};
use flyr::parse::{extract_script, parse_calendar, parse_html, parse_js, parse_payload};
use serde_json::json;

#[test]
fn extract_script_finds_ds1() {
    let html = r#"
    <html><head>
    <script class="ds:0">var x = 1;</script>
    <script class="ds:1">data:[1,2,3],sideChannel</script>
    <script class="ds:2">var z = 3;</script>
    </head></html>
    "#;
    let result = extract_script(html).unwrap();
    assert!(result.contains("data:"));
}

#[test]
fn extract_script_missing_ds1() {
    let html = r#"<html><head><script class="ds:0">x</script></head></html>"#;
    let result = extract_script(html);
    assert!(result.is_err());
}

#[test]
fn parse_js_splits_correctly() {
    let js = r#"some_func();data:[1,2,3],sideChannel"#;
    let result = parse_js(js).unwrap();
    assert_eq!(result, json!([1, 2, 3]));
}

#[test]
fn parse_js_no_data_marker() {
    let js = "no marker here";
    assert!(parse_js(js).is_err());
}

#[test]
fn parse_payload_null_flights() {
    let payload = json!([null, null, null, [null], null, null, null, [null, [[], []]]]);
    let result = parse_payload(&payload).unwrap();
    assert!(result.flights.is_empty());
}

#[test]
fn parse_payload_extracts_metadata() {
    let payload = json!([
        null, null, null, [null], null, null, null,
        [null, [
            [["*A", "Star Alliance"], ["OW", "oneworld"]],
            [["AY", "Finnair"], ["IB", "Iberia"]]
        ]]
    ]);
    let result = parse_payload(&payload).unwrap();
    assert_eq!(result.metadata.alliances.len(), 2);
    assert_eq!(result.metadata.alliances[0].code, "*A");
    assert_eq!(result.metadata.airlines.len(), 2);
    assert_eq!(result.metadata.airlines[0].code, "AY");
    assert_eq!(result.metadata.airlines[1].name, "Iberia");
}

fn make_segment() -> serde_json::Value {
    let mut seg = vec![serde_json::Value::Null; 22];
    seg[3] = json!("HEL");
    seg[4] = json!("Helsinki Airport");
    seg[5] = json!("Barcelona Airport");
    seg[6] = json!("BCN");
    seg[8] = json!([10, 30]);
    seg[10] = json!([14, 45]);
    seg[11] = json!(255);
    seg[17] = json!("Airbus A350");
    seg[20] = json!([2026, 3, 1]);
    seg[21] = json!([2026, 3, 1]);
    json!(seg)
}

fn make_flight_entry(segments: Vec<serde_json::Value>) -> serde_json::Value {
    let mut flight = vec![serde_json::Value::Null; 23];
    flight[0] = json!("Regular");
    flight[1] = json!(["AY"]);
    flight[2] = json!(segments);

    let mut extras = vec![serde_json::Value::Null; 9];
    extras[7] = json!(145000);
    extras[8] = json!(180000);
    flight[22] = json!(extras);

    let price = json!([[null, 299]]);
    json!([flight, price])
}

#[test]
fn parse_payload_extracts_flights() {
    let seg = make_segment();
    let entry = make_flight_entry(vec![seg]);

    let payload = json!([
        null, null, null, [[entry]], null, null, null,
        [null, [[], []]]
    ]);

    let result = parse_payload(&payload).unwrap();
    assert_eq!(result.flights.len(), 1);

    let f = &result.flights[0];
    assert_eq!(f.flight_type, "Regular");
    assert_eq!(f.airlines, vec!["AY"]);
    assert_eq!(f.price, Some(299));
    assert_eq!(f.segments.len(), 1);

    let s = &f.segments[0];
    assert_eq!(s.from_airport.code, "HEL");
    assert_eq!(s.from_airport.name, "Helsinki Airport");
    assert_eq!(s.to_airport.code, "BCN");
    assert_eq!(s.to_airport.name, "Barcelona Airport");
    assert_eq!(s.departure.hour, 10);
    assert_eq!(s.departure.minute, 30);
    assert_eq!(s.arrival.hour, 14);
    assert_eq!(s.arrival.minute, 45);
    assert_eq!(s.duration_minutes, 255);
    assert_eq!(s.aircraft.as_deref(), Some("Airbus A350"));
    assert_eq!(s.departure.year, 2026);
}

#[test]
fn parse_payload_extracts_carbon() {
    let seg = make_segment();
    let entry = make_flight_entry(vec![seg]);

    let payload = json!([
        null, null, null, [[entry]], null, null, null,
        [null, [[], []]]
    ]);

    let result = parse_payload(&payload).unwrap();
    let f = &result.flights[0];
    assert_eq!(f.carbon.emission_grams, Some(145000));
    assert_eq!(f.carbon.typical_grams, Some(180000));
}

#[test]
fn parse_payload_multi_segment() {
    let seg1 = make_segment();
    let mut seg2_vec = vec![serde_json::Value::Null; 22];
    seg2_vec[3] = json!("CDG");
    seg2_vec[4] = json!("Paris CDG");
    seg2_vec[5] = json!("Barcelona Airport");
    seg2_vec[6] = json!("BCN");
    seg2_vec[8] = json!([16, 0]);
    seg2_vec[10] = json!([18, 30]);
    seg2_vec[11] = json!(150);
    seg2_vec[17] = json!("Boeing 737");
    seg2_vec[20] = json!([2026, 3, 1]);
    seg2_vec[21] = json!([2026, 3, 1]);
    let seg2 = json!(seg2_vec);

    let entry = make_flight_entry(vec![seg1, seg2]);
    let payload = json!([
        null, null, null, [[entry]], null, null, null,
        [null, [[], []]]
    ]);

    let result = parse_payload(&payload).unwrap();
    assert_eq!(result.flights[0].segments.len(), 2);
    assert_eq!(result.flights[0].segments[1].from_airport.code, "CDG");
}

#[test]
fn parse_html_integration() {
    let html = r#"
    <html><head>
    <script class="ds:1">AF_initDataCallback({data:[
        null, null, null,
        [null],
        null, null, null,
        [null, [[], []]]
    ],sideChannel: {}});</script>
    </head></html>
    "#;

    let result = parse_html(html).unwrap();
    assert!(result.flights.is_empty());
}

#[test]
fn parse_payload_missing_price() {
    let seg = make_segment();
    let mut flight = vec![serde_json::Value::Null; 23];
    flight[0] = json!("Regular");
    flight[1] = json!(["AY"]);
    flight[2] = json!([seg]);
    let mut extras = vec![serde_json::Value::Null; 9];
    extras[7] = json!(145000);
    extras[8] = json!(180000);
    flight[22] = json!(extras);

    let entry = json!([flight, [[ ]]]);
    let payload = json!([
        null, null, null, [[entry]], null, null, null,
        [null, [[], []]]
    ]);

    let result = parse_payload(&payload).unwrap();
    assert_eq!(result.flights[0].price, None);
}

#[test]
fn parse_segment_hour_only_time() {
    let mut seg = vec![serde_json::Value::Null; 22];
    seg[3] = json!("JFK");
    seg[4] = json!("JFK Airport");
    seg[5] = json!("ORD Airport");
    seg[6] = json!("ORD");
    seg[8] = json!([9]);
    seg[10] = json!([18]);
    seg[11] = json!(180);
    seg[17] = json!("Boeing 737");
    seg[20] = json!([2026, 4, 1]);
    seg[21] = json!([2026, 4, 1]);

    let entry = make_flight_entry(vec![json!(seg)]);
    let payload = json!([
        null, null, null, [[entry]], null, null, null,
        [null, [[], []]]
    ]);

    let result = parse_payload(&payload).unwrap();
    assert_eq!(result.flights.len(), 1);
    let s = &result.flights[0].segments[0];
    assert_eq!(s.departure.hour, 9);
    assert_eq!(s.departure.minute, 0);
    assert_eq!(s.arrival.hour, 18);
    assert_eq!(s.arrival.minute, 0);
}

#[test]
fn parse_segment_missing_airport_name() {
    let mut seg = vec![serde_json::Value::Null; 22];
    seg[3] = json!("JFK");
    seg[6] = json!("ORD");
    seg[8] = json!([10, 30]);
    seg[10] = json!([14, 0]);
    seg[11] = json!(210);
    seg[20] = json!([2026, 4, 1]);
    seg[21] = json!([2026, 4, 1]);

    let entry = make_flight_entry(vec![json!(seg)]);
    let payload = json!([
        null, null, null, [[entry]], null, null, null,
        [null, [[], []]]
    ]);

    let result = parse_payload(&payload).unwrap();
    assert_eq!(result.flights.len(), 1);
    assert_eq!(result.flights[0].segments.len(), 1);
    assert_eq!(result.flights[0].segments[0].from_airport.code, "JFK");
    assert_eq!(result.flights[0].segments[0].from_airport.name, "");
}

#[test]
fn parse_segment_null_hour_is_midnight() {
    let mut seg = vec![serde_json::Value::Null; 22];
    seg[3] = json!("HEL");
    seg[6] = json!("DXB");
    seg[8] = json!([15, 45]);
    seg[10] = json!([null, 20]);
    seg[11] = json!(395);
    seg[20] = json!([2026, 11, 2]);
    seg[21] = json!([2026, 11, 3]);

    let entry = make_flight_entry(vec![json!(seg)]);
    let payload = json!([null, null, null, [[entry]], null, null, null, [null, [[], []]]]);

    let result = parse_payload(&payload).unwrap();
    let s = &result.flights[0].segments[0];
    assert_eq!((s.arrival.hour, s.arrival.minute), (0, 20));
}

#[test]
fn layovers_fall_back_to_segment_times() {
    let mut first = make_segment();
    first[6] = json!("LGW");
    first[10] = json!([14, 45]);
    let mut second = make_segment();
    second[3] = json!("LHR");
    second[8] = json!([17, 15]);

    let entry = make_flight_entry(vec![first, second]);
    let payload = json!([null, null, null, [[entry]], null, null, null, [null, [[], []]]]);

    let flight = &parse_payload(&payload).unwrap().flights[0];
    assert_eq!(flight.layovers.len(), 1);
    let layover = &flight.layovers[0];
    assert_eq!(layover.airport, "LGW");
    assert_eq!(layover.duration_minutes, 150);
    assert!(layover.change_of_airport);
    assert!(!layover.overnight);
    assert_eq!(flight.duration_minutes, 255 + 150 + 255);
}

fn fixture(name: &str) -> SearchResult {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    let payload: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    parse_payload(&payload).unwrap()
}

fn find<'a>(result: &'a SearchResult, first_flight: &str) -> &'a flyr::model::FlightResult {
    result
        .flights
        .iter()
        .find(|f| f.segments[0].flight_number.as_deref() == Some(first_flight))
        .unwrap_or_else(|| panic!("no flight starting with {first_flight}"))
}

#[test]
fn real_page_includes_top_flights() {
    let mut result = fixture("lhr_jfk_2026-11-01.json");
    assert_eq!(result.flights.len(), 21);
    assert_eq!(result.flights[0].price, Some(450));

    result.keep_cheapest(1);
    let cheapest = &result.flights[0];
    assert_eq!(cheapest.price, Some(450));
    assert_eq!(cheapest.segments[0].flight_number.as_deref(), Some("B62220"));
}

#[test]
fn real_page_duration_includes_layovers() {
    let result = fixture("lhr_jfk_2026-11-01.json");
    let flight = find(&result, "FI455");

    let flying: u32 = flight.segments.iter().map(|s| s.duration_minutes).sum();
    assert_eq!(flying, 565);
    assert_eq!(flight.duration_minutes, 1585);
    assert_eq!(flight.layovers.len(), 1);
    assert_eq!(flight.layovers[0].airport, "KEF");
    assert_eq!(flight.layovers[0].duration_minutes, 1020);
    assert!(flight.layovers[0].overnight);
    assert!(!flight.layovers[0].change_of_airport);
    assert!(result.cheaper_date.is_none());
}

#[test]
fn real_page_keeps_segments_at_midnight() {
    let result = fixture("hel_bkk_2026-11-02.json");
    assert_eq!(result.flights.len(), 11);
    assert!(result.flights.iter().all(|f| !f.segments.is_empty()));

    let emirates = find(&result, "EK168");
    assert_eq!(emirates.segments.len(), 2);
    assert_eq!(emirates.segments[0].to_airport.code, "DXB");
    assert_eq!((emirates.segments[0].arrival.hour, emirates.segments[0].arrival.minute), (0, 20));

    let finnair = find(&result, "AY145");
    assert_eq!((finnair.segments[0].departure.hour, finnair.segments[0].departure.minute), (0, 15));
}

#[test]
fn real_page_three_segment_itinerary() {
    let result = fixture("hel_bkk_2026-11-02.json");
    let flight = find(&result, "SK1705");

    let numbers: Vec<_> = flight.segments.iter().filter_map(|s| s.flight_number.as_deref()).collect();
    assert_eq!(numbers, ["SK1705", "EY178", "EY406"]);
    assert_eq!(flight.segments[0].codeshares, ["EY3986"]);
    assert_eq!(flight.duration_minutes, 1830);

    let layovers: Vec<_> = flight
        .layovers
        .iter()
        .map(|l| (l.airport.as_str(), l.duration_minutes, l.overnight))
        .collect();
    assert_eq!(layovers, [("CPH", 190, false), ("AUH", 780, true)]);
}

#[test]
fn real_page_cheaper_date_hint() {
    let one_way = fixture("hel_bkk_2026-11-02.json");
    assert_eq!(
        one_way.cheaper_date,
        Some(DatePrice { date: "2026-11-03".into(), return_date: None, price: 354 })
    );

    let round_trip = fixture("hel_bcn_2026-11-02_2026-11-09.json");
    assert_eq!(round_trip.flights.len(), 9);
    assert_eq!(round_trip.flights.iter().filter_map(|f| f.price).min(), Some(297));
    assert_eq!(
        round_trip.cheaper_date,
        Some(DatePrice {
            date: "2026-11-03".into(),
            return_date: Some("2026-11-10".into()),
            price: 215,
        })
    );
}

fn calendar_fixture(name: &str) -> Result<Vec<DatePrice>, flyr::error::FlightError> {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    parse_calendar(&std::fs::read_to_string(path).unwrap())
}

#[test]
fn calendar_one_way() {
    let dates = calendar_fixture("calendar_hel_bkk_oneway.txt").unwrap();
    assert_eq!(dates.len(), 6);
    assert_eq!(dates[0].date, "2026-11-01");
    assert_eq!(dates[2], DatePrice { date: "2026-11-03".into(), return_date: None, price: 354 });
}

#[test]
fn calendar_round_trip() {
    let dates = calendar_fixture("calendar_hel_bcn_roundtrip.txt").unwrap();
    assert_eq!(dates.len(), 30);
    assert_eq!(
        dates[0],
        DatePrice { date: "2026-11-01".into(), return_date: Some("2026-11-08".into()), price: 314 }
    );
    assert!(dates.windows(2).all(|w| w[0].date < w[1].date));
}

#[test]
fn calendar_rejection_carries_code() {
    let err = calendar_fixture("calendar_rejected.txt").unwrap_err();
    assert!(matches!(
        err,
        flyr::error::FlightError::Rejected(Some(flyr::error::INVALID_ARGUMENT))
    ));
}

#[test]
fn calendar_without_envelope_is_parse_error() {
    let err = parse_calendar(")]}'\n\n[[\"di\",31]]").unwrap_err();
    assert!(matches!(err, flyr::error::FlightError::JsParse(_)));
}

#[test]
fn real_page_overnight_means_crossing_midnight() {
    let result = fixture("hel_bkk_2026-11-02.json");

    let qatar = find(&result, "QR302");
    assert_eq!(qatar.layovers[0].airport, "DOH");
    assert!(qatar.layovers[0].overnight);

    let turkish = find(&result, "TK1762");
    assert_eq!(turkish.layovers[0].airport, "IST");
    assert!(!turkish.layovers[0].overnight);
}
