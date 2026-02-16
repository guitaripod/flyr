# flyr

[![License: GPL-3.0](https://img.shields.io/github/license/guitaripod/flyr)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange)](https://www.rust-lang.org)
[![GitHub stars](https://img.shields.io/github/stars/guitaripod/flyr)](https://github.com/guitaripod/flyr/stargazers)
[![GitHub last commit](https://img.shields.io/github/last-commit/guitaripod/flyr)](https://github.com/guitaripod/flyr/commits/master)

Google Flights from your terminal. Single binary, no API key, no browser.

> **prompt:** find the cheapest round-trip from Helsinki to somewhere warm, departing tomorrow. open the best ones in my browser.
>
> **agent:** *searches 20 destinations in parallel*
>
> | # | Dest | City | Price | Route | Stops | Temp |
> |---|------|------|-------|-------|-------|------|
> | 1 | AYT | Antalya | **€236** | Pegasus direct | 0 | 15°C |
> | 2 | NCE | Nice | €260 | Air France via CDG | 1 | 12°C |
> | 3 | ATH | Athens | €331 | KLM via AMS | 1 | 13°C |
> | 4 | RAK | Marrakech | €347 | Finnair/TAP via LIS | 1 | 20°C |
> | 5 | LPA | Gran Canaria | €443 | Norwegian direct | 0 | 21°C |
> | 6 | SIN | Singapore | €710 | KLM via AMS | 1 | 31°C |
> | 7 | BKK | Bangkok | €859 | Finnair/Qatar via DOH | 1 | 33°C |
>
> *Opens Antalya, Gran Canaria, and Singapore on Google Flights for booking.*

20 parallel `flyr` calls. Each search takes 1-2 seconds. No browser, no clicking, no cookie banners. Pick the flights you want and the agent opens them directly on Google Flights to book.

## Why

Google Flights has no API. The website is slow, requires a browser, and you can only search one route at a time. If you want to compare 10 destinations you're clicking through 10 separate searches, waiting for each page to load, fighting cookie banners.

flyr fixes this. It's a single static binary that scrapes Google Flights directly. Run searches in parallel. Pipe JSON into `jq`, feed it to scripts, or let an AI agent search dozens of routes and compile results for you -- like the table above.

Built for people who book flights programmatically -- whether that's a bash loop, a Python script, or an LLM agent that can run shell commands.

## Install

```bash
cargo install --path .
```

Requires `cmake`, `perl`, and `clang` for the BoringSSL build (wreq dependency).

Arch: `pacman -S cmake perl clang`

## Usage

```bash
flyr search -f HEL -t BKK -d 2026-03-01
flyr search -f LAX -t NRT -d 2026-05-01 --return-date 2026-05-15
flyr search -f HEL -t BKK -d 2026-03-01 --json --currency EUR
```

### Concurrent searches

Search 5 destinations at once:

```bash
for dest in BKK SIN KUL HKT DPS; do
  flyr search -f HEL -t $dest -d 2026-03-01 --return-date 2026-03-08 --json --currency EUR &
done | jq -s '[.[] | .flights[0] | {dest: .segments[0].to_airport.code, price, airlines}] | sort_by(.price)'
```

### AI agent usage

flyr's structured JSON output makes it a natural tool for LLM agents. An agent can:

- Search dozens of routes in parallel
- Filter by price, stops, departure time
- Compare destinations and compile results
- Open the best options directly in a browser

```bash
flyr search -f HEL -t BKK -d 2026-02-17 --return-date 2026-02-24 --json --currency EUR
```

Returns structured data the agent can parse and reason about -- prices, airlines, segments, times, carbon emissions, all as clean JSON.

### Localization

Results adapt to any language and currency Google Flights supports:

```bash
flyr search -f HEL -t BKK -d 2026-03-01 --currency EUR --lang fi
flyr search -f HEL -t BKK -d 2026-03-01 --currency JPY --lang ja
flyr search -f HEL -t BKK -d 2026-03-01 --currency THB --lang th
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

<details>
<summary><strong>jq recipes</strong></summary>

```bash
flyr search -f HEL -t BKK -d 2026-03-01 --json | jq '.flights | sort_by(.price) | first'

flyr search -f JFK -t LHR -d 2026-04-01 --json | jq '[.flights[] | select(.segments | length == 1)]'

flyr search -f HEL -t BCN -d 2026-03-01 --json | jq '.flights[] | {airlines, price}'
```

</details>

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
