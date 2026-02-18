use std::collections::BTreeMap;

use rmcp::handler::server::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::*;
use rmcp::schemars;
use rmcp::{tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler, ServiceExt};
use serde::Deserialize;
use tokio::task::JoinSet;

use crate::fetch::FetchOptions;
use crate::model::SearchResult;
use crate::query::{FlightLeg, Passengers, QueryParams, Seat, SearchQuery, TripType};

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct SearchArgs {
    #[schemars(
        description = "Departure airport IATA code, exactly 3 uppercase letters. Example: HEL, JFK, LAX"
    )]
    from: String,
    #[schemars(
        description = "Arrival airport IATA code(s). Comma-separate for multi-destination. Examples: BCN or BCN,ATH,AYT"
    )]
    to: String,
    #[schemars(description = "Departure date in YYYY-MM-DD format. Example: 2026-03-01")]
    date: String,
    #[schemars(
        description = "Return date in YYYY-MM-DD for round-trip. Auto-sets trip type to round-trip"
    )]
    return_date: Option<String>,
    #[schemars(
        description = "One of: economy, premium-economy, business, first. Default: economy"
    )]
    seat: Option<String>,
    #[schemars(description = "Maximum stops. 0 = nonstop only. Omit for any number of stops")]
    max_stops: Option<u32>,
    #[schemars(description = "Filter airlines by IATA code, comma-separated. Example: AY,IB")]
    airlines: Option<String>,
    #[schemars(description = "Adult passengers (12+). Default: 1")]
    adults: Option<u32>,
    #[schemars(description = "Child passengers (2-11). Default: 0")]
    children: Option<u32>,
    #[schemars(description = "Infants with own seat (under 2). Default: 0")]
    infants_in_seat: Option<u32>,
    #[schemars(description = "Infants on adult's lap (under 2). Default: 0")]
    infants_on_lap: Option<u32>,
    #[schemars(description = "Currency code. Examples: USD, EUR, JPY. Default: USD")]
    currency: Option<String>,
    #[schemars(description = "Return only N cheapest results")]
    top: Option<usize>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct GetUrlArgs {
    #[schemars(
        description = "Departure airport IATA code, exactly 3 uppercase letters. Example: HEL, JFK, LAX"
    )]
    from: String,
    #[schemars(
        description = "Arrival airport IATA code(s). Comma-separate for multi-destination. Examples: BCN or BCN,ATH,AYT"
    )]
    to: String,
    #[schemars(description = "Departure date in YYYY-MM-DD format. Example: 2026-03-01")]
    date: String,
    #[schemars(
        description = "Return date in YYYY-MM-DD for round-trip. Auto-sets trip type to round-trip"
    )]
    return_date: Option<String>,
    #[schemars(
        description = "One of: economy, premium-economy, business, first. Default: economy"
    )]
    seat: Option<String>,
    #[schemars(description = "Adult passengers (12+). Default: 1")]
    adults: Option<u32>,
    #[schemars(description = "Currency code. Examples: USD, EUR, JPY. Default: USD")]
    currency: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct OpenUrlArgs {
    #[schemars(description = "URL to open. Must start with http:// or https://")]
    url: String,
}

fn parse_legs(
    from: &str,
    to: &str,
    date: &str,
    return_date: Option<&str>,
    max_stops: Option<u32>,
    airlines: Option<&str>,
) -> (Vec<FlightLeg>, TripType) {
    let parsed_airlines: Option<Vec<String>> = airlines
        .map(|s| s.split(',').map(|a| a.trim().to_uppercase()).collect());

    let mut legs = vec![FlightLeg {
        date: date.to_string(),
        from_airport: from.to_uppercase(),
        to_airport: to.to_uppercase(),
        max_stops,
        airlines: parsed_airlines.clone(),
    }];

    let trip = if let Some(ret) = return_date {
        legs.push(FlightLeg {
            date: ret.to_string(),
            from_airport: to.to_uppercase(),
            to_airport: from.to_uppercase(),
            max_stops,
            airlines: parsed_airlines,
        });
        TripType::RoundTrip
    } else {
        TripType::OneWay
    };

    (legs, trip)
}

fn tool_error(msg: impl Into<String>) -> Result<CallToolResult, McpError> {
    Ok(CallToolResult::error(vec![Content::text(msg.into())]))
}

fn apply_top(result: &mut SearchResult, n: usize) {
    result
        .flights
        .sort_by_key(|f| f.price.unwrap_or(i64::MAX));
    result.flights.truncate(n);
}

