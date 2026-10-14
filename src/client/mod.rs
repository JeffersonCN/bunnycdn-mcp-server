//! Generic Bunny API client (design §3, REQ-CLI-*).

pub mod error;

use std::hash::{BuildHasher, RandomState};
use std::time::Duration;

use reqwest::header::{ACCEPT, HeaderMap, HeaderValue, RETRY_AFTER};
use reqwest::{Method, StatusCode};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::Value;
use url::Url;

pub use error::BunnyError;

use crate::config::Config;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_RETRIES: u32 = 3;
const BACKOFF_BASE: Duration = Duration::from_millis(500);
const MAX_RETRY_AFTER: Duration = Duration::from_secs(10);
const USER_AGENT: &str = concat!("bunnycdn-mcp/", env!("CARGO_PKG_VERSION"));

/// Installs the `ring` rustls provider. reqwest is built with `rustls-no-provider`, so a client
/// cannot be built before this runs; repeated calls are harmless.
pub fn install_crypto_provider() {
    let _ = rustls::crypto::ring::default_provider().install_default();
}

/// Cheap to clone: the inner `reqwest::Client` is reference-counted, so every clone shares one
/// connection pool (REQ-CLI-06).
#[derive(Clone)]
pub struct BunnyClient {
    http: reqwest::Client,
    base_url: Url,
    /// Upper bound applied to every retry delay. `None` in production; tests set it to zero so
    /// retry paths run instantly.
    max_retry_delay: Option<Duration>,
}

/// Error body returned by the Bunny API (`ApiErrorData`).
#[derive(Deserialize)]
struct ApiErrorData {
    #[serde(rename = "ErrorKey")]
    error_key: Option<String>,
    #[serde(rename = "Field")]
    field: Option<String>,
    #[serde(rename = "Message")]
    message: Option<String>,
}

impl BunnyClient {
    pub fn new(cfg: &Config) -> Result<Self, BunnyError> {
        install_crypto_provider();

        let mut key = HeaderValue::from_str(cfg.api_key.expose()).map_err(|_| {
            BunnyError::InvalidArgument(
                "API key contains characters not allowed in a header".into(),
            )
        })?;
        key.set_sensitive(true);
        let mut headers = HeaderMap::new();
        headers.insert("AccessKey", key);
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));

        let http = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .default_headers(headers)
            .timeout(REQUEST_TIMEOUT)
            .connect_timeout(CONNECT_TIMEOUT)
            .build()
            .map_err(|e| BunnyError::Network(e.to_string()))?;

        Ok(Self {
            http,
            base_url: cfg.base_url.clone(),
            max_retry_delay: None,
        })
    }

    /// Caps every retry delay (backoff and `Retry-After`). Meant for tests.
    pub fn with_max_retry_delay(mut self, cap: Duration) -> Self {
        self.max_retry_delay = Some(cap);
        self
    }

    /// Core request: builds the URL, sends headers, retries (REQ-CLI-02/03) and maps errors
    /// (REQ-CLI-04). An empty body yields `Ok(Value::Null)` (REQ-CLI-05).
    pub async fn request(
        &self,
        method: Method,
        path: &str,
        query: &[(&str, String)],
        body: Option<&Value>,
    ) -> Result<Value, BunnyError> {
        let url = self.url(path, query)?;
        // Only idempotent verbs are retried on 5xx/network errors; a POST may already have
        // been applied.
        let retry_failures = method == Method::GET || method == Method::DELETE;
        let mut attempt = 0;

        loop {
            let mut req = self.http.request(method.clone(), url.clone());
            if let Some(body) = body {
                req = req.json(body);
            }

            let resp = match req.send().await {
                Ok(resp) => resp,
                Err(e) if e.is_timeout() => return Err(BunnyError::Timeout),
                Err(e) => {
                    if retry_failures && attempt < MAX_RETRIES {
                        self.sleep(backoff(attempt)).await;
                        attempt += 1;
                        continue;
                    }
                    return Err(BunnyError::Network(e.without_url().to_string()));
                }
            };

            let status = resp.status();
            if status == StatusCode::TOO_MANY_REQUESTS {
                if attempt < MAX_RETRIES {
                    let delay = retry_after(resp.headers()).unwrap_or_else(|| backoff(attempt));
                    self.sleep(delay).await;
                    attempt += 1;
                    continue;
                }
                return Err(BunnyError::RateLimited);
            }
            if status.is_server_error() && retry_failures && attempt < MAX_RETRIES {
                self.sleep(backoff(attempt)).await;
                attempt += 1;
                continue;
            }

            let bytes = resp.bytes().await.map_err(|e| {
                if e.is_timeout() {
                    BunnyError::Timeout
                } else {
                    BunnyError::Network(e.without_url().to_string())
                }
            })?;

            if status.is_success() {
                if bytes.iter().all(u8::is_ascii_whitespace) {
                    return Ok(Value::Null);
                }
                return serde_json::from_slice(&bytes)
                    .map_err(|e| BunnyError::Decode(e.to_string()));
            }
            return Err(map_error(status, path, &bytes));
        }
    }

    pub async fn get<T: DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, String)],
    ) -> Result<T, BunnyError> {
        decode(self.request(Method::GET, path, query, None).await?)
    }

    pub async fn post<T: DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, String)],
        body: Option<&Value>,
    ) -> Result<T, BunnyError> {
        decode(self.request(Method::POST, path, query, body).await?)
    }

    pub async fn delete<T: DeserializeOwned>(
        &self,
        path: &str,
        body: Option<&Value>,
    ) -> Result<T, BunnyError> {
        decode(self.request(Method::DELETE, path, &[], body).await?)
    }

    fn url(&self, path: &str, query: &[(&str, String)]) -> Result<Url, BunnyError> {
        let base = self.base_url.as_str().trim_end_matches('/');
        let path = path.strip_prefix('/').unwrap_or(path);
        let mut url = Url::parse(&format!("{base}/{path}"))
            .map_err(|e| BunnyError::InvalidArgument(format!("bad request path {path:?}: {e}")))?;
        if !query.is_empty() {
            url.query_pairs_mut()
                .extend_pairs(query.iter().map(|(k, v)| (k, v)));
        }
        Ok(url)
    }

    async fn sleep(&self, delay: Duration) {
        let delay = match self.max_retry_delay {
            Some(cap) => delay.min(cap),
            None => delay,
        };
        if !delay.is_zero() {
            tokio::time::sleep(delay).await;
        }
    }
}

