use rmcp::handler::server::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::*;
use rmcp::schemars;
use rmcp::{tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler, ServiceExt};
use serde::Deserialize;

use crate::date::Date;
use crate::error::FlightError;
use crate::fetch::FetchOptions;
use crate::model::keep_cheapest_dates;
use crate::query::{parse_codes, route_queries, DateQuery, Filters, Passengers, QueryParams, Seat};
use crate::table;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct FilterArgs {
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
}

impl FilterArgs {
    fn to_filters(&self) -> Result<Filters, FlightError> {
        Ok(Filters {
            max_stops: self.max_stops,
            airlines: self.airlines.as_deref().map(parse_codes),
            passengers: Passengers {
                adults: self.adults.unwrap_or(1),
                children: self.children.unwrap_or(0),
                infants_in_seat: self.infants_in_seat.unwrap_or(0),
                infants_on_lap: self.infants_on_lap.unwrap_or(0),
            },
            seat: self
                .seat
                .as_deref()
                .map(Seat::from_str_loose)
                .transpose()?
                .unwrap_or(Seat::Economy),
            language: "en".into(),
            currency: self.currency.as_deref().unwrap_or("USD").to_uppercase(),
        })
    }
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct RouteArgs {
    #[schemars(
        description = "Departure airport or city IATA code, exactly 3 uppercase letters. Examples: HEL, JFK, or LON for all London airports"
    )]
    from: String,
    #[schemars(
        description = "Arrival airport or city IATA code(s). Comma-separate for multi-destination. Examples: BCN, NYC or BCN,ATH,AYT"
    )]
    to: String,
    #[schemars(description = "Departure date in YYYY-MM-DD format. Example: 2026-03-01")]
    date: String,
    #[schemars(
        description = "Return date in YYYY-MM-DD for round-trip. Auto-sets trip type to round-trip"
    )]
    return_date: Option<String>,
    #[serde(flatten)]
    filters: FilterArgs,
}