#[derive(Debug, Clone)]
struct FlyrMcp {
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl FlyrMcp {
    fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        description = "Search for flights and return results as JSON. Searches Google Flights for available flights between airports on specific dates. Returns flight options with prices, airlines, duration, stops, and schedule. Comma-separate 'to' for multi-destination comparison. To open results in browser: call flyr_get_url with the same parameters, then call open_url with the returned URL."
    )]
    async fn flyr_search(
        &self,
        Parameters(args): Parameters<SearchArgs>,
    ) -> Result<CallToolResult, McpError> {
        let is_multi = args.to.contains(',');

        if is_multi {
            let from = args.from.to_uppercase();
            let date = args.date;

            let seat = match args
                .seat
                .as_deref()
                .map(Seat::from_str_loose)
                .transpose()
            {
                Ok(s) => s.unwrap_or(Seat::Economy),
                Err(e) => return tool_error(e.to_string()),
            };

            let passengers = Passengers {
                adults: args.adults.unwrap_or(1),
                children: args.children.unwrap_or(0),
                infants_in_seat: args.infants_in_seat.unwrap_or(0),
                infants_on_lap: args.infants_on_lap.unwrap_or(0),
            };

            let airlines: Option<Vec<String>> = args
                .airlines
                .as_ref()
                .map(|s| s.split(',').map(|a| a.trim().to_uppercase()).collect());

            let currency = args.currency.unwrap_or_else(|| "USD".into());

            let destinations: Vec<String> = args
                .to
                .split(',')
                .map(|s| s.trim().to_uppercase())
                .filter(|s| !s.is_empty())
                .collect();

            let mut join_set = JoinSet::new();

            for dest in &destinations {
                let mut legs = vec![FlightLeg {
                    date: date.clone(),
                    from_airport: from.clone(),
                    to_airport: dest.clone(),
                    max_stops: args.max_stops,
                    airlines: airlines.clone(),
                }];

                let trip = if let Some(ref ret) = args.return_date {
                    legs.push(FlightLeg {
                        date: ret.clone(),
                        from_airport: dest.clone(),
                        to_airport: from.clone(),
                        max_stops: args.max_stops,
                        airlines: airlines.clone(),
                    });
                    TripType::RoundTrip
                } else {
                    TripType::OneWay
                };

                let params = QueryParams {
                    legs,
                    passengers: passengers.clone(),
                    seat: seat.clone(),
                    trip,
                    language: "en".into(),
                    currency: currency.clone(),
                };

                if let Err(e) = params.validate() {
                    return tool_error(format!("{dest}: {e}"));
                }

                let dest_code = dest.clone();
                let top = args.top;
                join_set.spawn(async move {
                    let result =
                        crate::search(SearchQuery::Structured(params), FetchOptions::default())
                            .await;
                    (dest_code, result, top)
                });
            }

            let mut results: BTreeMap<String, SearchResult> = BTreeMap::new();
            while let Some(join_result) = join_set.join_next().await {
                let (dest_code, search_result, top) = join_result.unwrap();
                match search_result {
                    Ok(mut result) => {
                        if let Some(n) = top {
                            apply_top(&mut result, n);
                        }
                        results.insert(dest_code, result);
                    }
                    Err(e) => {
                        results.insert(dest_code.clone(), SearchResult::default());
                        eprintln!("warning: {dest_code}: {e}");
                    }
                }
            }

            let json = serde_json::to_string_pretty(&results).unwrap();
            Ok(CallToolResult::success(vec![Content::text(json)]))
        } else {
            let (legs, trip) = parse_legs(
                &args.from,
                &args.to,
                &args.date,
                args.return_date.as_deref(),
                args.max_stops,
                args.airlines.as_deref(),
            );

            let seat = match args
                .seat
                .as_deref()
                .map(Seat::from_str_loose)
                .transpose()
            {
                Ok(s) => s.unwrap_or(Seat::Economy),
                Err(e) => return tool_error(e.to_string()),
            };

            let passengers = Passengers {
                adults: args.adults.unwrap_or(1),
                children: args.children.unwrap_or(0),
                infants_in_seat: args.infants_in_seat.unwrap_or(0),
                infants_on_lap: args.infants_on_lap.unwrap_or(0),
            };

            let currency = args.currency.unwrap_or_else(|| "USD".into());

            let params = QueryParams {
                legs,
                passengers,
                seat,
                trip,
                language: "en".into(),
                currency,
            };

            if let Err(e) = params.validate() {
                return tool_error(e.to_string());
            }

            match crate::search(SearchQuery::Structured(params), FetchOptions::default()).await {
                Ok(mut result) => {
                    if let Some(n) = args.top {
                        apply_top(&mut result, n);
                    }
                    let json = serde_json::to_string_pretty(&result).unwrap();
                    Ok(CallToolResult::success(vec![Content::text(json)]))
                }
                Err(e) => tool_error(e.to_string()),
            }
        }
    }

    #[tool(
        description = "Generate a Google Flights URL for the given search parameters. This is the ONLY way to get a valid Google Flights URL. Returns an encoded URL that can be opened in a browser with open_url. NEVER construct Google Flights URLs manually -- always use this tool."
    )]
    async fn flyr_get_url(
        &self,
        Parameters(args): Parameters<GetUrlArgs>,
    ) -> Result<CallToolResult, McpError> {
        let is_multi = args.to.contains(',');

        if is_multi {
            let seat = match args
                .seat
                .as_deref()
                .map(Seat::from_str_loose)
                .transpose()
            {
                Ok(s) => s.unwrap_or(Seat::Economy),
                Err(e) => return tool_error(e.to_string()),
            };

            let passengers = Passengers {
                adults: args.adults.unwrap_or(1),
                ..Default::default()
            };

            let currency = args.currency.unwrap_or_else(|| "USD".into());

            let destinations: Vec<String> = args
                .to
                .split(',')
                .map(|s| s.trim().to_uppercase())
                .filter(|s| !s.is_empty())
                .collect();

            let mut urls = Vec::new();
            for dest in &destinations {
                let mut legs = vec![FlightLeg {
                    date: args.date.clone(),
                    from_airport: args.from.to_uppercase(),
                    to_airport: dest.clone(),
                    max_stops: None,
                    airlines: None,
                }];

                let trip = if let Some(ref ret) = args.return_date {
                    legs.push(FlightLeg {
                        date: ret.clone(),
                        from_airport: dest.clone(),
                        to_airport: args.from.to_uppercase(),
                        max_stops: None,
                        airlines: None,
                    });
                    TripType::RoundTrip
                } else {
                    TripType::OneWay
                };

                let params = QueryParams {
                    legs,
                    passengers: passengers.clone(),
                    seat: seat.clone(),
                    trip,
                    language: "en".into(),
                    currency: currency.clone(),
                };

                if let Err(e) = params.validate() {
                    return tool_error(format!("{dest}: {e}"));
                }

                urls.push(crate::generate_browser_url(&params));
            }

            Ok(CallToolResult::success(vec![Content::text(
                urls.join("\n"),
            )]))
        } else {
            let (legs, trip) = parse_legs(
                &args.from,
                &args.to,
                &args.date,
                args.return_date.as_deref(),
                None,
                None,
            );

            let seat = match args
                .seat
                .as_deref()
                .map(Seat::from_str_loose)
                .transpose()
            {
                Ok(s) => s.unwrap_or(Seat::Economy),
                Err(e) => return tool_error(e.to_string()),
            };

            let passengers = Passengers {
                adults: args.adults.unwrap_or(1),
                ..Default::default()
            };

            let currency = args.currency.unwrap_or_else(|| "USD".into());

            let params = QueryParams {
                legs,
                passengers,
                seat,
                trip,
                language: "en".into(),
                currency,
            };

            if let Err(e) = params.validate() {
                return tool_error(e.to_string());
            }

            let url = crate::generate_browser_url(&params);
            Ok(CallToolResult::success(vec![Content::text(url)]))
        }
    }

    #[tool(description = "Open a URL in the default web browser. IMPORTANT: To open flight results, you MUST call flyr_get_url first to get the URL, then pass that URL here. NEVER construct Google Flights URLs yourself -- they require special encoding that only flyr_get_url can produce.")]
    async fn open_url(
        &self,
        Parameters(args): Parameters<OpenUrlArgs>,
    ) -> Result<CallToolResult, McpError> {
        if !args.url.starts_with("http://") && !args.url.starts_with("https://") {
            return tool_error("URL must start with http:// or https://");
        }

        match open::that(&args.url) {
            Ok(()) => Ok(CallToolResult::success(vec![Content::text(format!(
                "Opened: {}",
                args.url
            ))])),
            Err(e) => tool_error(format!("failed to open browser: {e}")),
        }
    }
}

