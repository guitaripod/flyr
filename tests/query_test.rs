use flyr::date::Date;
use flyr::query::{
    parse_codes, parse_date_range, route_queries, to_google_flights_url, DateQuery, Filters,
    FlightLeg, Passengers, QueryParams, Seat, TripType,
};

fn make_valid_query() -> QueryParams {
    QueryParams {
        legs: vec![FlightLeg {
            date: "2026-03-01".into(),
            from_airport: "HEL".into(),
            to_airport: "BCN".into(),
            max_stops: None,
            airlines: None,
        }],
        passengers: Passengers::default(),
        seat: Seat::Economy,
        trip: TripType::OneWay,
        language: "en".into(),
        currency: "USD".into(),
    }
}

#[test]
fn valid_query_passes() {
    let q = make_valid_query();
    assert!(q.validate().is_ok());
}

#[test]
fn rejects_lowercase_airport() {
    let mut q = make_valid_query();
    q.legs[0].from_airport = "hel".into();
    assert!(q.validate().is_err());
}

#[test]
fn rejects_too_short_airport() {
    let mut q = make_valid_query();
    q.legs[0].from_airport = "HE".into();
    assert!(q.validate().is_err());
}

#[test]
fn rejects_too_long_airport() {
    let mut q = make_valid_query();
    q.legs[0].from_airport = "HELX".into();
    assert!(q.validate().is_err());
}

#[test]
fn rejects_numeric_airport() {
    let mut q = make_valid_query();
    q.legs[0].from_airport = "H3L".into();
    assert!(q.validate().is_err());
}

#[test]
fn rejects_invalid_date_format() {
    let mut q = make_valid_query();
    q.legs[0].date = "03-01-2026".into();
    assert!(q.validate().is_err());
}

#[test]
fn rejects_invalid_month() {
    let mut q = make_valid_query();
    q.legs[0].date = "2026-13-01".into();
    assert!(q.validate().is_err());
}

#[test]
fn rejects_too_many_passengers() {
    let mut q = make_valid_query();
    q.passengers = Passengers {
        adults: 5,
        children: 3,
        infants_in_seat: 2,
        infants_on_lap: 0,
    };
    assert!(q.validate().is_err());
}

#[test]
fn rejects_zero_passengers() {
    let mut q = make_valid_query();
    q.passengers = Passengers {
        adults: 0,
        children: 0,
        infants_in_seat: 0,
        infants_on_lap: 0,
    };
    assert!(q.validate().is_err());
}

#[test]
fn rejects_infants_exceeding_adults() {
    let mut q = make_valid_query();
    q.passengers = Passengers {
        adults: 1,
        children: 0,
        infants_in_seat: 0,
        infants_on_lap: 2,
    };
    assert!(q.validate().is_err());
}

#[test]
fn accepts_nine_passengers() {
    let mut q = make_valid_query();
    q.passengers = Passengers {
        adults: 5,
        children: 2,
        infants_in_seat: 1,
        infants_on_lap: 1,
    };
    assert!(q.validate().is_ok());
}

#[test]
fn accepts_zero_stops() {
    let mut q = make_valid_query();
    q.legs[0].max_stops = Some(0);
    assert!(q.validate().is_ok());
}

#[test]
fn rejects_empty_legs() {
    let mut q = make_valid_query();
    q.legs.clear();
    assert!(q.validate().is_err());
}

#[test]
fn url_params_contain_tfs() {
    let q = make_valid_query();
    let params = q.to_url_params();
    assert!(params.iter().any(|(k, _)| k == "tfs"));
    assert!(params.iter().any(|(k, v)| k == "hl" && v == "en"));
    assert!(params.iter().any(|(k, v)| k == "curr" && v == "USD"));
}

#[test]
fn rejects_feb_30() {
    let mut q = make_valid_query();
    q.legs[0].date = "2026-02-30".into();
    assert!(q.validate().is_err());
}

#[test]
fn rejects_apr_31() {
    let mut q = make_valid_query();
    q.legs[0].date = "2026-04-31".into();
    assert!(q.validate().is_err());
}

#[test]
fn accepts_feb_28_non_leap() {
    let mut q = make_valid_query();
    q.legs[0].date = "2025-02-28".into();
    assert!(q.validate().is_ok());
}

