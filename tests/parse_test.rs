use flyr::parse::{extract_script, parse_html, parse_js, parse_payload};
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
