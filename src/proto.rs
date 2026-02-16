use crate::query::{FlightLeg, Passengers, Seat, TripType};

fn encode_varint(mut value: u64, buf: &mut Vec<u8>) {
    loop {
        let byte = (value & 0x7F) as u8;
        value >>= 7;
        if value == 0 {
            buf.push(byte);
            break;
        }
        buf.push(byte | 0x80);
    }
}

fn encode_tag(field: u32, wire_type: u8, buf: &mut Vec<u8>) {
    encode_varint(((field as u64) << 3) | wire_type as u64, buf);
}

fn encode_string(field: u32, s: &str, buf: &mut Vec<u8>) {
    encode_tag(field, 2, buf);
    encode_varint(s.len() as u64, buf);
    buf.extend_from_slice(s.as_bytes());
}

fn encode_submessage(field: u32, inner: &[u8], buf: &mut Vec<u8>) {
    encode_tag(field, 2, buf);
    encode_varint(inner.len() as u64, buf);
    buf.extend_from_slice(inner);
}

fn encode_airport(code: &str) -> Vec<u8> {
    let mut buf = Vec::new();
    encode_string(2, code, &mut buf);
    buf
}

fn encode_flight_data(leg: &FlightLeg) -> Vec<u8> {
    let mut buf = Vec::new();

    encode_string(2, &leg.date, &mut buf);

    if let Some(max_stops) = leg.max_stops {
        encode_tag(5, 0, &mut buf);
        encode_varint(max_stops as u64, &mut buf);
    }

    if let Some(ref airlines) = leg.airlines {
        for airline in airlines {
            encode_string(6, airline, &mut buf);
        }
    }

    let from = encode_airport(&leg.from_airport);
    encode_submessage(13, &from, &mut buf);

    let to = encode_airport(&leg.to_airport);
    encode_submessage(14, &to, &mut buf);

    buf
}

fn seat_to_varint(seat: &Seat) -> u64 {
    match seat {
        Seat::Economy => 1,
        Seat::PremiumEconomy => 2,
        Seat::Business => 3,
        Seat::First => 4,
    }
}

fn trip_to_varint(trip: &TripType) -> u64 {
    match trip {
        TripType::RoundTrip => 1,
        TripType::OneWay => 2,
        TripType::MultiCity => 3,
    }
}

fn passengers_to_enums(p: &Passengers) -> Vec<u64> {
    let mut vals = Vec::new();
    vals.extend(std::iter::repeat_n(1, p.adults as usize));
    vals.extend(std::iter::repeat_n(2, p.children as usize));
    vals.extend(std::iter::repeat_n(3, p.infants_in_seat as usize));
    vals.extend(std::iter::repeat_n(4, p.infants_on_lap as usize));
    vals
}

pub fn encode(
    legs: &[FlightLeg],
    passengers: &Passengers,
    seat: &Seat,
    trip: &TripType,
) -> Vec<u8> {
    let mut buf = Vec::new();

    for leg in legs {
        let fd = encode_flight_data(leg);
        encode_submessage(3, &fd, &mut buf);
    }

    let pax = passengers_to_enums(passengers);
    if !pax.is_empty() {
        let mut packed = Vec::new();
        for v in &pax {
            encode_varint(*v, &mut packed);
        }
        encode_tag(8, 2, &mut buf);
        encode_varint(packed.len() as u64, &mut buf);
        buf.extend_from_slice(&packed);
    }

    encode_tag(9, 0, &mut buf);
    encode_varint(seat_to_varint(seat), &mut buf);

    encode_tag(19, 0, &mut buf);
    encode_varint(trip_to_varint(trip), &mut buf);

    buf
}
