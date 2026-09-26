use std::sync::Arc;
use std::time::Duration;

use scraper::{Html, Selector};
use wreq::Client;
use wreq::cookie::Jar;
use wreq_util::Emulation;

use crate::error::{self, FlightError};

fn cache_buster() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .to_string()
}

const BASE_URL: &str = "https://www.google.com/travel/flights";
const CALENDAR_URL: &str = "https://www.google.com/_/FlightsFrontendUi/data/travel.frontend.flights.FlightsFrontendService/GetCalendarGraph";
const MAX_REDIRECTS: u8 = 10;

#[derive(Clone)]
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

fn is_redirect(status: u16) -> bool {
    matches!(status, 301 | 302 | 303 | 307 | 308)
}

fn extract_location(response: &wreq::Response) -> Option<String> {
    response
        .headers()
        .get("location")
        .and_then(|v| v.to_str().ok())
        .map(String::from)
}

fn extract_consent_form(html: &str) -> Option<String> {
    let document = Html::parse_document(html);
    let form_sel = Selector::parse("form[action=\"https://consent.google.com/save\"]").ok()?;
    let input_sel = Selector::parse("input[type=\"hidden\"]").ok()?;

    let form = document.select(&form_sel).next()?;

    let mut fields: Vec<(String, String)> = Vec::new();
    for input in form.select(&input_sel) {
        if let (Some(name), Some(value)) = (input.attr("name"), input.attr("value")) {
            fields.push((name.to_string(), value.to_string()));
        }
    }

    if fields.is_empty() {
        return None;
    }

    Some(
        fields
            .iter()
            .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
            .collect::<Vec<_>>()
            .join("&"),
    )
}

async fn follow_redirects(client: &Client, start_url: &str) -> Result<String, FlightError> {
    let mut url = start_url.to_string();

    for _ in 0..MAX_REDIRECTS {
        let response = client
            .get(&url)
            .send()
            .await
            .map_err(error::from_http_error)?;

        let status = response.status().as_u16();

        if is_redirect(status) {
            url = extract_location(&response)
                .ok_or_else(|| FlightError::JsParse("redirect without location".into()))?;
            continue;
        }

        check_status(status)?;

        let html = response.text().await.map_err(error::from_http_error)?;

        if let Some(form_body) = extract_consent_form(&html) {
            let save_resp = client
                .post("https://consent.google.com/save")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(form_body)
                .send()
                .await
                .map_err(error::from_http_error)?;

            if is_redirect(save_resp.status().as_u16()) {
                url = extract_location(&save_resp)
                    .ok_or_else(|| FlightError::JsParse("consent save: no redirect".into()))?;
                continue;
            }

            return Err(FlightError::Blocked(save_resp.status().as_u16()));
        }

        return Ok(html);
    }

    Err(FlightError::Blocked(302))
}

fn client(options: &FetchOptions) -> Result<Client, FlightError> {
    let jar = Arc::new(Jar::default());

    let mut builder = Client::builder()
        .emulation(Emulation::Chrome149)
        .cookie_provider(jar)
        .timeout(Duration::from_secs(options.timeout));

    if let Some(ref proxy) = options.proxy {
        builder = builder.proxy(
            wreq::Proxy::all(proxy).map_err(error::from_http_error)?,
        );
    }

    builder.build().map_err(error::from_http_error)
}

fn check_status(status: u16) -> Result<(), FlightError> {
    match status {
        429 => Err(FlightError::RateLimited),
        403 | 503 => Err(FlightError::Blocked(status)),
        s if s >= 400 => Err(FlightError::HttpStatus(s)),
        _ => Ok(()),
    }
}

pub async fn fetch_html(
    params: &[(String, String)],
    options: &FetchOptions,
) -> Result<String, FlightError> {
    let client = client(options)?;

    let mut params = params.to_vec();
    params.push(("cx".to_string(), cache_buster()));

    let mut start_url = format!("{BASE_URL}?");
    for (i, (k, v)) in params.iter().enumerate() {
        if i > 0 {
            start_url.push('&');
        }
        start_url.push_str(&urlencoding::encode(k));
        start_url.push('=');
        start_url.push_str(&urlencoding::encode(v));
    }

    follow_redirects(&client, &start_url).await
}

/// Posts a date-grid request to Google's `GetCalendarGraph` RPC and returns the raw body.
pub async fn fetch_calendar(
    body: String,
    currency: &str,
    language: &str,
    options: &FetchOptions,
) -> Result<String, FlightError> {
    let url = format!(
        "{CALENDAR_URL}?curr={}&hl={}",
        urlencoding::encode(currency),
        urlencoding::encode(language)
    );

    let response = client(options)?
        .post(&url)
        .header("content-type", "application/x-www-form-urlencoded;charset=UTF-8")
        .body(body)
        .send()
        .await
        .map_err(error::from_http_error)?;

    let status = response.status().as_u16();
    if is_redirect(status) {
        return Err(FlightError::Blocked(status));
    }
    check_status(status)?;

    response.text().await.map_err(error::from_http_error)
}
