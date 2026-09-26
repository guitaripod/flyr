use std::process;

use clap::Parser;
use serde::Serialize;
use serde_json::Value;

use flyr::error::{FlightError, INVALID_ARGUMENT};
use flyr::fetch::FetchOptions;
use flyr::model::{keep_cheapest_dates, SearchResult};
use flyr::query::{
    parse_codes, parse_date_range, route_queries, DateQuery, Filters, Passengers, QueryParams,
    Seat, TripType,
};
use flyr::table;

#[derive(Parser)]
#[command(
    name = "flyr",
    about = "Search Google Flights from the terminal",
    version,
    after_help = "\
Examples:
  flyr search -f JFK -t LHR -d 2026-04-01
  flyr search -f HEL -t BCN -d 2026-03-01 --json --pretty
  flyr search -f LAX -t NRT -d 2026-05-01 --return-date 2026-05-15
  flyr search -f HEL -t BKK -d 2026-03-01 --seat business --max-stops 1
  flyr search --leg \"2026-03-01 LAX NRT\" --leg \"2026-03-10 NRT LAX\"
  flyr search -f HEL -t BCN -d 2026-03-01 --airlines AY,IB --adults 2
  flyr dates -f HEL -t BCN -d 2026-03 --stay 7

Agent-optimized:
  flyr search -f HEL -t BCN,ATH,AYT -d 2026-03-01 --compact --top 3 --currency EUR"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand)]
enum Commands {
    #[command(
        about = "Search for flights",
        long_about = "Search for flights between airports on specific dates.\n\
            Use -f/-t/-d for simple searches, or --leg for multi-city itineraries.\n\
            For AI agents: use --compact --top N for minimal output. Comma-separate -t for multi-destination.",
        after_help = "\
Examples:
  One-way:      flyr search -f JFK -t LHR -d 2026-04-01
  Round-trip:   flyr search -f LAX -t NRT -d 2026-05-01 --return-date 2026-05-15
  Multi-city:   flyr search --leg \"2026-03-01 LAX NRT\" --leg \"2026-03-10 NRT SEA\"
  Business:     flyr search -f HEL -t BKK -d 2026-03-01 --seat business --max-stops 1
  JSON output:  flyr search -f HEL -t BCN -d 2026-03-01 --json --pretty
  With filter:  flyr search -f HEL -t BCN -d 2026-03-01 --airlines AY,IB

Agent-optimized:
  flyr search -f HEL -t BCN,ATH,AYT -d 2026-03-01 --compact --top 3 --currency EUR"
    )]
    Search(SearchArgs),
    #[command(
        about = "Find the cheapest dates to fly across a month or date range",
        long_about = "Show the cheapest fare for every departure date in a month or date range, \
            in a single request.\n\
            One-way by default; add --stay N for round trips of N days.\n\
            Takes airport codes only: city codes like LON aren't supported for date searches.",
        after_help = "\
Examples:
  Whole month:  flyr dates -f HEL -t BCN -d 2026-03
  Date range:   flyr dates -f LHR -t JFK -d 2026-03-01..2026-04-15
  Round-trip:   flyr dates -f HEL -t BCN -d 2026-03 --stay 7
  Cheapest 5:   flyr dates -f HEL -t BKK -d 2026-03 --top 5 --compact"
    )]
    Dates(DatesArgs),
    #[command(about = "Start MCP server for AI agents (stdio transport)")]
    Mcp,
}

