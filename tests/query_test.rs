use flyr::query::{FlightLeg, Passengers, QueryParams, Seat, TripType};

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