#[test]
fn rejects_feb_29_non_leap() {
    let mut q = make_valid_query();
    q.legs[0].date = "2025-02-29".into();
    assert!(q.validate().is_err());
}

#[test]
fn accepts_feb_29_leap() {
    let mut q = make_valid_query();
    q.legs[0].date = "2028-02-29".into();
    assert!(q.validate().is_ok());
}

#[test]
fn empty_lang_omitted_from_params() {
    let mut q = make_valid_query();
    q.language = "".into();
    let params = q.to_url_params();
    assert!(!params.iter().any(|(k, _)| k == "hl"));
}

#[test]
fn browser_url_uses_tfs_path() {
    let q = make_valid_query();
    let url = to_google_flights_url(&q);
    assert!(url.starts_with("https://www.google.com/travel/flights/search?tfs="));
}

#[test]
fn browser_url_contains_tfu() {
    let q = make_valid_query();
    let url = to_google_flights_url(&q);
    assert!(url.contains("&tfu=EgYIABAAGAA"));
}

#[test]
fn browser_url_tfs_is_url_safe_base64() {
    let q = make_valid_query();
    let url = to_google_flights_url(&q);
    let tfs_start = url.find("tfs=").unwrap() + 4;
    let tfs_end = url[tfs_start..].find('&').unwrap() + tfs_start;
    let tfs_value = &url[tfs_start..tfs_end];
    assert!(!tfs_value.contains('+'), "tfs contains '+' (not URL-safe)");
    assert!(!tfs_value.contains('/'), "tfs contains '/' (not URL-safe)");
    assert!(!tfs_value.contains('='), "tfs contains '=' (has padding)");
}

#[test]
fn date_round_trips_through_days() {
    assert_eq!(Date { year: 1970, month: 1, day: 1 }.days(), 0);
    for s in ["2000-01-01", "2024-02-29", "2026-11-03", "2099-12-31"] {
        let date = Date::parse(s).unwrap();
        assert_eq!(Date::from_days(date.days()), date);
        assert_eq!(date.to_string(), s);
    }
}

#[test]
fn date_weekdays() {
    let weekday = |s| Date::parse(s).unwrap().weekday();
    assert_eq!(weekday("2026-09-26"), "Sat");
    assert_eq!(weekday("2026-11-03"), "Tue");
    assert_eq!(weekday("2000-01-01"), "Sat");
    assert_eq!(weekday("2028-02-29"), "Tue");
}

#[test]
fn date_add_days_crosses_months_and_leap_days() {
    let add = |s, n| Date::parse(s).unwrap().add_days(n).to_string();
    assert_eq!(add("2028-02-28", 1), "2028-02-29");
    assert_eq!(add("2026-12-28", 7), "2027-01-04");
    assert_eq!(add("2026-03-01", -1), "2026-02-28");
}

#[test]
fn date_parse_is_strict() {
    for s in [
        "2026-3-1", "03-01-2026", "2026-02-29", "2026-13-01", "1999-12-31", "2026-+1-01",
        "2026-01-01-01", "",
    ] {
        assert!(Date::parse(s).is_none(), "{s} should not parse");
    }
}

#[test]
fn date_range_month() {
    let (start, end) = parse_date_range("2028-02").unwrap();
    assert_eq!(start.to_string(), "2028-02-01");
    assert_eq!(end.to_string(), "2028-02-29");
}

#[test]
fn date_range_explicit_and_single_day() {
    let (start, end) = parse_date_range("2026-03-01..2026-04-15").unwrap();
    assert_eq!((start.to_string(), end.to_string()), ("2026-03-01".into(), "2026-04-15".into()));

    let (start, end) = parse_date_range("2026-03-05").unwrap();
    assert_eq!(start, end);

    for bad in ["2026-04-01..2026-03-01", "2026-3", "march", "2026-03-01..", "2026-13"] {
        assert!(parse_date_range(bad).is_err(), "{bad} should not parse");
    }
}

#[test]
fn parse_codes_trims_uppercases_and_dedupes() {
    assert_eq!(parse_codes(" bcn,ATH,,bcn "), ["BCN", "ATH"]);
    assert!(parse_codes(" , ").is_empty());
}

