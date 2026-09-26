pub mod date;
pub mod error;
pub mod fetch;
pub mod mcp;
pub mod model;
pub mod parse;
pub mod proto;
pub mod query;
pub mod table;

use tokio::task::JoinSet;

use error::FlightError;
use fetch::FetchOptions;
use model::{DatePrice, SearchResult};
use query::{DateQuery, QueryParams};

/// Searches run at once when fanning out over several destinations, to stay under
/// Google's rate limits for a single IP.
const MAX_CONCURRENT_SEARCHES: usize = 4;

pub async fn search(
    query: &QueryParams,
    options: &FetchOptions,
) -> Result<SearchResult, FlightError> {
    let params = query.to_url_params();
    let html = fetch::fetch_html(&params, options).await?;
    parse::parse_html(&html)
}

/// Runs labelled searches with bounded concurrency and returns the outcomes in input order.
pub async fn search_all(
    queries: Vec<(String, QueryParams)>,
    options: &FetchOptions,
) -> Vec<(String, Result<SearchResult, FlightError>)> {
    let mut outcomes: Vec<Option<(String, Result<SearchResult, FlightError>)>> =
        queries.iter().map(|_| None).collect();
    let mut pending = queries.into_iter().enumerate();
    let mut running = JoinSet::new();

    loop {
        while running.len() < MAX_CONCURRENT_SEARCHES {
            let Some((index, (label, query))) = pending.next() else {
                break;
            };
            let options = options.clone();
            running.spawn(async move { (index, label, search(&query, &options).await) });
        }

        let Some(done) = running.join_next().await else {
            break;
        };
        let (index, label, outcome) = done.expect("search task panicked");
        outcomes[index] = Some((label, outcome));
    }

    outcomes.into_iter().flatten().collect()
}

/// Cheapest fare per departure date across the query's range, in one request.
pub async fn search_dates(
    query: &DateQuery,
    options: &FetchOptions,
) -> Result<Vec<DatePrice>, FlightError> {
    let body = fetch::fetch_calendar(
        query.to_request_body(),
        &query.filters.currency,
        &query.filters.language,
        options,
    )
    .await?;
    parse::parse_calendar(&body)
}

pub fn generate_browser_url(params: &QueryParams) -> String {
    query::to_google_flights_url(params)
}
