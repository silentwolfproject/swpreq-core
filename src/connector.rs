use crate::resolver::AsyncResolver;
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketFamily {
    Any,
    IPv4,
    IPv6,
}

#[derive(Clone)]
pub struct TCPConnector {
    pub limit: usize,
    pub limit_per_host: usize,
    pub ttl_dns_cache: Duration,
    pub use_dns_cache: bool,
    pub force_close: bool,
    pub enable_cleanup_closed: bool,
    pub keepalive_timeout: Duration,
    pub family: SocketFamily,
    pub resolver: Option<Arc<AsyncResolver>>,
    pub ssl_verify: bool,
    pub ssl_ca_file: Option<String>,
    pub ssl_cert_file: Option<String>,
    pub ssl_key_file: Option<String>,
}

impl Default for TCPConnector {
    fn default() -> Self {
        Self {
            limit: 100,
            limit_per_host: 0,
            ttl_dns_cache: Duration::from_secs(10),
            use_dns_cache: true,
            force_close: false,
            enable_cleanup_closed: false,
            keepalive_timeout: Duration::from_secs(15),
            family: SocketFamily::Any,
            resolver: None,
            ssl_verify: true,
            ssl_ca_file: None,
            ssl_cert_file: None,
            ssl_key_file: None,
        }
    }
}

impl TCPConnector {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }

    pub fn with_limit_per_host(mut self, limit: usize) -> Self {
        self.limit_per_host = limit;
        self
    }

    pub fn with_ttl_dns_cache(mut self, ttl: Duration) -> Self {
        self.ttl_dns_cache = ttl;
        self
    }

    pub fn with_force_close(mut self, force: bool) -> Self {
        self.force_close = force;
        self
    }

    pub fn with_family(mut self, family: SocketFamily) -> Self {
        self.family = family;
        self
    }

    pub fn with_resolver(mut self, resolver: Arc<AsyncResolver>) -> Self {
        self.resolver = Some(resolver);
        self
    }

    pub fn with_ssl_verify(mut self, verify: bool) -> Self {
        self.ssl_verify = verify;
        self
    }

    pub fn with_ssl_ca(mut self, ca_file: &str) -> Self {
        self.ssl_ca_file = Some(ca_file.to_string());
        self
    }

    pub fn with_ssl_client_cert(mut self, cert_file: &str, key_file: &str) -> Self {
        self.ssl_cert_file = Some(cert_file.to_string());
        self.ssl_key_file = Some(key_file.to_string());
        self
    }
}