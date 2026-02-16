use std::sync::Arc;
use std::time::Duration;

use wreq::Client;
use wreq::cookie::Jar;
use wreq_util::Emulation;

use crate::error::{self, FlightError};

const BASE_URL: &str = "https://www.google.com/travel/flights";

pub struct FetchOptions {
    pub proxy: Option<String>,
    pub timeout: u64,
}

impl Default for FetchOptions {
    fn default() -> Self {
        Self {
            proxy: None,
            timeout: 30,
        }
    }
}

pub async fn fetch_html(
    params: &[(String, String)],
    options: &FetchOptions,
) -> Result<String, FlightError> {
    let jar = Arc::new(Jar::default());
    let url: wreq::Uri = "https://www.google.com".parse().unwrap();
    jar.add(
        "SOCS=CAESEwgDEgk2MjA5NDM1NjAaAmVuIAEaBgiA_Le-Bg",
        &url,
    );
    jar.add("CONSENT=PENDING+987", &url);

    let mut builder = Client::builder()
        .emulation(Emulation::Chrome137)
        .cookie_provider(jar)
        .timeout(Duration::from_secs(options.timeout));

    if let Some(ref proxy) = options.proxy {
        builder = builder.proxy(
            wreq::Proxy::all(proxy).map_err(error::from_http_error)?,
        );
    }

    let client = builder.build().map_err(error::from_http_error)?;

    let response = client
        .get(BASE_URL)
        .query(params)
        .send()
        .await
        .map_err(error::from_http_error)?;

    let status = response.status().as_u16();
    match status {
        200 => {}
        429 => return Err(FlightError::RateLimited),
        403 | 503 => return Err(FlightError::Blocked(status)),
        _ if status >= 400 => return Err(FlightError::HttpStatus(status)),
        _ => {}
    }

    response.text().await.map_err(error::from_http_error)
}
