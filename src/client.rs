use crate::auth_digest::DigestAuth;
use crate::connector::TCPConnector;
use crate::cookie::CookieJar;
use crate::error::{Result, SwpreqError};
use crate::request::RequestBuilder;
use crate::trace::TraceConfig;
use parking_lot::RwLock;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::redirect::Policy;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex as AsyncMutex;

#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub timeout: Duration,
    pub connect_timeout: Duration,
    pub pool_size: usize,
    pub pool_idle_timeout: Duration,
    pub http2: bool,
    pub redirect_limit: usize,
    pub danger_accept_invalid_certs: bool,
    pub proxy: Option<String>,
    pub user_agent: String,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            connect_timeout: Duration::from_secs(10),
            pool_size: 100,
            pool_idle_timeout: Duration::from_secs(90),
            http2: true,
            redirect_limit: 10,
            danger_accept_invalid_certs: false,
            proxy: None,
            user_agent: format!("swpreq/{}", env!("CARGO_PKG_VERSION")),
        }
    }
}

pub struct Client {
    pub inner: reqwest::Client,
    pub config: ClientConfig,
    pub cookies: Arc<RwLock<CookieJar>>,
    pub default_headers: Arc<RwLock<HeaderMap>>,
    pub proxy_pool: Arc<RwLock<HashMap<String, Arc<reqwest::Client>>>>,
    pub redirect_pool: Arc<RwLock<HashMap<usize, Arc<reqwest::Client>>>>,
    pub hooks: Arc<RwLock<crate::hooks::Hooks>>,
    pub adapters: Arc<RwLock<crate::adapter::AdapterRegistry>>,
    pub retry_policy: crate::retry::RetryPolicy,
    pub netrc: Option<crate::netrc::Netrc>,
    pub connector: Option<Arc<TCPConnector>>,
    pub trace_config: Option<Arc<TraceConfig>>,
    pub digest_auth: Option<Arc<AsyncMutex<DigestAuth>>>,
}

impl Client {
    pub fn new(config: ClientConfig) -> Result<Self> {
        let inner = Self::build_inner(&config, None, None)?;
        let netrc = crate::netrc::Netrc::load().ok().flatten();

        Ok(Self {
            inner,
            config,
            cookies: Arc::new(RwLock::new(CookieJar::new())),
            default_headers: Arc::new(RwLock::new(HeaderMap::new())),
            proxy_pool: Arc::new(RwLock::new(HashMap::new())),
            redirect_pool: Arc::new(RwLock::new(HashMap::new())),
            hooks: Arc::new(RwLock::new(crate::hooks::Hooks::new())),
            adapters: Arc::new(RwLock::new(crate::adapter::AdapterRegistry::new())),
            retry_policy: crate::retry::RetryPolicy::default(),
            netrc,
            connector: None,
            trace_config: None,
            digest_auth: None,
        })
    }

    fn build_inner(
        config: &ClientConfig,
        override_proxy: Option<&str>,
        override_redirect: Option<usize>,
    ) -> Result<reqwest::Client> {
        let redirect_policy = match override_redirect {
            Some(0) => Policy::none(),
            Some(n) => Policy::limited(n),
            None => Policy::limited(config.redirect_limit),
        };

        let mut builder = reqwest::Client::builder()
            .timeout(config.timeout)
            .connect_timeout(config.connect_timeout)
            .pool_max_idle_per_host(config.pool_size)
            .pool_idle_timeout(config.pool_idle_timeout)
            .redirect(redirect_policy)
            .danger_accept_invalid_certs(config.danger_accept_invalid_certs)
            .user_agent(&config.user_agent)
            .gzip(true)
            .brotli(true)
            .deflate(true)
            .zstd(true);

        let proxy_str = override_proxy.map(|s| s.to_string()).or_else(|| config.proxy.clone());
        if let Some(p) = proxy_str {
            let proxy = reqwest::Proxy::all(&p)
                .map_err(|e| SwpreqError::Proxy(e.to_string()))?;
            builder = builder.proxy(proxy);
        }

        builder
            .build()
            .map_err(|e| SwpreqError::Internal(e.to_string()))
    }

    pub fn get_client_for_request(
        &self,
        proxy: Option<&str>,
        redirect_limit: Option<usize>,
    ) -> Result<Arc<reqwest::Client>> {
        if proxy.is_none() && redirect_limit.is_none() {
            return Ok(Arc::new(self.inner.clone()));
        }

        if let Some(p) = proxy {
            let pool = self.proxy_pool.read();
            if let Some(client) = pool.get(p) {
                return Ok(client.clone());
            }
            drop(pool);

            let client = Arc::new(Self::build_inner(&self.config, Some(p), redirect_limit)?);
            self.proxy_pool
                .write()
                .insert(p.to_string(), client.clone());
            return Ok(client);
        }

        if let Some(limit) = redirect_limit {
            let pool = self.redirect_pool.read();
            if let Some(client) = pool.get(&limit) {
                return Ok(client.clone());
            }
            drop(pool);

            let client = Arc::new(Self::build_inner(&self.config, None, Some(limit))?);
            self.redirect_pool
                .write()
                .insert(limit, client.clone());
            return Ok(client);
        }

        Ok(Arc::new(self.inner.clone()))
    }

