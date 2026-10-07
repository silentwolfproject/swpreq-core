use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

pub struct ConnectionStats {
    pub total_requests: AtomicU64,
    pub active_connections: AtomicU64,
    pub idle_connections: AtomicU64,
    pub failed_requests: AtomicU64,
    pub retried_requests: AtomicU64,
}

impl ConnectionStats {
    pub fn new() -> Self {
        Self {
            total_requests: AtomicU64::new(0),
            active_connections: AtomicU64::new(0),
            idle_connections: AtomicU64::new(0),
            failed_requests: AtomicU64::new(0),
            retried_requests: AtomicU64::new(0),
        }
    }

    pub fn snapshot(&self) -> HashMap<String, u64> {
        let mut m = HashMap::new();
        m.insert(
            "total_requests".to_string(),
            self.total_requests.load(Ordering::Relaxed),
        );
        m.insert(
            "active_connections".to_string(),
            self.active_connections.load(Ordering::Relaxed),
        );
        m.insert(
            "idle_connections".to_string(),
            self.idle_connections.load(Ordering::Relaxed),
        );
        m.insert(
            "failed_requests".to_string(),
            self.failed_requests.load(Ordering::Relaxed),
        );
        m.insert(
            "retried_requests".to_string(),
            self.retried_requests.load(Ordering::Relaxed),
        );
        m
    }
}

impl Default for ConnectionStats {
    fn default() -> Self {
        Self::new()
    }
}

pub type SharedStats = Arc<ConnectionStats>;