#[derive(clap::Args)]
struct SearchArgs {
    #[arg(
        short, long,
        value_name = "IATA",
        help = "Departure airport code",
        long_help = "Departure airport IATA code (3 letters, e.g. JFK, HEL, LAX). \
            City codes cover every airport in a metro area (e.g. LON, NYC, PAR, TYO). \
            Required unless using --leg."
    )]
    from: Option<String>,

    #[arg(
        short, long,
        value_name = "IATA",
        help = "Arrival airport code (comma-separate for multi-destination)",
        long_help = "Arrival airport IATA code (3 letters, e.g. LHR, BCN, NRT). \
            City codes cover every airport in a metro area (e.g. LON, NYC, PAR, TYO). \
            Comma-separate for multi-destination search (e.g. BCN,ATH,AYT). \
            Required unless using --leg."
    )]
    to: Option<String>,

    #[arg(
        short, long,
        value_name = "YYYY-MM-DD",
        help = "Departure date",
        long_help = "Departure date in YYYY-MM-DD format. Required unless using --leg."
    )]
    date: Option<String>,

    #[arg(
        long,
        value_name = "\"DATE FROM TO\"",
        help = "Flight leg (repeatable, for multi-city)",
        long_help = "Define a flight leg as \"YYYY-MM-DD FROM TO\". Repeat for multi-city \
            itineraries. Replaces -f/-t/-d when used.\n\
            Example: --leg \"2026-03-01 LAX NRT\" --leg \"2026-03-10 NRT SEA\"",
        num_args = 1,
    )]
    leg: Vec<String>,

    #[arg(
        long,
        value_name = "YYYY-MM-DD",
        help = "Return date (auto-sets round-trip)",
        long_help = "Return date in YYYY-MM-DD format. Automatically creates a return leg \
            and sets trip type to round-trip. Results list outbound flights, priced for \
            the whole round trip."
    )]
    return_date: Option<String>,

    #[arg(
        long,
        default_value = "one-way",
        value_name = "TYPE",
        help = "Trip type [one-way, round-trip, multi-city]"
    )]
    trip: String,

    #[command(flatten)]
    filters: FilterArgs,

    #[command(flatten)]
    output: OutputArgs,

    #[arg(
        long,
        default_value = "en",
        value_name = "CODE",
        help_heading = "Output",
        help = "Language code (e.g. en, de, ja)"
    )]
    lang: String,

    #[arg(long, help_heading = "Output", help = "Open results in Google Flights")]
    open: bool,

    #[arg(long, help_heading = "Output", help = "Output Google Flights URL only (for AI agents)")]
    url: bool,

    #[command(flatten)]
    net: NetArgs,
}

#[derive(clap::Args)]
struct DatesArgs {
    #[arg(
        short, long,
        value_name = "IATA",
        help = "Departure airport code",
        long_help = "Departure airport IATA code (3 letters, e.g. HEL, JFK). \
            City codes like LON aren't supported for date searches."
    )]
    from: String,

    #[arg(
        short, long,
        value_name = "IATA",
        help = "Arrival airport code",
        long_help = "Arrival airport IATA code (3 letters, e.g. BCN, LHR). \
            City codes like NYC aren't supported for date searches."
    )]
    to: String,

    #[arg(
        short, long,
        value_name = "RANGE",
        help = "Month (YYYY-MM) or date range (YYYY-MM-DD..YYYY-MM-DD)",
        long_help = "Departure dates to price: a whole month as YYYY-MM (e.g. 2026-03), \
            a range as YYYY-MM-DD..YYYY-MM-DD (e.g. 2026-03-01..2026-04-15), or a single \
            YYYY-MM-DD. Dates already in the past are skipped."
    )]
    date: String,

    #[arg(
        long,
        value_name = "DAYS",
        help = "Round trips of this many days (omit for one-way)"
    )]
    stay: Option<u32>,

    #[command(flatten)]
    filters: FilterArgs,

    #[command(flatten)]
    output: OutputArgs,

    #[command(flatten)]
    net: NetArgs,
}

#[derive(clap::Args)]
#[command(next_help_heading = "Filters")]
struct FilterArgs {
    #[arg(
        long,
        default_value = "economy",
        value_name = "CLASS",
        help = "Seat class [economy, premium-economy, business, first]"
    )]
    seat: String,

    #[arg(
        long,
        value_name = "N",
        help = "Maximum number of stops (0 = nonstop only)"
    )]
    max_stops: Option<u32>,

    #[arg(
        long,
        value_name = "AA,DL,...",
        help = "Filter airlines (comma-separated IATA codes)"
    )]
    airlines: Option<String>,

    #[arg(long, default_value = "1", value_name = "N", help = "Number of adult passengers")]
    adults: u32,

    #[arg(long, default_value = "0", value_name = "N", help = "Number of child passengers (2-11)")]
    children: u32,

    #[arg(long, default_value = "0", value_name = "N", help = "Infants with own seat (under 2)")]
    infants_in_seat: u32,

    #[arg(long, default_value = "0", value_name = "N", help = "Infants on adult's lap (under 2)")]
    infants_on_lap: u32,
}

