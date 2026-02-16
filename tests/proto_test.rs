use base64::Engine;
use base64::engine::general_purpose::STANDARD;

use flyr::proto;
use flyr::query::{FlightLeg, Passengers, Seat, TripType};

fn encode_b64(
    legs: &[FlightLeg],
    passengers: &Passengers,
    seat: &Seat,
    trip: &TripType,
) -> String {
    STANDARD.encode(proto::encode(legs, passengers, seat, trip))
}

#[test]
fn basic_one_way_economy() {
    let legs = vec![FlightLeg {
        date: "2026-03-01".into(),
        from_airport: "LAX".into(),
        to_airport: "NRT".into(),
        max_stops: None,
        airlines: None,
    }];
    let pax = Passengers {
        adults: 1,
        children: 0,
        infants_in_seat: 0,
        infants_on_lap: 0,
    };

    let result = encode_b64(&legs, &pax, &Seat::Economy, &TripType::OneWay);
    assert_eq!(
        result,
        "GhoSCjIwMjYtMDMtMDFqBRIDTEFYcgUSA05SVEIBAUgBmAEC"
    );
}

#[test]
fn round_trip_with_return_leg() {
    let legs = vec![
        FlightLeg {
            date: "2026-03-01".into(),
            from_airport: "LAX".into(),
            to_airport: "NRT".into(),
            max_stops: None,
            airlines: None,
        },
        FlightLeg {
            date: "2026-03-10".into(),
            from_airport: "NRT".into(),
            to_airport: "LAX".into(),
            max_stops: None,
            airlines: None,
        },
    ];
    let pax = Passengers {
        adults: 1,
        children: 0,
        infants_in_seat: 0,
        infants_on_lap: 0,
    };

    let result = encode_b64(&legs, &pax, &Seat::Economy, &TripType::RoundTrip);
    assert_eq!(
        result,
        "GhoSCjIwMjYtMDMtMDFqBRIDTEFYcgUSA05SVBoaEgoyMDI2LTAzLTEwagUSA05SVHIFEgNMQVhCAQFIAZgBAQ=="
    );
}

#[test]
fn multiple_passengers() {
    let legs = vec![FlightLeg {
        date: "2026-03-01".into(),
        from_airport: "HEL".into(),
        to_airport: "BCN".into(),
        max_stops: None,
        airlines: None,
    }];
    let pax = Passengers {
        adults: 2,
        children: 1,
        infants_in_seat: 1,
        infants_on_lap: 0,
    };

    let result = encode_b64(&legs, &pax, &Seat::Economy, &TripType::OneWay);
    assert_eq!(
        result,
        "GhoSCjIwMjYtMDMtMDFqBRIDSEVMcgUSA0JDTkIEAQECA0gBmAEC"
    );
}

#[test]
fn with_max_stops() {
    let legs = vec![FlightLeg {
        date: "2026-03-01".into(),
        from_airport: "HEL".into(),
        to_airport: "BKK".into(),
        max_stops: Some(1),
        airlines: None,
    }];
    let pax = Passengers {
        adults: 1,
        children: 0,
        infants_in_seat: 0,
        infants_on_lap: 0,
    };

    let result = encode_b64(&legs, &pax, &Seat::Business, &TripType::OneWay);
    assert_eq!(
        result,
        "GhwSCjIwMjYtMDMtMDEoAWoFEgNIRUxyBRIDQktLQgEBSAOYAQI="
    );
}

#[test]
fn with_airline_filter() {
    let legs = vec![FlightLeg {
        date: "2026-03-01".into(),
        from_airport: "HEL".into(),
        to_airport: "BCN".into(),
        max_stops: None,
        airlines: Some(vec!["AY".into(), "IB".into()]),
    }];
    let pax = Passengers {
        adults: 1,
        children: 0,
        infants_in_seat: 0,
        infants_on_lap: 0,
    };

    let result = encode_b64(&legs, &pax, &Seat::Economy, &TripType::OneWay);
    assert_eq!(
        result,
        "GiISCjIwMjYtMDMtMDEyAkFZMgJJQmoFEgNIRUxyBRIDQkNOQgEBSAGYAQI="
    );
}

#[test]
fn multi_city_three_legs() {
    let legs = vec![
        FlightLeg {
            date: "2026-03-01".into(),
            from_airport: "LAX".into(),
            to_airport: "NRT".into(),
            max_stops: None,
            airlines: None,
        },
        FlightLeg {
            date: "2026-03-05".into(),
            from_airport: "NRT".into(),
            to_airport: "ICN".into(),
            max_stops: None,
            airlines: None,
        },
        FlightLeg {
            date: "2026-03-10".into(),
            from_airport: "ICN".into(),
            to_airport: "LAX".into(),
            max_stops: None,
            airlines: None,
        },
    ];
    let pax = Passengers {
        adults: 2,
        children: 0,
        infants_in_seat: 0,
        infants_on_lap: 0,
    };

    let result = encode_b64(&legs, &pax, &Seat::PremiumEconomy, &TripType::MultiCity);
    assert_eq!(
        result,
        "GhoSCjIwMjYtMDMtMDFqBRIDTEFYcgUSA05SVBoaEgoyMDI2LTAzLTA1agUSA05SVHIFEgNJQ04aGhIKMjAyNi0wMy0xMGoFEgNJQ05yBRIDTEFYQgIBAUgCmAED"
    );
}