#[test]
fn route_builds_round_trip() {
    let q = QueryParams::route("HEL", "BCN", "2026-03-01", Some("2026-03-08"), &Filters::default());
    assert!(q.validate().is_ok());
    assert_eq!(q.trip, TripType::RoundTrip);
    assert_eq!(q.legs.len(), 2);
    assert_eq!((q.legs[1].from_airport.as_str(), q.legs[1].to_airport.as_str()), ("BCN", "HEL"));
}

#[test]
fn route_queries_one_per_destination() {
    let filters = Filters { max_stops: Some(0), ..Filters::default() };
    let queries = route_queries("HEL", &["BCN".into(), "ATH".into()], "2026-03-01", None, &filters);
    let labels: Vec<_> = queries.iter().map(|(label, _)| label.as_str()).collect();
    assert_eq!(labels, ["BCN", "ATH"]);
    assert!(queries.iter().all(|(_, q)| q.trip == TripType::OneWay && q.legs[0].max_stops == Some(0)));
}

#[test]
fn round_trip_needs_a_return_leg() {
    let mut q = make_valid_query();
    q.trip = TripType::RoundTrip;
    assert!(q.validate().is_err());
}

#[test]
fn multi_city_needs_two_legs() {
    let mut q = make_valid_query();
    q.trip = TripType::MultiCity;
    assert!(q.validate().is_err());
}

#[test]
fn legs_must_be_in_date_order() {
    let q = QueryParams::route("HEL", "BCN", "2026-03-08", Some("2026-03-01"), &Filters::default());
    assert!(q.validate().is_err());
}

fn date_query(stay_days: Option<u32>, filters: Filters) -> DateQuery {
    DateQuery {
        from_airport: "HEL".into(),
        to_airport: "BCN".into(),
        start: Date::parse("2099-03-01").unwrap(),
        end: Date::parse("2099-03-31").unwrap(),
        stay_days,
        filters,
    }
}

fn decode_calendar_filters(body: &str) -> serde_json::Value {
    let encoded = body.strip_prefix("f.req=").expect("form body");
    let outer: serde_json::Value =
        serde_json::from_str(&urlencoding::decode(encoded).unwrap()).unwrap();
    serde_json::from_str(outer[1].as_str().unwrap()).unwrap()
}

#[test]
fn date_query_body_one_way() {
    let filters = decode_calendar_filters(&date_query(None, Filters::default()).to_request_body());
    assert_eq!(filters[1][2], 2);
    assert_eq!(filters[1][5], 1);
    assert_eq!(filters[1][6], serde_json::json!([1, 0, 0, 0]));
    assert_eq!(filters[1][13].as_array().unwrap().len(), 1);
    assert_eq!(filters[1][13][0][0], serde_json::json!([[["HEL", 0]]]));
    assert_eq!(filters[1][13][0][3], 0);
    assert_eq!(filters[2], serde_json::json!(["2099-03-01", "2099-03-31"]));
    assert_eq!(filters.as_array().unwrap().len(), 3);
}

#[test]
fn date_query_body_round_trip_with_filters() {
    let filters = Filters {
        max_stops: Some(0),
        airlines: Some(vec!["AY".into()]),
        seat: Seat::Business,
        ..Filters::default()
    };
    let body = decode_calendar_filters(&date_query(Some(7), filters).to_request_body());
    assert_eq!(body[1][2], 1);
    assert_eq!(body[1][5], 3);
    let legs = body[1][13].as_array().unwrap();
    assert_eq!(legs.len(), 2);
    assert_eq!(legs[0][3], 1);
    assert_eq!(legs[0][4], serde_json::json!(["AY"]));
    assert_eq!(legs[1][0], serde_json::json!([[["BCN", 0]]]));
    assert_eq!(legs[1][6], "2099-03-08");
    assert_eq!(body[4], serde_json::json!([7, 7]));
}

#[test]
fn date_query_validation() {
    assert!(date_query(None, Filters::default()).validate().is_ok());

    let mut past = date_query(None, Filters::default());
    past.start = Date::parse("2020-01-01").unwrap();
    past.end = Date::parse("2020-01-31").unwrap();
    assert!(past.validate().is_err());

    let mut city_list = date_query(None, Filters::default());
    city_list.to_airport = "BCN,ATH".into();
    assert!(city_list.validate().is_err());
}