impl FilterArgs {
    fn to_filters(&self, language: &str, currency: &str) -> Result<Filters, FlightError> {
        Ok(Filters {
            max_stops: self.max_stops,
            airlines: self.airlines.as_deref().map(parse_codes),
            passengers: Passengers {
                adults: self.adults,
                children: self.children,
                infants_in_seat: self.infants_in_seat,
                infants_on_lap: self.infants_on_lap,
            },
            seat: Seat::from_str_loose(&self.seat)?,
            language: language.to_string(),
            currency: currency.to_uppercase(),
        })
    }
}

#[derive(clap::Args)]
#[command(next_help_heading = "Output")]
struct OutputArgs {
    #[arg(long, value_name = "N", help = "Show only the N cheapest results")]
    top: Option<usize>,

    #[arg(long, help = "One line per result (recommended for scripts and AI agents)")]
    compact: bool,

    #[arg(long, help = "Output as JSON")]
    json: bool,

    #[arg(long, help = "Output as pretty-printed JSON")]
    pretty: bool,

    #[arg(
        long,
        default_value = "USD",
        value_name = "CODE",
        help = "Currency code (e.g. USD, EUR, JPY)"
    )]
    currency: String,
}

impl OutputArgs {
    fn is_json(&self) -> bool {
        self.json || self.pretty
    }

    fn to_json(&self, value: &impl Serialize) -> String {
        if self.pretty {
            serde_json::to_string_pretty(value).unwrap()
        } else {
            serde_json::to_string(value).unwrap()
        }
    }
}

#[derive(clap::Args)]
#[command(next_help_heading = "Connection")]
struct NetArgs {
    #[arg(long, value_name = "URL", help = "HTTP or SOCKS5 proxy")]
    proxy: Option<String>,

    #[arg(long, default_value = "30", value_name = "SECS", help = "Request timeout")]
    timeout: u64,
}

impl NetArgs {
    fn fetch_options(&self) -> FetchOptions {
        FetchOptions {
            proxy: self.proxy.clone(),
            timeout: self.timeout,
        }
    }
}

fn error_code(err: &FlightError) -> i32 {
    match err {
        FlightError::InvalidAirport(_)
        | FlightError::InvalidDate(_)
        | FlightError::Validation(_)
        | FlightError::Rejected(Some(INVALID_ARGUMENT)) => 2,
        FlightError::Timeout
        | FlightError::ConnectionFailed(_)
        | FlightError::DnsResolution(_)
        | FlightError::TlsError(_)
        | FlightError::ProxyError(_) => 3,
        FlightError::RateLimited | FlightError::Blocked(_) | FlightError::Rejected(_) => 4,
        FlightError::HttpStatus(_) => 5,
        FlightError::ScriptTagNotFound | FlightError::JsParse(_) => 6,
    }
}

fn error_kind(err: &FlightError) -> &'static str {
    match err {
        FlightError::InvalidAirport(_) => "invalid_airport",
        FlightError::InvalidDate(_) => "invalid_date",
        FlightError::Validation(_) => "validation_error",
        FlightError::Timeout => "timeout",
        FlightError::ConnectionFailed(_) => "connection_failed",
        FlightError::DnsResolution(_) => "dns_error",
        FlightError::TlsError(_) => "tls_error",
        FlightError::ProxyError(_) => "proxy_error",
        FlightError::RateLimited => "rate_limited",
        FlightError::Blocked(_) => "blocked",
        FlightError::Rejected(_) => "rejected",
        FlightError::HttpStatus(_) => "http_error",
        FlightError::ScriptTagNotFound => "parse_error",
        FlightError::JsParse(_) => "parse_error",
    }
}

fn error_json(err: &FlightError) -> Value {
    serde_json::json!({
        "error": {
            "kind": error_kind(err),
            "message": err.to_string(),
        }
    })
}

