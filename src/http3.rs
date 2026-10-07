use crate::error::{Result, SwpreqError};
use crate::response::Response;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct Http3Config {
    pub enabled: bool,
    pub max_idle_timeout: Duration,
    pub keep_alive_interval: Duration,
    pub max_concurrent_streams: u32,
    pub enable_0rtt: bool,
}

impl Default for Http3Config {
    fn default() -> Self {
        Self {
            enabled: false,
            max_idle_timeout: Duration::from_secs(30),
            keep_alive_interval: Duration::from_secs(15),
            max_concurrent_streams: 100,
            enable_0rtt: false,
        }
    }
}

pub async fn request_http3(
    _url: &str,
    _method: &str,
    _headers: Vec<(String, String)>,
    _body: Option<Vec<u8>>,
    _timeout: Duration,
) -> Result<Response> {
    Err(SwpreqError::Internal(
        "HTTP/3 support requires 'http3' feature flag and QUIC runtime. Falling back to HTTP/2.".to_string(),
    ))
}

pub fn is_http3_supported() -> bool {
    cfg!(feature = "http3-enabled")
}