fn decode<T: DeserializeOwned>(v: Value) -> Result<T, BunnyError> {
    serde_json::from_value(v).map_err(|e| BunnyError::Decode(e.to_string()))
}

fn map_error(status: StatusCode, path: &str, body: &[u8]) -> BunnyError {
    match status {
        StatusCode::UNAUTHORIZED => BunnyError::Unauthorized,
        StatusCode::NOT_FOUND => BunnyError::NotFound {
            path: path.to_string(),
        },
        _ => {
            let (key, field, message) = match serde_json::from_slice::<ApiErrorData>(body) {
                Ok(d) => (d.error_key, d.field, d.message),
                Err(_) => {
                    // Non-JSON error body (proxy page, plain text): keep a short excerpt.
                    let text = String::from_utf8_lossy(body);
                    let text = text.trim();
                    let excerpt = (!text.is_empty()).then(|| text.chars().take(200).collect());
                    (None, None, excerpt)
                }
            };
            BunnyError::Api {
                status: status.as_u16(),
                key,
                field,
                message,
            }
        }
    }
}

/// `Retry-After` in seconds, capped at 10 s. HTTP-date values fall back to backoff.
fn retry_after(headers: &HeaderMap) -> Option<Duration> {
    let secs: u64 = headers
        .get(RETRY_AFTER)?
        .to_str()
        .ok()?
        .trim()
        .parse()
        .ok()?;
    Some(Duration::from_secs(secs).min(MAX_RETRY_AFTER))
}

/// Exponential backoff with jitter: base·2^attempt + rand[0, base).
fn backoff(attempt: u32) -> Duration {
    let base = BACKOFF_BASE.as_millis() as u64;
    // RandomState is randomly seeded per instance: enough entropy for jitter without a crate.
    let jitter = RandomState::new().hash_one(attempt) % base;
    Duration::from_millis(base * 2u64.pow(attempt) + jitter)
}

#[cfg(test)]
mod tests {
    use super::*;

    // covers: REQ-CLI-02
    #[test]
    fn retry_after_is_parsed_and_capped() {
        let mut h = HeaderMap::new();
        assert_eq!(retry_after(&h), None);
        h.insert(RETRY_AFTER, HeaderValue::from_static("2"));
        assert_eq!(retry_after(&h), Some(Duration::from_secs(2)));
        h.insert(RETRY_AFTER, HeaderValue::from_static("120"));
        assert_eq!(retry_after(&h), Some(MAX_RETRY_AFTER));
        h.insert(
            RETRY_AFTER,
            HeaderValue::from_static("Wed, 21 Oct 2015 07:28:00 GMT"),
        );
        assert_eq!(retry_after(&h), None);
    }

    // covers: REQ-CLI-02
    #[test]
    fn backoff_grows_exponentially_with_bounded_jitter() {
        for attempt in 0..3 {
            let d = backoff(attempt).as_millis() as u64;
            let floor = 500 * 2u64.pow(attempt);
            assert!(d >= floor && d < floor + 500, "attempt {attempt}: {d}ms");
        }
    }

    #[test]
    fn url_joins_path_and_query() {
        let cfg = Config {
            api_key: crate::secret::Secret::new("k"),
            base_url: Url::parse("http://127.0.0.1:1234/").unwrap(),
        };
        let c = BunnyClient::new(&cfg).unwrap();
        assert_eq!(
            c.url("/pullzone/1", &[]).unwrap().as_str(),
            "http://127.0.0.1:1234/pullzone/1"
        );
        assert_eq!(
            c.url("/purge", &[("url", "https://a.b/c d".into())])
                .unwrap()
                .as_str(),
            "http://127.0.0.1:1234/purge?url=https%3A%2F%2Fa.b%2Fc+d"
        );
    }

    #[test]
    fn non_json_error_body_is_excerpted() {
        let e = map_error(StatusCode::BAD_GATEWAY, "/x", b"<html>bad gateway</html>");
        assert!(
            matches!(e, BunnyError::Api { status: 502, message: Some(ref m), .. } if m.contains("bad gateway"))
        );
    }
}