/// Writes a line to stdout, exiting quietly once the reader has gone away, as when
/// piping into `head`.
fn emit(text: impl std::fmt::Display) {
    use std::io::Write;

    if let Err(e) = writeln!(std::io::stdout().lock(), "{text}") {
        if e.kind() == std::io::ErrorKind::BrokenPipe {
            process::exit(0);
        }
        eprintln!("error: failed to write output: {e}");
        process::exit(1);
    }
}

fn die(err: &FlightError, json_mode: bool) -> ! {
    if json_mode {
        emit(error_json(err));
    } else {
        eprintln!("error: {err}");
    }
    process::exit(error_code(err));
}

fn required<'a>(value: &'a Option<String>, flag: &str) -> Result<&'a str, FlightError> {
    value
        .as_deref()
        .ok_or_else(|| FlightError::Validation(format!("{flag} is required (or use --leg)")))
}

fn determine_trip(args: &SearchArgs) -> Result<TripType, FlightError> {
    let trip = TripType::from_str_loose(&args.trip)?;
    if args.return_date.is_some() {
        return Ok(TripType::RoundTrip);
    }
    if args.leg.len() >= 2 && trip == TripType::OneWay {
        return Ok(TripType::MultiCity);
    }
    Ok(trip)
}

fn parse_leg(leg: &str) -> Result<(String, String, String), FlightError> {
    match leg.split_whitespace().collect::<Vec<_>>()[..] {
        [date, from, to] => Ok((date.to_string(), from.to_uppercase(), to.to_uppercase())),
        _ => Err(FlightError::Validation(format!(
            "--leg must be \"DATE FROM TO\", got: \"{leg}\""
        ))),
    }
}

/// One validated query per destination, or a single query for `--leg` itineraries.
fn build_queries(args: &SearchArgs) -> Result<Vec<(String, QueryParams)>, FlightError> {
    let filters = args.filters.to_filters(&args.lang, &args.output.currency)?;
    let trip = determine_trip(args)?;

    let queries = if !args.leg.is_empty() {
        if args.to.as_deref().is_some_and(|t| t.contains(',')) {
            return Err(FlightError::Validation(
                "--leg cannot be used with comma-separated -t destinations".into(),
            ));
        }
        let legs = args
            .leg
            .iter()
            .map(|leg| parse_leg(leg))
            .collect::<Result<Vec<_>, _>>()?;
        let legs: Vec<(&str, &str, &str)> = legs
            .iter()
            .map(|(date, from, to)| (date.as_str(), from.as_str(), to.as_str()))
            .collect();
        vec![(String::new(), QueryParams::new(&legs, trip, &filters))]
    } else {
        let from = required(&args.from, "--from")?.trim().to_uppercase();
        let destinations = parse_codes(required(&args.to, "--to")?);
        if destinations.is_empty() {
            return Err(FlightError::Validation("--to is required (or use --leg)".into()));
        }
        let date = required(&args.date, "--date")?;

        let mut queries =
            route_queries(&from, &destinations, date, args.return_date.as_deref(), &filters);
        for (_, query) in &mut queries {
            query.trip = trip.clone();
        }
        queries
    };

    for (_, query) in &queries {
        query.validate()?;
    }
    Ok(queries)
}

fn print_result(result: &SearchResult, args: &SearchArgs, round_trip: bool) {
    let currency = args.output.currency.to_uppercase();
    if args.output.compact {
        emit(table::compact(result, &currency, round_trip));
    } else if args.output.is_json() {
        emit(args.output.to_json(result));
    } else {
        emit(table::render(result, &currency, round_trip));
    }
}

fn print_multi_result(
    outcomes: &[(String, Result<SearchResult, FlightError>)],
    args: &SearchArgs,
    round_trip: bool,
) {
    let currency = args.output.currency.to_uppercase();
    if args.output.compact {
        emit(table::sections(outcomes, "\n", |r| {
            table::compact(r, &currency, round_trip)
        }));
    } else if args.output.is_json() {
        let by_destination: serde_json::Map<String, Value> = outcomes
            .iter()
            .map(|(dest, outcome)| {
                let value = match outcome {
                    Ok(result) => serde_json::to_value(result).unwrap(),
                    Err(e) => error_json(e),
                };
                (dest.clone(), value)
            })
            .collect();
        emit(args.output.to_json(&by_destination));
    } else {
        emit(table::sections(outcomes, "\n\n", |r| {
            table::render(r, &currency, round_trip)
        }));
    }
}

