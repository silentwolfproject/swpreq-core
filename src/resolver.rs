use crate::error::{Result, SwpreqError};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct AsyncResolver {
    nameservers: Vec<String>,
    cache: Arc<RwLock<HashMap<String, CacheEntry>>>,
    ttl: Duration,
}

#[derive(Clone)]
struct CacheEntry {
    addresses: Vec<IpAddr>,
    expires_at: Instant,
}

impl AsyncResolver {
    pub fn new(nameservers: Vec<String>) -> Self {
        Self {
            nameservers,
            cache: Arc::new(RwLock::new(HashMap::new())),
            ttl: Duration::from_secs(300),
        }
    }

    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.ttl = ttl;
        self
    }

    pub async fn resolve(&self, hostname: &str) -> Result<Vec<IpAddr>> {
        {
            let cache = self.cache.read().await;
            if let Some(entry) = cache.get(hostname) {
                if entry.expires_at > Instant::now() {
                    return Ok(entry.addresses.clone());
                }
            }
        }

        let addresses = self.lookup(hostname).await?;
        let entry = CacheEntry {
            addresses: addresses.clone(),
            expires_at: Instant::now() + self.ttl,
        };

        self.cache.write().await.insert(hostname.to_string(), entry);
        Ok(addresses)
    }

    async fn lookup(&self, hostname: &str) -> Result<Vec<IpAddr>> {
        use tokio::net::lookup_host;

        let addresses: Vec<IpAddr> = lookup_host(format!("{}:0", hostname))
            .await
            .map_err(|e| SwpreqError::Network(e.to_string()))?
            .map(|addr| addr.ip())
            .collect();

        if addresses.is_empty() {
            return Err(SwpreqError::NotFound(format!(
                "cannot resolve: {}",
                hostname
            )));
        }

        Ok(addresses)
    }

    pub async fn clear_cache(&self) {
        self.cache.write().await.clear();
    }

    pub fn nameservers(&self) -> &[String] {
        &self.nameservers
    }
}