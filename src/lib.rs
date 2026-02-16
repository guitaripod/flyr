pub mod error;
pub mod fetch;
pub mod model;
pub mod parse;
pub mod proto;
pub mod query;
pub mod table;

use error::FlightError;
use fetch::FetchOptions;
use model::SearchResult;
use query::SearchQuery;

pub async fn search(
    query: SearchQuery,
    options: FetchOptions,
) -> Result<SearchResult, FlightError> {
    let params = query.to_url_params();
    let html = fetch::fetch_html(&params, &options).await?;
    parse::parse_html(&html)
}