#[tool_handler]
impl ServerHandler for FlyrMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation {
                name: "flyr".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                ..Default::default()
            },
            instructions: Some(
                "Flight search tool. Workflow: (1) flyr_search to find flights. (2) To open in browser: call flyr_get_url with same params to get URL, then call open_url with that URL. NEVER construct Google Flights URLs yourself -- they require special protobuf encoding.".into(),
            ),
        }
    }
}

pub async fn run() {
    let service = FlyrMcp::new()
        .serve(rmcp::transport::stdio())
        .await
        .expect("failed to start MCP server");
    service.waiting().await.expect("MCP server error");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_legs_from_to_date() {
        let (legs, trip) = parse_legs("HEL", "BCN", "2026-03-01", None, None, None);
        assert_eq!(legs.len(), 1);
        assert_eq!(legs[0].from_airport, "HEL");
        assert_eq!(legs[0].to_airport, "BCN");
        assert_eq!(legs[0].date, "2026-03-01");
        assert!(matches!(trip, TripType::OneWay));
    }

    #[test]
    fn parse_legs_with_return_date() {
        let (legs, trip) =
            parse_legs("HEL", "BCN", "2026-03-01", Some("2026-03-08"), None, None);
        assert_eq!(legs.len(), 2);
        assert_eq!(legs[0].from_airport, "HEL");
        assert_eq!(legs[0].to_airport, "BCN");
        assert_eq!(legs[1].from_airport, "BCN");
        assert_eq!(legs[1].to_airport, "HEL");
        assert_eq!(legs[1].date, "2026-03-08");
        assert!(matches!(trip, TripType::RoundTrip));
    }

    #[test]
    fn parse_legs_with_airlines() {
        let (legs, _) = parse_legs("HEL", "BCN", "2026-03-01", None, Some(1), Some("AY,IB"));
        assert_eq!(legs[0].max_stops, Some(1));
        assert_eq!(
            legs[0].airlines,
            Some(vec!["AY".to_string(), "IB".to_string()])
        );
    }
}
