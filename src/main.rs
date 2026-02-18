use std::collections::BTreeMap;
use std::process;

use clap::Parser;
use tokio::task::JoinSet;

use flyr::error::FlightError;
use flyr::fetch::FetchOptions;
use flyr::model::SearchResult;
use flyr::query::{FlightLeg, Passengers, QueryParams, Seat, SearchQuery, TripType};
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
            Required unless using --leg."
    )]
    from: Option<String>,

    #[arg(
        short, long,
        value_name = "IATA",
        help = "Arrival airport code (comma-separate for multi-destination)",
        long_help = "Arrival airport IATA code (3 letters, e.g. LHR, BCN, NRT). \
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
            and sets trip type to round-trip."
    )]
    return_date: Option<String>,

    #[arg(
        long,
        default_value = "one-way",
        value_name = "TYPE",
        help = "Trip type [one-way, round-trip, multi-city]"
    )]
    trip: String,

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

    #[arg(long, default_value = "en", value_name = "CODE", help = "Language code (e.g. en, de, ja)")]
    lang: String,

    #[arg(long, default_value = "USD", value_name = "CODE", help = "Currency code (e.g. USD, EUR, JPY)")]
    currency: String,

    #[arg(long, value_name = "N", help = "Show only the N cheapest results")]
    top: Option<usize>,

    #[arg(long, help = "One-line-per-flight output (recommended for scripts and AI agents)")]
    compact: bool,

    #[arg(long, help = "Output as JSON")]
    json: bool,

    #[arg(long, help = "Output as pretty-printed JSON")]
    pretty: bool,

    #[arg(long, help = "Open results in Google Flights")]
    open: bool,

    #[arg(long, help = "Output Google Flights URL only (for AI agents)")]
    url: bool,

    #[arg(long, value_name = "URL", help = "HTTP or SOCKS5 proxy")]
    proxy: Option<String>,

    #[arg(long, default_value = "30", value_name = "SECS", help = "Request timeout")]
    timeout: u64,
}

fn is_json(args: &SearchArgs) -> bool {
    args.json || args.pretty
}

fn apply_top(result: &mut SearchResult, n: usize) {
    result
        .flights
        .sort_by_key(|f| f.price.unwrap_or(i64::MAX));
    result.flights.truncate(n);
}

fn open_browser(query_params: &QueryParams, json_mode: bool) -> ! {
    let url = flyr::generate_browser_url(query_params);
    println!("Opening: {url}");
    if let Err(e) = open::that(&url) {
        die(
            &FlightError::Validation(format!("failed to open browser: {e}")),
            json_mode,
        );
    }
    std::process::exit(0);
}

fn error_code(err: &FlightError) -> i32 {
    match err {
        FlightError::InvalidAirport(_)
        | FlightError::InvalidDate(_)
        | FlightError::Validation(_) => 2,
        FlightError::Timeout
        | FlightError::ConnectionFailed(_)
        | FlightError::DnsResolution(_)
        | FlightError::TlsError(_)
        | FlightError::ProxyError(_) => 3,
        FlightError::RateLimited | FlightError::Blocked(_) => 4,
        FlightError::HttpStatus(_) => 5,
        FlightError::ScriptTagNotFound | FlightError::JsParse(_) => 6,
        FlightError::NoResults => 0,
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
        FlightError::HttpStatus(_) => "http_error",
        FlightError::ScriptTagNotFound => "parse_error",
        FlightError::JsParse(_) => "parse_error",
        FlightError::NoResults => "no_results",
    }
}

fn die(err: &FlightError, json_mode: bool) -> ! {
    if json_mode {
        let json = serde_json::json!({
            "error": {
                "kind": error_kind(err),
                "message": err.to_string(),
            }
        });
        println!("{}", serde_json::to_string(&json).unwrap());
    } else {
        eprintln!("error: {err}");
    }
    process::exit(error_code(err));
}

