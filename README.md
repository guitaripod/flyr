# flyr

[![crates.io](https://img.shields.io/crates/v/flyr-cli)](https://crates.io/crates/flyr-cli)
[![License: GPL-3.0](https://img.shields.io/github/license/guitaripod/flyr)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.89%2B-orange)](https://www.rust-lang.org)
[![GitHub stars](https://img.shields.io/github/stars/guitaripod/flyr)](https://github.com/guitaripod/flyr/stargazers)
[![GitHub last commit](https://img.shields.io/github/last-commit/guitaripod/flyr)](https://github.com/guitaripod/flyr/commits/master)

Google Flights from your terminal. Single binary, no API key, no browser.

> **prompt:** find the cheapest round-trip from Helsinki to somewhere warm, a week in mid-November. open the best one in my browser.
>
> **agent runs:**
>
> ```
> flyr search -f HEL -t AYT,NCE,ATH,RAK,LPA,SIN,BKK -d 2026-11-16 --return-date 2026-11-23 --compact --top 1 --currency EUR
> ```

> **output:**
>
> ```
> === AYT ===
> €251 round trip | HEL>SAW>AYT | 7h00m | 1 stop SAW 2h00m | Pegasus | Nov16 14:50>22:50 | PC1312,PC2020
> Cheaper on 2026-11-17 Tue returning 2026-11-24 Tue: €167
> === NCE ===
> €197 round trip | HEL>CDG>NCE | 6h10m | 1 stop CDG 1h10m | Air France | Nov16 17:35>22:45 | AF1071,AF7318
> === ATH ===
> €258 round trip | HEL>AMS>ATH | 7h00m | 1 stop AMS 1h15m | KLM | Nov16 18:20>01:20+1 | KL1254,KL1957
> Cheaper on 2026-11-17 Tue returning 2026-11-24 Tue: €188
> === RAK ===
> €273 round trip | HEL>FRA>RAK | 8h20m | 1 stop FRA 1h40m | Lufthansa City Airlines, Discover Airlines | Nov16 17:35>23:55 | VL851,4Y204
> === LPA ===
> €340 round trip | HEL>ARN>LPA | 9h25m | 1 stop ARN 2h20m | Norwegian | Nov16 09:20>16:45 | D84282,D85701
> === SIN ===
> €550 round trip | HEL>IST>SIN | 16h35m | 1 stop IST 1h55m overnight | Turkish Airlines | Nov16 19:05>17:40+1 | TK1764,TK54
> === BKK ===
> €646 round trip | HEL>DOH>BKK | 17h00m | 1 stop DOH 4h30m overnight | Qatar Airways | Nov16 15:05>13:05+1 | QR302,QR836
> ```
>
> **then opens the winner:** `flyr search -f HEL -t NCE -d 2026-11-16 --return-date 2026-11-23 --currency EUR --open`
>
> One command, 7 destinations, a few hundred tokens. No browser, no clicking, no cookie banners. Pick the flights you want and the agent opens them directly on Google Flights to book.

## Why

Google Flights has no API. The website is slow, requires a browser, and you can only search one route at a time. If you want to compare 10 destinations you're clicking through 10 separate searches, waiting for each page to load, fighting cookie banners.

flyr fixes this. It's a single static binary that scrapes Google Flights directly. Search multiple destinations in one call. Pipe output into scripts, or let an AI agent search dozens of routes and compile results -- like the table above.

Built for people who book flights programmatically -- whether that's a bash loop, a Python script, or an LLM agent that can run shell commands.

## Install

```bash
cargo install flyr-cli
```

Requires `cmake`, `perl`, and `clang` for the BoringSSL build (wreq dependency).

Arch: `pacman -S cmake perl clang`

## Usage

```bash
flyr search -f HEL -t BKK -d 2026-03-01
flyr search -f LAX -t NRT -d 2026-05-01 --return-date 2026-05-15
flyr search -f HEL -t BKK -d 2026-03-01 --json --currency EUR
flyr search -f HEL -t DXB -d 2026-03-01 --open
flyr dates -f HEL -t BCN -d 2026-03 --stay 7
```

### Agent mode

flyr is designed for LLM agents. Three flags minimize token consumption:

- **`--compact`** -- one line per flight, pipe-delimited, no box-drawing characters
- **`--top N`** -- only return the N cheapest results
- **`-t BCN,ATH,AYT`** -- multi-destination in a single invocation

```bash
flyr search -f HEL -t BCN,ATH,AYT -d 2026-03-01 --compact --top 3 --currency EUR
flyr dates -f HEL -t BCN -d 2026-03 --top 5 --compact --currency EUR
```

An agent can:

- Search dozens of routes in one call
- Filter by price, stops, departure time
- Compare destinations and compile results
- Open the best options directly in a browser

### MCP (Model Context Protocol)

flyr includes a built-in MCP server (`flyr mcp`) that works with local LLMs via [opencode](https://opencode.ai) + [Ollama](https://ollama.com). Search flights, compare destinations, find the cheapest dates, and open results in your browser — entirely offline, no API keys, no cloud LLMs. Any model with tool support works (Ministral, Llama, Qwen, etc.).

Tools: `flyr_search` (flights on given dates, one compact line per flight), `flyr_dates` (cheapest fare per departure date across a range) and `flyr_get_url` (the Google Flights link for a search; `open: true` opens it in the browser).

This is one way to use flyr with AI. Agents with shell access (Claude Code, Cursor, aider) don't need MCP — they can run `flyr search` directly, which exposes more options.

opencode config (`opencode.jsonc`):
```json
{ "mcp": { "flyr": { "type": "local", "command": ["flyr", "mcp"] } } }
```

### Multi-destination search

Comma-separate destination codes in `-t`:

```bash
flyr search -f HEL -t BKK,SIN,KUL,HKT,DPS -d 2026-03-01 --compact --top 3 --currency EUR
```

Destinations are searched concurrently, up to 4 at a time. Output is grouped by destination, in the order given.

Works with `--return-date` (each destination gets its own return leg), `--top`, `--url`, `--open` and all output modes.

A destination that fails shows its error in place of results (an `error` object with `--json`), so a rate limit never reads as "no flights". The exit code is non-zero only when every destination fails.

Cannot be combined with `--leg` (use separate invocations for multi-city itineraries).

### City codes

IATA city codes search every airport in a metro area, anywhere `flyr search` accepts an airport code (`flyr dates` needs airport codes):

```bash
flyr search -f LON -t NYC -d 2026-03-01 --compact --top 3
```

Common ones: `LON`, `NYC`, `PAR`, `TYO`, `MIL`, `ROM`, `STO`, `CHI`, `WAS`, `SEL`, `BJS`, `OSA`.

### Cheapest dates

`flyr dates` prices every departure date in a month (`-d 2026-11`) or range (`-d 2026-11-10..2026-11-16`) with a single request:

```
$ flyr dates -f HEL -t BKK -d 2026-11-10..2026-11-16 --currency EUR
┌────────────────┬───────┐
│ Depart         ┆ Price │
╞════════════════╪═══════╡
│ 2026-11-10 Tue ┆ €310  │
├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌┤
│ 2026-11-11 Wed ┆ €310  │
├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌┤
│ 2026-11-12 Thu ┆ €310  │
├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌┤
│ 2026-11-13 Fri ┆ €374  │
...
```

Add `--stay N` for round trips of N days, and `--top N` for the N cheapest dates:

```
$ flyr dates -f HEL -t BCN -d 2026-11 --stay 7 --top 5 --compact --currency EUR
2026-11-11 Wed > 2026-11-18 Wed | €167 round trip
2026-11-28 Sat > 2026-12-05 Sat | €167 round trip
2026-11-17 Tue > 2026-11-24 Tue | €188 round trip
2026-11-03 Tue > 2026-11-10 Tue | €189 round trip
2026-11-24 Tue > 2026-12-01 Tue | €189 round trip
```

Dates already in the past are skipped. Filters (`--seat`, `--max-stops`, `--airlines`, passengers) work as in `flyr search`.

`flyr search` also prints Google's hint when a nearby date is cheaper, e.g. `Cheaper on 2026-11-17 Tue returning 2026-11-24 Tue: €167`.

### Concurrent searches (advanced)

For more complex scenarios beyond multi-destination, you can still run parallel shell processes:

```bash
for dest in BKK SIN KUL HKT DPS; do
  flyr search -f HEL -t $dest -d 2026-03-01 --return-date 2026-03-08 --json --currency EUR &
done | jq -s '[.[] | .flights | min_by(.price) | {dest: .segments[-1].to_airport.code, price, airlines}] | sort_by(.price)'
```

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
  -f, --from <IATA>           Departure airport or city (3-letter IATA code)
  -t, --to <IATA>             Arrival airport or city (comma-separate for multi-destination)
  -d, --date <YYYY-MM-DD>     Departure date

MULTI-CITY (replaces -f/-t/-d):
  --leg <"DATE FROM TO">      Flight leg, repeatable

TRIP:
  --return-date <YYYY-MM-DD>  Return date (auto-sets round-trip)
  --trip <TYPE>               one-way | round-trip | multi-city  [default: one-way]

FILTERS:
  --seat <CLASS>              economy | premium-economy | business | first  [default: economy]
  --max-stops <N>             0 = nonstop only
  --airlines <AA,DL,...>      Comma-separated IATA codes
  --adults <N>                [default: 1]
  --children <N>              [default: 0]
  --infants-in-seat <N>       [default: 0]
  --infants-on-lap <N>        [default: 0]

OUTPUT:
  --top <N>                   Show only the N cheapest results
  --compact                   One line per result (recommended for scripts and AI agents)
  --json                      JSON to stdout
  --pretty                    Pretty-printed JSON to stdout
  --currency <CODE>           [default: USD]
  --lang <CODE>               [default: en]
  --open                      Open results in Google Flights
  --url                       Output Google Flights URL only (for AI agents)

CONNECTION:
  --proxy <URL>               HTTP or SOCKS5 proxy
  --timeout <SECS>            [default: 30]

flyr dates [OPTIONS]

  -f, --from <IATA>           Departure airport (no city codes)
  -t, --to <IATA>             Arrival airport (no city codes)
  -d, --date <RANGE>          YYYY-MM, YYYY-MM-DD..YYYY-MM-DD or YYYY-MM-DD
  --stay <DAYS>               Round trips of this many days (omit for one-way)

  Plus the FILTERS, --top, --compact, --json, --pretty, --currency and CONNECTION options above.
```

</details>

## Output

### Compact (recommended for agents)

```
$ flyr search -f HEL -t BCN -d 2026-11-16 --compact --top 3 --currency EUR
€124 | HEL>ARN>BCN | 6h05m | 1 stop ARN 1h25m | Norwegian | Nov16 16:40>21:45 | D82612,D85503
€138 | HEL>AMS>BCN | 10h00m | 1 stop AMS 5h10m | KLM | Nov16 14:05>23:05 | KL1252,KL1521
€147 | HEL>BCN | 4h00m | nonstop | Finnair | Nov16 16:55>19:55 | AY1653
```

Fields: price (`round trip` when the price covers both directions) | route | total travel time including layovers | stops with each layover's wait (`overnight` when it spans midnight, `airport change` when you switch airports) | airlines | departure>arrival local times (`+1` = next day) | flight numbers.

Round trips list the outbound flights, each priced for the whole trip; pick the return on Google Flights with `--open`.

### Table (default)

```
$ flyr search -f HEL -t BKK -d 2026-11-16 --currency EUR --top 2
┌───────────────────────────────┬───────────┬────────┬──────────────────┬──────────────────┬──────────┬──────────────────────┬────────────────┬───────┐
│ Airlines                      ┆ Route     ┆ Flight ┆ Depart           ┆ Arrive           ┆ Duration ┆ Stops                ┆ Aircraft       ┆ Price │
╞═══════════════════════════════╪═══════════╪════════╪══════════════════╪══════════════════╪══════════╪══════════════════════╪════════════════╪═══════╡
│ Scandinavian Airlines, Etihad ┆ HEL → CPH ┆ SK1705 ┆ 2026-11-16 06:15 ┆ 2026-11-17 23:40 ┆ 36h25m   ┆ CPH 3h10m            ┆ Airbus A320neo ┆ €310  │
│                               ┆ CPH → AUH ┆ EY178  ┆                  ┆                  ┆          ┆ AUH 18h50m overnight ┆ Boeing 787     ┆       │
│                               ┆ AUH → BKK ┆ EY400  ┆                  ┆                  ┆          ┆                      ┆ Airbus A321neo ┆       │
├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌┤
│ Scandinavian Airlines, Etihad ┆ HEL → CPH ┆ SK1705 ┆ 2026-11-16 06:15 ┆ 2026-11-17 06:35 ┆ 19h20m   ┆ CPH 3h10m            ┆ Airbus A320neo ┆ €445  │
│                               ┆ CPH → AUH ┆ EY178  ┆                  ┆                  ┆          ┆ AUH 1h50m            ┆ Boeing 787     ┆       │
│                               ┆ AUH → BKK ┆ EY402  ┆                  ┆                  ┆          ┆                      ┆ Airbus A380    ┆       │
└───────────────────────────────┴───────────┴────────┴──────────────────┴──────────────────┴──────────┴──────────────────────┴────────────────┴───────┘
```

### JSON

```bash
flyr search -f HEL -t BCN -d 2026-11-16 --currency EUR --top 1 --json --pretty
```

```json
{
  "flights": [
    {
      "flight_type": "D8",
      "airlines": ["Norwegian"],
      "segments": [
        {
          "from_airport": { "code": "HEL", "name": "Helsinki Airport" },
          "to_airport": { "code": "ARN", "name": "Stockholm Arlanda Airport" },
          "departure": { "year": 2026, "month": 11, "day": 16, "hour": 16, "minute": 40 },
          "arrival": { "year": 2026, "month": 11, "day": 16, "hour": 16, "minute": 40 },
          "duration_minutes": 60,
          "aircraft": "Boeing 737MAX 8 Passenger",
          "flight_number": "D82612",
          "codeshares": []
        },
        {
          "from_airport": { "code": "ARN", "name": "Stockholm Arlanda Airport" },
          "to_airport": { "code": "BCN", "name": "Josep Tarradellas Barcelona-El Prat Airport" },
          "departure": { "year": 2026, "month": 11, "day": 16, "hour": 18, "minute": 5 },
          "arrival": { "year": 2026, "month": 11, "day": 16, "hour": 21, "minute": 45 },
          "duration_minutes": 220,
          "aircraft": "Boeing 737",
          "flight_number": "D85503",
          "codeshares": []
        }
      ],
      "layovers": [
        { "airport": "ARN", "duration_minutes": 85, "overnight": false, "change_of_airport": false }
      ],
      "duration_minutes": 365,
      "price": 124,
      "carbon": { "emission_grams": 232000, "typical_grams": 231000 }
    }
  ],
  "cheaper_date": null,
  "metadata": {
    "airlines": [{ "code": "A3", "name": "Aegean" }, { "code": "BT", "name": "Air Baltic" }],
    "alliances": [{ "code": "ONEWORLD", "name": "Oneworld" }]
  }
}
```

Times are local to each airport. `duration_minutes` on a flight is the total door-to-door time including layovers; on a segment it's that flight alone. `cheaper_date` holds Google's nearby-date suggestion (`date`, `return_date`, `price`) when it has one. With several destinations, the JSON is keyed by destination code.

`flyr dates --json` prints an array of `{ "date", "return_date", "price" }`.

<details>
<summary><strong>jq recipes</strong></summary>

```bash
flyr search -f HEL -t BKK -d 2026-03-01 --json | jq '.flights | sort_by(.price) | first'

flyr search -f JFK -t LHR -d 2026-04-01 --json | jq '[.flights[] | select(.layovers | length == 0)]'

flyr search -f HEL -t BKK -d 2026-03-01 --json | jq '[.flights[] | select(all(.layovers[]; .overnight | not))] | min_by(.price)'

flyr search -f HEL -t BCN -d 2026-03-01 --json | jq '.flights[] | {airlines, price}'
```

</details>

## Exit codes

| Code | Meaning                                                 |
| ---- | ------------------------------------------------------- |
| 0    | Success                                                 |
| 2    | Validation error (bad airport code, invalid date, etc.) |
| 3    | Network error (timeout, DNS, TLS, proxy)                |
| 4    | Rate limited or blocked by Google                       |
| 5    | Unexpected HTTP status                                  |
| 6    | Parse error (Google changed their page structure)       |

In `--json` mode, errors are structured JSON to stdout:

```json
{
  "error": {
    "kind": "invalid_airport",
    "message": "invalid airport code \"XX\" -- must be exactly 3 letters"
  }
}
```

In human mode, errors go to stderr.

`flyr dates` reports `"kind": "rejected"` when Google turns the date search down, typically for city codes like LON.

<details>
<summary><strong>How it works</strong></summary>

1. **Query encoding** -- Flight parameters are protobuf-encoded (hand-rolled encoder, ~130 LOC) and base64-encoded into the `tfs` URL parameter, matching what Google Flights expects.

2. **HTTP request** -- Uses [wreq](https://github.com/nickel-org/wreq) (reqwest fork) with Chrome 149 TLS fingerprint emulation to avoid bot detection. Automatically handles Google's EU consent wall by detecting consent redirects and submitting the acceptance form.

3. **HTML parsing** -- Extracts the `<script class="ds:1">` tag, isolates the `data:` JSON payload, parses with serde_json.

4. **Payload navigation** -- The JSON payload is deeply nested arrays. Google's "top flights" live at `payload[2][0]` and the remaining flights at `payload[3][0]`, with segments, layovers, total duration, prices, carbon data and metadata at fixed indices; `payload[6]` carries the cheaper-nearby-date hint. All access is safe (no panics on missing data).

5. **Date search** -- `flyr dates` posts the same filters the Google Flights date grid sends to its `GetCalendarGraph` endpoint and reads the cheapest fare per day from the response.

</details>

<details>
<summary><strong>Library usage</strong></summary>

The crate exposes a public API for use as a library:

```rust
use flyr::fetch::FetchOptions;
use flyr::query::{parse_date_range, DateQuery, Filters, QueryParams};

let filters = Filters { max_stops: Some(1), ..Filters::default() };

let params = QueryParams::route("HEL", "BKK", "2026-03-01", None, &filters);
params.validate()?;

let result = flyr::search(&params, &FetchOptions::default()).await?;
for flight in &result.flights {
    println!("{}: {} min, ${}", flight.airlines.join(", "), flight.duration_minutes, flight.price.unwrap_or(0));
}

let (start, end) = parse_date_range("2026-03")?;
let dates = DateQuery { from_airport: "HEL".into(), to_airport: "BCN".into(), start, end, stay_days: Some(7), filters };
dates.validate()?;
for day in flyr::search_dates(&dates, &FetchOptions::default()).await? {
    println!("{} {}", day.date, day.price);
}
```

</details>

<details>
<summary><strong>Project structure</strong></summary>

```
src/
├── main.rs     CLI entry point (clap)
├── lib.rs      Public API: search, search_all, search_dates
├── mcp.rs      Built-in MCP server (rmcp, stdio transport)
├── proto.rs    Hand-rolled protobuf encoder (~130 LOC)
├── query.rs    Query building, validation, URL params and date-search requests
├── fetch.rs    HTTP client with Chrome TLS impersonation + consent handling
├── parse.rs    Results-page and date-search response parsing
├── model.rs    All data types (Serialize + Debug + Clone)
├── table.rs    Table and compact rendering with currency symbols
├── date.rs     Calendar dates: parsing, arithmetic, weekdays
└── error.rs    Error types with actionable messages
tests/
├── cli_test.rs     46 tests -- arg parsing, help output, errors, exit codes, MCP over stdio
├── parse_test.rs   25 tests -- script extraction, recorded Google responses, edge cases
├── proto_test.rs    6 tests -- byte-level protobuf correctness
├── query_test.rs   38 tests -- validation, dates, ranges, date-search request encoding
├── table_test.rs    7 tests -- compact and table rendering
└── fixtures/       Recorded Google Flights payloads and date-search responses
```

</details>

## License

GPL-3.0
