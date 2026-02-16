# flyr

A native Rust CLI that scrapes Google Flights. Single static binary, no API key, no browser, no Python.

Inspired by the [fast-flights](https://github.com/AWeirdDev/flights) Python library, rewritten from scratch in Rust with fixes for GDPR consent walls, flaky HTML parsing, and Python startup overhead.

## Install

```bash
cargo install --path .
```

Requires `cmake`, `perl`, and `clang` for the BoringSSL build (wreq dependency).

Arch: `pacman -S cmake perl clang`

## Usage

```bash
# One-way
flyr search -f HEL -t BKK -d 2026-03-01

# Round-trip
flyr search -f LAX -t NRT -d 2026-05-01 --return-date 2026-05-15

# Multi-city
flyr search --leg "2026-03-01 LAX NRT" --leg "2026-03-10 NRT SEA"

# Filters
flyr search -f HEL -t BCN -d 2026-03-01 --seat business --max-stops 1 --airlines AY,IB

# JSON output
flyr search -f HEL -t BKK -d 2026-03-01 --json --pretty

# Proxy + timeout
flyr search -f JFK -t LHR -d 2026-04-01 --proxy socks5://127.0.0.1:1080 --timeout 60

# Multiple passengers
flyr search -f HEL -t ATH -d 2026-03-01 --adults 2 --children 1

# Currency and language
flyr search -f HEL -t BKK -d 2026-03-01 --currency EUR --lang fi
```

<details>
<summary><strong>All options</strong></summary>

```
flyr search [OPTIONS]

REQUIRED (simple mode):
  -f, --from <IATA>           Departure airport (3-letter IATA code)
  -t, --to <IATA>             Arrival airport
  -d, --date <YYYY-MM-DD>     Departure date

MULTI-CITY (replaces -f/-t/-d):
  --leg <"DATE FROM TO">      Flight leg, repeatable

TRIP:
  --return-date <YYYY-MM-DD>  Return date (auto-sets round-trip)
  --trip <TYPE>                one-way | round-trip | multi-city  [default: one-way]
  --seat <CLASS>               economy | premium-economy | business | first  [default: economy]

FILTERS:
  --max-stops <N>              0 = nonstop only
  --airlines <AA,DL,...>       Comma-separated IATA codes

PASSENGERS:
  --adults <N>                 [default: 1]
  --children <N>               [default: 0]
  --infants-in-seat <N>        [default: 0]
  --infants-on-lap <N>         [default: 0]

OUTPUT:
  --json                       JSON to stdout
  --pretty                     Pretty-printed JSON to stdout
  --currency <CODE>            [default: USD]
  --lang <CODE>                [default: en]

CONNECTION:
  --proxy <URL>                HTTP or SOCKS5 proxy
  --timeout <SECS>             [default: 30]
```

</details>

## Output

### Table (default)

```
$ flyr search -f HEL -t BKK -d 2026-03-01 --currency EUR
┌───────────────────┬───────────┬──────────────────┬──────────────────┬────────┬────────┬────────────┬───────┐
│ Airlines          │ Route     │ Depart           │ Arrive           │ Dur.   │ Stops  │ Aircraft   │ Price │
├───────────────────┼───────────┼──────────────────┼──────────────────┼────────┼────────┼────────────┼───────┤
│ Finnair           │ HEL → BKK │ 2026-03-01 17:00 │ 2026-03-02 07:15 │ 10h 15 │ Nonstop│ Airbus A350│ €589  │
│ Turkish, THAI     │ HEL → IST │ 2026-03-01 19:00 │ 2026-03-02 05:20 │ 12h 40 │ 1 (IST)│ A321, A350 │ €498  │
│                   │ IST → BKK │                  │                  │        │        │            │       │
└───────────────────┴───────────┴──────────────────┴──────────────────┴────────┴────────┴────────────┴───────┘
```

### JSON

```bash
flyr search -f HEL -t BKK -d 2026-03-01 --json --pretty
```

```json
{
  "flights": [
    {
      "flight_type": "AY",
      "airlines": ["Finnair"],
      "segments": [
        {
          "from_airport": { "code": "HEL", "name": "Helsinki Airport" },
          "to_airport": { "code": "BKK", "name": "Suvarnabhumi Airport" },
          "departure": { "year": 2026, "month": 3, "day": 1, "hour": 17, "minute": 0 },
          "arrival": { "year": 2026, "month": 3, "day": 2, "hour": 7, "minute": 15 },
          "duration_minutes": 675,
          "aircraft": "Airbus A350"
        }
      ],
      "price": 589,
      "carbon": { "emission_grams": 570000, "typical_grams": 690000 }
    }
  ],
  "metadata": {
    "airlines": [{ "code": "AY", "name": "Finnair" }],
    "alliances": [{ "code": "ONEWORLD", "name": "Oneworld" }]
  }
}
```

Pipe to `jq` for scripting:

```bash
# Cheapest price
flyr search -f HEL -t BKK -d 2026-03-01 --json | jq '.flights | sort_by(.price) | first'

# All nonstop flights
flyr search -f JFK -t LHR -d 2026-04-01 --json | jq '[.flights[] | select(.segments | length == 1)]'

# Just prices and airlines
flyr search -f HEL -t BCN -d 2026-03-01 --json | jq '.flights[] | {airlines, price}'
```

## Exit codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 2 | Validation error (bad airport code, invalid date, etc.) |
| 3 | Network error (timeout, DNS, TLS, proxy) |
| 4 | Rate limited or blocked by Google |
| 5 | Unexpected HTTP status |
| 6 | Parse error (Google changed their page structure) |

In `--json` mode, errors are structured JSON to stdout:

```json
{ "error": { "kind": "invalid_airport", "message": "invalid airport code \"XX\" -- must be exactly 3 letters" } }
```

In human mode, errors go to stderr.

<details>
<summary><strong>How it works</strong></summary>

1. **Query encoding** -- Flight parameters are protobuf-encoded (hand-rolled encoder, ~130 LOC) and base64-encoded into the `tfs` URL parameter, matching what Google Flights expects.

2. **HTTP request** -- Uses [wreq](https://github.com/nickel-org/wreq) (reqwest fork) with Chrome 137 TLS fingerprint emulation to avoid bot detection. Pre-loads GDPR consent cookies to bypass the EU consent wall.

3. **HTML parsing** -- Extracts the `<script class="ds:1">` tag, isolates the `data:` JSON payload, parses with serde_json.

4. **Payload navigation** -- The JSON payload is deeply nested arrays. Flight data lives at `payload[3][0][i]`, with segments, prices, carbon data, and metadata at fixed indices. All access is safe (no panics on missing data).

</details>

<details>
<summary><strong>Library usage</strong></summary>

The crate exposes a public API for use as a library:

```rust
use flyr::query::*;
use flyr::fetch::FetchOptions;

let params = QueryParams {
    legs: vec![FlightLeg {
        date: "2026-03-01".into(),
        from_airport: "HEL".into(),
        to_airport: "BKK".into(),
        max_stops: None,
        airlines: None,
    }],
    passengers: Passengers::default(),
    seat: Seat::Economy,
    trip: TripType::OneWay,
    language: "en".into(),
    currency: "USD".into(),
};

params.validate()?;

let result = flyr::search(
    SearchQuery::Structured(params),
    FetchOptions::default(),
).await?;

for flight in &result.flights {
    println!("{}: {} segments, ${}", flight.airlines.join(", "), flight.segments.len(), flight.price.unwrap_or(0));
}
```

</details>

<details>
<summary><strong>Project structure</strong></summary>

```
src/
├── main.rs     CLI entry point (clap)
├── lib.rs      Public API: search(query, options) -> Result<SearchResult>
├── proto.rs    Hand-rolled protobuf encoder (~130 LOC)
├── query.rs    Query building, validation, URL param generation
├── fetch.rs    HTTP client with Chrome TLS impersonation + GDPR cookies
├── parse.rs    HTML script extraction + JSON payload navigation
├── model.rs    All data types (Serialize + Debug + Clone)
├── table.rs    Human-readable table rendering with currency symbols
└── error.rs    Error types with actionable messages
tests/
├── cli_test.rs     29 tests -- arg parsing, help output, error messages, exit codes
├── parse_test.rs   13 tests -- script extraction, JSON parsing, edge cases
├── proto_test.rs    6 tests -- byte-level protobuf correctness
└── query_test.rs   20 tests -- validation rules, date handling, leap years
```

</details>

## License

GPL-3.0