fn build_legs(args: &SearchArgs) -> Result<Vec<FlightLeg>, FlightError> {
    let airlines: Option<Vec<String>> = args
        .airlines
        .as_ref()
        .map(|s| s.split(',').map(|a| a.trim().to_uppercase()).collect());

    if !args.leg.is_empty() {
        let mut legs = Vec::new();
        for leg_str in &args.leg {
            let parts: Vec<&str> = leg_str.split_whitespace().collect();
            if parts.len() != 3 {
                return Err(FlightError::Validation(format!(
                    "--leg must be \"DATE FROM TO\", got: \"{leg_str}\""
                )));
            }
            legs.push(FlightLeg {
                date: parts[0].to_string(),
                from_airport: parts[1].to_uppercase(),
                to_airport: parts[2].to_uppercase(),
                max_stops: args.max_stops,
                airlines: airlines.clone(),
            });
        }
        return Ok(legs);
    }

    let from = args
        .from
        .as_ref()
        .ok_or_else(|| FlightError::Validation("--from is required (or use --leg)".into()))?;
    let to = args
        .to
        .as_ref()
        .ok_or_else(|| FlightError::Validation("--to is required (or use --leg)".into()))?;
    let date = args
        .date
        .as_ref()
        .ok_or_else(|| FlightError::Validation("--date is required (or use --leg)".into()))?;

    let mut legs = vec![FlightLeg {
        date: date.clone(),
        from_airport: from.to_uppercase(),
        to_airport: to.to_uppercase(),
        max_stops: args.max_stops,
        airlines: airlines.clone(),
    }];

    if let Some(ref ret_date) = args.return_date {
        legs.push(FlightLeg {
            date: ret_date.clone(),
            from_airport: to.to_uppercase(),
            to_airport: from.to_uppercase(),
            max_stops: args.max_stops,
            airlines: airlines.clone(),
        });
    }

    Ok(legs)
}

fn determine_trip(args: &SearchArgs) -> String {
    if args.return_date.is_some() {
        return "round-trip".to_string();
    }
    if args.leg.len() >= 2 && args.trip == "one-way" {
        return "multi-city".to_string();
    }
    args.trip.clone()
}

fn print_compact(result: &SearchResult, currency: &str) {
    for flight in &result.flights {
        let price = table::format_price(flight.price, currency);

        let route: Vec<&str> = std::iter::once(
            flight
                .segments
                .first()
                .map(|s| s.from_airport.code.as_str())
                .unwrap_or("?"),
        )
        .chain(flight.segments.iter().map(|s| s.to_airport.code.as_str()))
        .collect();
        let route_str = route.join(">");

        let duration = if flight.segments.is_empty() {
            "—".to_string()
        } else {
            let total: u32 = flight.segments.iter().map(|s| s.duration_minutes).sum();
            format!("{}h{:02}m", total / 60, total % 60)
        };

        let stops = if flight.segments.len() <= 1 {
            "nonstop".to_string()
        } else {
            let n = flight.segments.len() - 1;
            let codes: Vec<&str> = flight.segments[..n]
                .iter()
                .map(|s| s.to_airport.code.as_str())
                .collect();
            format!("{n} stop {}", codes.join(","))
        };

        let airlines = flight.airlines.join(", ");

        let depart = flight.segments.first();
        let arrive = flight.segments.last();
        let time_str = match (depart, arrive) {
            (Some(d), Some(a)) => format!(
                "{}{:02} {:02}:{:02}>{:02}:{:02}",
                month_abbr(d.departure.month),
                d.departure.day,
                d.departure.hour,
                d.departure.minute,
                a.arrival.hour,
                a.arrival.minute,
            ),
            _ => "—".to_string(),
        };

        println!("{price} | {route_str} | {duration} | {stops} | {airlines} | {time_str}");
    }
}

fn month_abbr(m: u32) -> &'static str {
    match m {
        1 => "Jan",
        2 => "Feb",
        3 => "Mar",
        4 => "Apr",
        5 => "May",
        6 => "Jun",
        7 => "Jul",
        8 => "Aug",
        9 => "Sep",
        10 => "Oct",
        11 => "Nov",
        12 => "Dec",
        _ => "???",
    }
}

fn print_result(result: &SearchResult, args: &SearchArgs) {
    if args.compact {
        if result.flights.is_empty() {
            println!("No flights found.");
            return;
        }
        print_compact(result, &args.currency);
    } else if is_json(args) {
        let output = if args.pretty {
            serde_json::to_string_pretty(result).unwrap()
        } else {
            serde_json::to_string(result).unwrap()
        };
        println!("{output}");
    } else {
        if result.flights.is_empty() {
            println!("No flights found.");
            return;
        }
        println!("{}", table::render(result, &args.currency));
    }
}