impl RouteArgs {
    fn queries(&self) -> Result<(Filters, Vec<(String, QueryParams)>), FlightError> {
        let filters = self.filters.to_filters()?;
        let destinations = parse_codes(&self.to);
        if destinations.is_empty() {
            return Err(FlightError::Validation(
                "'to' needs at least one IATA code".into(),
            ));
        }

        let queries = route_queries(
            &self.from.trim().to_uppercase(),
            &destinations,
            &self.date,
            self.return_date.as_deref(),
            &filters,
        );
        for (_, query) in &queries {
            query.validate()?;
        }
        Ok((filters, queries))
    }
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct SearchArgs {
    #[serde(flatten)]
    route: RouteArgs,
    #[schemars(description = "Return only N cheapest results")]
    top: Option<usize>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct GetUrlArgs {
    #[serde(flatten)]
    route: RouteArgs,
    #[schemars(description = "Also open the URL(s) in the user's web browser. Default: false")]
    open: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct DatesArgs {
    #[schemars(description = "Departure airport IATA code (no city codes). Example: HEL")]
    from: String,
    #[schemars(description = "Arrival airport IATA code (no city codes). Example: BCN")]
    to: String,
    #[schemars(description = "First departure date to price, YYYY-MM-DD. Example: 2026-03-01")]
    start_date: String,
    #[schemars(description = "Last departure date to price, YYYY-MM-DD. Example: 2026-03-31")]
    end_date: String,
    #[schemars(
        description = "Trip length in days for round trips, e.g. 7. Omit for one-way fares"
    )]
    stay_days: Option<u32>,
    #[serde(flatten)]
    filters: FilterArgs,
    #[schemars(description = "Return only the N cheapest dates, cheapest first")]
    top: Option<usize>,
}

impl DatesArgs {
    fn query(&self) -> Result<DateQuery, FlightError> {
        let date = |s: &str| Date::parse(s).ok_or_else(|| FlightError::InvalidDate(s.into()));
        let query = DateQuery {
            from_airport: self.from.trim().to_uppercase(),
            to_airport: self.to.trim().to_uppercase(),
            start: date(&self.start_date)?,
            end: date(&self.end_date)?,
            stay_days: self.stay_days,
            filters: self.filters.to_filters()?,
        };
        query.validate()?;
        Ok(query)
    }
}

fn tool_error(msg: impl Into<String>) -> Result<CallToolResult, McpError> {
    Ok(CallToolResult::error(vec![Content::text(msg.into())]))
}

fn tool_text(text: impl Into<String>) -> Result<CallToolResult, McpError> {
    Ok(CallToolResult::success(vec![Content::text(text.into())]))
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
        description = "Search Google Flights for flights between airports on specific dates. Returns one line per flight: price | route | total duration | stops with layover times | airlines | departure>arrival times (+1 = next day) | flight numbers. Comma-separate 'to' to compare destinations. Round-trip prices cover both directions; the listed flights are outbound. To open results in the browser: call flyr_get_url with the same parameters and open=true."
    )]
    async fn flyr_search(
        &self,
        Parameters(args): Parameters<SearchArgs>,
    ) -> Result<CallToolResult, McpError> {
        let (filters, queries) = match args.route.queries() {
            Ok(built) => built,
            Err(e) => return tool_error(e.to_string()),
        };
        let round_trip = args.route.return_date.is_some();

        let mut outcomes = crate::search_all(queries, &FetchOptions::default()).await;
        if let Some(n) = args.top {
            for result in outcomes.iter_mut().filter_map(|(_, o)| o.as_mut().ok()) {
                result.keep_cheapest(n);
            }
        }

        let render =
            |r: &crate::model::SearchResult| table::compact(r, &filters.currency, round_trip);
        if let [(_, outcome)] = &outcomes[..] {
            return match outcome {
                Ok(result) => tool_text(render(result)),
                Err(e) => tool_error(e.to_string()),
            };
        }

        let text = table::sections(&outcomes, "\n", render);
        if outcomes.iter().all(|(_, o)| o.is_err()) {
            return tool_error(text);
        }
        tool_text(text)
    }

    #[tool(
        description = "Find the cheapest dates to fly between two airports across a date range, in one request. Returns one line per departure date with the lowest fare. For round trips set stay_days to the trip length in days. Airport codes only: city codes like LON are not supported here."
    )]
    async fn flyr_dates(
        &self,
        Parameters(args): Parameters<DatesArgs>,
    ) -> Result<CallToolResult, McpError> {
        let query = match args.query() {
            Ok(query) => query,
            Err(e) => return tool_error(e.to_string()),
        };

        match crate::search_dates(&query, &FetchOptions::default()).await {
            Ok(mut dates) => {
                if let Some(n) = args.top {
                    keep_cheapest_dates(&mut dates, n);
                }
                tool_text(table::compact_dates(&dates, &query.filters.currency))
            }
            Err(e) => tool_error(e.to_string()),
        }
    }

    #[tool(
        description = "Get the Google Flights URL for the given search parameters, one per destination, and optionally open it in the user's browser with open=true. This is the ONLY way to get a valid Google Flights URL. NEVER construct Google Flights URLs manually."
    )]
    async fn flyr_get_url(
        &self,
        Parameters(args): Parameters<GetUrlArgs>,
    ) -> Result<CallToolResult, McpError> {
        let urls: Vec<String> = match args.route.queries() {
            Ok((_, queries)) => queries
                .iter()
                .map(|(_, query)| crate::generate_browser_url(query))
                .collect(),
            Err(e) => return tool_error(e.to_string()),
        };

        if !args.open.unwrap_or(false) {
            return tool_text(urls.join("\n"));
        }

        for url in &urls {
            if let Err(e) = open::that(url) {
                return tool_error(format!("failed to open browser: {e}"));
            }
        }
        tool_text(format!("Opened in the browser:\n{}", urls.join("\n")))
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
                "Flight search tools. flyr_search finds flights on specific dates (comma-separate 'to' to compare destinations). flyr_dates finds the cheapest departure dates across a date range. To open results in the browser, call flyr_get_url with the same parameters and open=true. NEVER construct Google Flights URLs yourself -- they require special protobuf encoding.".into(),
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
    use crate::query::TripType;

    fn route(to: &str, return_date: Option<&str>) -> RouteArgs {
        serde_json::from_value(serde_json::json!({
            "from": "hel",
            "to": to,
            "date": "2026-03-01",
            "return_date": return_date,
            "max_stops": 1,
            "airlines": "ay, ib",
        }))
        .unwrap()
    }

    #[test]
    fn one_way_query_per_destination() {
        let (filters, queries) = route("bcn,ath", None).queries().unwrap();
        assert_eq!(filters.currency, "USD");
        assert_eq!(queries.len(), 2);
        assert_eq!(queries[1].0, "ATH");
        let legs = &queries[0].1.legs;
        assert_eq!(legs.len(), 1);
        assert_eq!(legs[0].from_airport, "HEL");
        assert_eq!(legs[0].to_airport, "BCN");
        assert_eq!(legs[0].max_stops, Some(1));
        assert_eq!(legs[0].airlines, Some(vec!["AY".to_string(), "IB".to_string()]));
        assert_eq!(queries[0].1.trip, TripType::OneWay);
    }

    #[test]
    fn return_date_builds_round_trip() {
        let (_, queries) = route("BCN", Some("2026-03-08")).queries().unwrap();
        let query = &queries[0].1;
        assert_eq!(query.trip, TripType::RoundTrip);
        assert_eq!(query.legs[1].from_airport, "BCN");
        assert_eq!(query.legs[1].to_airport, "HEL");
        assert_eq!(query.legs[1].date, "2026-03-08");
    }

    #[test]
    fn invalid_seat_is_rejected() {
        let mut args = route("BCN", None);
        args.filters.seat = Some("luxury".into());
        assert!(args.queries().is_err());
    }

    #[test]
    fn tool_schemas_are_flat() {
        let schema = serde_json::to_value(schemars::schema_for!(SearchArgs)).unwrap();
        let properties = schema["properties"].as_object().unwrap();
        for field in ["from", "to", "date", "max_stops", "airlines", "currency", "top"] {
            assert!(properties.contains_key(field), "missing {field}");
        }
        let required = schema["required"].as_array().unwrap();
        assert_eq!(required.len(), 3);
    }

    #[test]
    fn dates_args_validate_range() {
        let args: DatesArgs = serde_json::from_value(serde_json::json!({
            "from": "HEL",
            "to": "BCN",
            "start_date": "2099-03-31",
            "end_date": "2099-03-01",
        }))
        .unwrap();
        assert!(args.query().is_err());
    }
}