async fn run_search(args: SearchArgs) {
    let json_mode = args.output.is_json();
    let queries = build_queries(&args).unwrap_or_else(|e| die(&e, json_mode));

    if args.url || args.open {
        for (_, query) in &queries {
            let url = flyr::generate_browser_url(query);
            if args.url {
                emit(&url);
                continue;
            }
            emit(format!("Opening: {url}"));
            if let Err(e) = open::that(&url) {
                die(
                    &FlightError::Validation(format!("failed to open browser: {e}")),
                    json_mode,
                );
            }
        }
        return;
    }

    let options = args.net.fetch_options();
    let round_trip = queries.iter().all(|(_, q)| q.trip == TripType::RoundTrip);

    if let [(_, query)] = &queries[..] {
        match flyr::search(query, &options).await {
            Ok(mut result) => {
                if let Some(n) = args.output.top {
                    result.keep_cheapest(n);
                }
                print_result(&result, &args, round_trip);
            }
            Err(e) => die(&e, json_mode),
        }
        return;
    }

    let mut outcomes = flyr::search_all(queries, &options).await;
    if let Some(n) = args.output.top {
        for result in outcomes.iter_mut().filter_map(|(_, o)| o.as_mut().ok()) {
            result.keep_cheapest(n);
        }
    }
    print_multi_result(&outcomes, &args, round_trip);

    if let Some(err) = all_failed(&outcomes) {
        process::exit(error_code(err));
    }
}

/// The first error when every destination failed; partial failures are reported inline
/// and still exit successfully.
fn all_failed(outcomes: &[(String, Result<SearchResult, FlightError>)]) -> Option<&FlightError> {
    let mut errors = outcomes.iter().map(|(_, o)| o.as_ref().err());
    let first = errors.next()??;
    errors.all(|e| e.is_some()).then_some(first)
}

fn build_date_query(args: &DatesArgs) -> Result<DateQuery, FlightError> {
    let filters = args.filters.to_filters("en", &args.output.currency)?;
    let (start, end) = parse_date_range(&args.date)?;
    let query = DateQuery {
        from_airport: args.from.trim().to_uppercase(),
        to_airport: args.to.trim().to_uppercase(),
        start,
        end,
        stay_days: args.stay,
        filters,
    };
    query.validate()?;
    Ok(query)
}

async fn run_dates(args: DatesArgs) {
    let json_mode = args.output.is_json();
    let query = build_date_query(&args).unwrap_or_else(|e| die(&e, json_mode));

    let mut dates = match flyr::search_dates(&query, &args.net.fetch_options()).await {
        Ok(dates) => dates,
        Err(e) => die(&e, json_mode),
    };
    if let Some(n) = args.output.top {
        keep_cheapest_dates(&mut dates, n);
    }

    let currency = &query.filters.currency;
    if args.output.compact {
        emit(table::compact_dates(&dates, currency));
    } else if json_mode {
        emit(args.output.to_json(&dates));
    } else {
        emit(table::render_dates(&dates, currency));
    }
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Search(args) => run_search(args).await,
        Commands::Dates(args) => run_dates(args).await,
        Commands::Mcp => flyr::mcp::run().await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn outcome(ok: bool) -> (String, Result<SearchResult, FlightError>) {
        let result = if ok { Ok(SearchResult::default()) } else { Err(FlightError::RateLimited) };
        (String::new(), result)
    }

    #[test]
    fn exit_fails_only_when_every_destination_fails() {
        assert!(all_failed(&[outcome(false), outcome(false)]).is_some());
        assert!(all_failed(&[outcome(false), outcome(true)]).is_none());
        assert!(all_failed(&[outcome(true), outcome(false)]).is_none());
        assert!(all_failed(&[]).is_none());
    }
}