fn is_multi_dest(args: &SearchArgs) -> bool {
    args.to.as_ref().is_some_and(|t| t.contains(','))
}

fn parse_destinations(args: &SearchArgs) -> Vec<String> {
    args.to
        .as_ref()
        .map(|t| {
            t.split(',')
                .map(|s| s.trim().to_uppercase())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

fn build_base_params(
    args: &SearchArgs,
) -> Result<(Passengers, Seat, TripType, Option<Vec<String>>), FlightError> {
    let trip_str = determine_trip(args);
    let trip = TripType::from_str_loose(&trip_str)?;
    let seat = Seat::from_str_loose(&args.seat)?;
    let passengers = Passengers {
        adults: args.adults,
        children: args.children,
        infants_in_seat: args.infants_in_seat,
        infants_on_lap: args.infants_on_lap,
    };
    let airlines: Option<Vec<String>> = args
        .airlines
        .as_ref()
        .map(|s| s.split(',').map(|a| a.trim().to_uppercase()).collect());
    Ok((passengers, seat, trip, airlines))
}

fn print_multi_result(
    results: &BTreeMap<String, SearchResult>,
    args: &SearchArgs,
) {
    if args.compact {
        for (dest, result) in results {
            println!("=== {dest} ===");
            if result.flights.is_empty() {
                println!("No flights found.");
            } else {
                print_compact(result, &args.currency);
            }
        }
    } else if is_json(args) {
        let output = if args.pretty {
            serde_json::to_string_pretty(results).unwrap()
        } else {
            serde_json::to_string(results).unwrap()
        };
        println!("{output}");
    } else {
        for (dest, result) in results {
            println!("=== {dest} ===");
            if result.flights.is_empty() {
                println!("No flights found.");
            } else {
                println!("{}", table::render(result, &args.currency));
            }
            println!();
        }
    }
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Mcp => flyr::mcp::run().await,
        Commands::Search(args) => {
            let json_mode = is_json(&args);

            if is_multi_dest(&args) {
                if !args.leg.is_empty() {
                    die(
                        &FlightError::Validation(
                            "--leg cannot be used with comma-separated -t destinations".into(),
                        ),
                        json_mode,
                    );
                }

                let from = match args.from.as_ref() {
                    Some(f) => f.to_uppercase(),
                    None => die(
                        &FlightError::Validation("--from is required (or use --leg)".into()),
                        json_mode,
                    ),
                };
                let date = match args.date.as_ref() {
                    Some(d) => d.clone(),
                    None => die(
                        &FlightError::Validation("--date is required (or use --leg)".into()),
                        json_mode,
                    ),
                };

                let (passengers, seat, _trip, airlines) = match build_base_params(&args) {
                    Ok(p) => p,
                    Err(e) => die(&e, json_mode),
                };

                let destinations = parse_destinations(&args);
                let fetch_options = FetchOptions {
                    proxy: args.proxy.clone(),
                    timeout: args.timeout,
                };

                if args.open {
                    let from = match args.from.as_ref() {
                        Some(f) => f.to_uppercase(),
                        None => die(
                            &FlightError::Validation("--from is required (or use --leg)".into()),
                            json_mode,
                        ),
                    };
                    let date = match args.date.as_ref() {
                        Some(d) => d.clone(),
                        None => die(
                            &FlightError::Validation("--date is required (or use --leg)".into()),
                            json_mode,
                        ),
                    };

                    let trip = if args.return_date.is_some() {
                        TripType::RoundTrip
                    } else {
                        TripType::OneWay
                    };

                    for dest in &destinations {
                        let mut legs = vec![FlightLeg {
                            date: date.clone(),
                            from_airport: from.clone(),
                            to_airport: dest.clone(),
                            max_stops: args.max_stops,
                            airlines: airlines.clone(),
                        }];

                        if args.return_date.is_some() {
                            legs.push(FlightLeg {
                                date: args.return_date.clone().unwrap(),
                                from_airport: dest.clone(),
                                to_airport: from.clone(),
                                max_stops: args.max_stops,
                                airlines: airlines.clone(),
                            });
                        }

                        let query_params = QueryParams {
                            legs,
                            passengers: passengers.clone(),
                            seat: seat.clone(),
                            trip: trip.clone(),
                            language: args.lang.clone(),
                            currency: args.currency.clone(),
                        };

                        let url = flyr::generate_browser_url(&query_params);
                        if args.url {
                            println!("{url}");
                        } else {
                            println!("Opening: {url}");
                            let _ = open::that(&url);
                        }
                    }
                    return;
                }

                let mut join_set = JoinSet::new();

                for dest in &destinations {
                    let mut legs = vec![FlightLeg {
                        date: date.clone(),
                        from_airport: from.clone(),
                        to_airport: dest.clone(),
                        max_stops: args.max_stops,
                        airlines: airlines.clone(),
                    }];

                    let trip = if args.return_date.is_some() {
                        legs.push(FlightLeg {
                            date: args.return_date.clone().unwrap(),
                            from_airport: dest.clone(),
                            to_airport: from.clone(),
                            max_stops: args.max_stops,
                            airlines: airlines.clone(),
                        });
                        TripType::RoundTrip
                    } else {
                        TripType::OneWay
                    };

                    let query_params = QueryParams {
                        legs,
                        passengers: passengers.clone(),
                        seat: seat.clone(),
                        trip,
                        language: args.lang.clone(),
                        currency: args.currency.clone(),
                    };

                if args.open {
                    open_browser(&query_params, json_mode);
                }

                if args.url {
                    let url = flyr::generate_browser_url(&query_params);
                    println!("{url}");
                    std::process::exit(0);
                }

                if let Err(e) = query_params.validate() {
                        die(&e, json_mode);
                    }

                    let opts = fetch_options.clone();
                    let dest_code = dest.clone();
                    join_set.spawn(async move {
                        let result =
                            flyr::search(SearchQuery::Structured(query_params), opts).await;
                        (dest_code, result)
                    });
                }

                let mut results: BTreeMap<String, SearchResult> = BTreeMap::new();

                while let Some(join_result) = join_set.join_next().await {
                    let (dest_code, search_result) = join_result.unwrap();
                    match search_result {
                        Ok(mut result) => {
                            if let Some(n) = args.top {
                                apply_top(&mut result, n);
                            }
                            results.insert(dest_code, result);
                        }
                        Err(e) => {
                            if json_mode {
                                let mut error_result = SearchResult::default();
                                error_result.flights = vec![];
                                results.insert(dest_code.clone(), error_result);
                                eprintln!("warning: {dest_code}: {e}");
                            } else {
                                eprintln!("error: {dest_code}: {e}");
                            }
                        }
                    }
                }

                print_multi_result(&results, &args);
            } else {
                let legs = match build_legs(&args) {
                    Ok(l) => l,
                    Err(e) => die(&e, json_mode),
                };

                let trip_str = determine_trip(&args);
                let trip = match TripType::from_str_loose(&trip_str) {
                    Ok(t) => t,
                    Err(e) => die(&e, json_mode),
                };
                let seat = match Seat::from_str_loose(&args.seat) {
                    Ok(s) => s,
                    Err(e) => die(&e, json_mode),
                };

                let passengers = Passengers {
                    adults: args.adults,
                    children: args.children,
                    infants_in_seat: args.infants_in_seat,
                    infants_on_lap: args.infants_on_lap,
                };

                let query_params = QueryParams {
                    legs,
                    passengers,
                    seat,
                    trip,
                    language: args.lang.clone(),
                    currency: args.currency.clone(),
                };

                if args.open {
                    open_browser(&query_params, json_mode);
                }

                if args.url {
                    let url = flyr::generate_browser_url(&query_params);
                    println!("{url}");
                    std::process::exit(0);
                }

                if let Err(e) = query_params.validate() {
                    die(&e, json_mode);
                }

                let fetch_options = FetchOptions {
                    proxy: args.proxy.clone(),
                    timeout: args.timeout,
                };

                match flyr::search(SearchQuery::Structured(query_params), fetch_options).await {
                    Ok(mut result) => {
                        if let Some(n) = args.top {
                            apply_top(&mut result, n);
                        }
                        print_result(&result, &args);
                    }
                    Err(e) => die(&e, json_mode),
                }
            }
        }
    }
}
