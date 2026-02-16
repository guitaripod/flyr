use std::fmt;

#[derive(Debug)]
pub enum FlightError {
    Timeout,
    ConnectionFailed(String),
    DnsResolution(String),
    ProxyError(String),
    RateLimited,
    Blocked(u16),
    HttpStatus(u16),
    TlsError(String),
    ScriptTagNotFound,
    JsParse(String),
    NoResults,
    InvalidAirport(String),
    InvalidDate(String),
    Validation(String),
}

impl fmt::Display for FlightError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Timeout => write!(
                f,
                "request timed out — Google may be slow or unreachable. \
                 Try increasing --timeout or check your connection"
            ),
            Self::ConnectionFailed(detail) => write!(
                f,
                "connection failed — check your internet connection ({detail})"
            ),
            Self::DnsResolution(host) => write!(
                f,
                "DNS resolution failed for {host} — check your internet connection"
            ),
            Self::ProxyError(detail) => write!(
                f,
                "proxy error — check your --proxy URL is correct ({detail})"
            ),
            Self::RateLimited => write!(
                f,
                "rate limited by Google (HTTP 429) — wait a few minutes before retrying, \
                 or use --proxy to route through a different IP"
            ),
            Self::Blocked(status) => write!(
                f,
                "request blocked by Google (HTTP {status}) — this usually means \
                 rate limiting or bot detection. Try again later or use --proxy"
            ),
            Self::HttpStatus(status) => write!(
                f,
                "unexpected HTTP status {status} from Google Flights"
            ),
            Self::TlsError(detail) => write!(
                f,
                "TLS/SSL error — connection to Google failed ({detail})"
            ),
            Self::ScriptTagNotFound => write!(
                f,
                "failed to parse Google Flights response — the page structure may have changed, \
                 or Google returned a CAPTCHA/consent page. \
                 Try again, use --proxy, or file an issue if this persists"
            ),
            Self::JsParse(detail) => write!(
                f,
                "failed to parse flight data from response — {detail}. \
                 This may indicate a Google Flights format change"
            ),
            Self::NoResults => write!(f, "no flights found for this search"),
            Self::InvalidAirport(code) => write!(
                f,
                "invalid airport code \"{code}\" — must be exactly 3 letters (e.g. JFK, HEL, NRT)"
            ),
            Self::InvalidDate(date) => write!(
                f,
                "invalid date \"{date}\" — must be YYYY-MM-DD format (e.g. 2026-03-01)"
            ),
            Self::Validation(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for FlightError {}

pub fn from_http_error(err: wreq::Error) -> FlightError {
    let msg = err.to_string();
    let lower = msg.to_lowercase();

    if err.is_timeout() {
        return FlightError::Timeout;
    }

    if err.is_connect() {
        if lower.contains("dns") || lower.contains("resolve") || lower.contains("getaddrinfo") {
            return FlightError::DnsResolution(msg);
        }
        return FlightError::ConnectionFailed(msg);
    }

    if lower.contains("proxy") || lower.contains("socks") {
        return FlightError::ProxyError(msg);
    }

    if lower.contains("tls") || lower.contains("ssl") || lower.contains("certificate") {
        return FlightError::TlsError(msg);
    }

    if lower.contains("builder error") && lower.contains("uri") {
        return FlightError::ProxyError(msg);
    }

    FlightError::ConnectionFailed(msg)
}