    pub fn request(&self, method: &str, url: &str) -> RequestBuilder {
        RequestBuilder::new(Arc::new(self.clone()), method, url)
    }

    pub fn get(&self, url: &str) -> RequestBuilder {
        self.request("GET", url)
    }

    pub fn post(&self, url: &str) -> RequestBuilder {
        self.request("POST", url)
    }

    pub fn put(&self, url: &str) -> RequestBuilder {
        self.request("PUT", url)
    }

    pub fn delete(&self, url: &str) -> RequestBuilder {
        self.request("DELETE", url)
    }

    pub fn patch(&self, url: &str) -> RequestBuilder {
        self.request("PATCH", url)
    }

    pub fn head(&self, url: &str) -> RequestBuilder {
        self.request("HEAD", url)
    }

    pub fn options(&self, url: &str) -> RequestBuilder {
        self.request("OPTIONS", url)
    }

    pub fn set_default_header(&self, key: &str, value: &str) -> Result<()> {
        let name = HeaderName::from_bytes(key.as_bytes())
            .map_err(|e| SwpreqError::InvalidArgument(e.to_string()))?;
        let val = HeaderValue::from_str(value)
            .map_err(|e| SwpreqError::InvalidArgument(e.to_string()))?;
        self.default_headers.write().insert(name, val);
        Ok(())
    }

    pub fn remove_default_header(&self, key: &str) -> Result<()> {
        let name = HeaderName::from_bytes(key.as_bytes())
            .map_err(|e| SwpreqError::InvalidArgument(e.to_string()))?;
        self.default_headers.write().remove(name);
        Ok(())
    }

    pub fn clear_default_headers(&self) {
        self.default_headers.write().clear();
    }

    pub fn set_cookie(&self, name: &str, value: &str, domain: &str) {
        self.cookies.write().set(name, value, domain);
    }

    pub fn get_cookie(&self, name: &str, domain: &str) -> Option<String> {
        self.cookies
            .read()
            .get(name, domain)
            .map(|c| c.value.clone())
    }

    pub fn clear_cookies(&self) {
        self.cookies.write().clear();
    }

    pub fn clear_proxy_pool(&self) {
        self.proxy_pool.write().clear();
    }

    pub fn clear_redirect_pool(&self) {
        self.redirect_pool.write().clear();
    }
    pub fn register_request_hook(&self, hook: crate::hooks::RequestHook) {
        self.hooks.write().register_request(hook);
    }

    pub fn register_response_hook(&self, hook: crate::hooks::ResponseHook) {
        self.hooks.write().register_response(hook);
    }

    pub fn mount(&self, prefix: &str, adapter: Arc<dyn crate::adapter::TransportAdapter>) {
        self.adapters.write().mount(prefix, adapter);
    }

    pub fn unmount(&self, prefix: &str) {
        self.adapters.write().unmount(prefix);
    }

    pub fn set_retry_policy(&mut self, policy: crate::retry::RetryPolicy) {
        self.retry_policy = policy;
    }

    pub fn netrc_lookup(&self, host: &str) -> Option<(String, String)> {
        self.netrc
            .as_ref()
            .and_then(|n| n.lookup(host))
            .map(|e| (e.login.clone(), e.password.clone()))
    }

        pub fn set_connector(&mut self, connector: TCPConnector) {
        self.connector = Some(Arc::new(connector));
    }

    pub fn set_trace_config(&mut self, config: TraceConfig) {
        self.trace_config = Some(Arc::new(config));
    }

    pub fn set_digest_auth(&mut self, auth: DigestAuth) {
        self.digest_auth = Some(Arc::new(AsyncMutex::new(auth)));
    }

}

impl Clone for Client {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            config: self.config.clone(),
            cookies: self.cookies.clone(),
            default_headers: self.default_headers.clone(),
            proxy_pool: self.proxy_pool.clone(),
            redirect_pool: self.redirect_pool.clone(),
            hooks: self.hooks.clone(),
            adapters: self.adapters.clone(),
            retry_policy: self.retry_policy.clone(),
            netrc: self.netrc.clone(),
            connector: self.connector.clone(),
            trace_config: self.trace_config.clone(),
            digest_auth: self.digest_auth.clone(),
        }
    }
}