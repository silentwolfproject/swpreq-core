use crate::client::Client;
use crate::error::Result;
use std::collections::HashMap;
use std::sync::Arc;

pub trait TransportAdapter: Send + Sync {
    fn name(&self) -> &str;
    fn prefix(&self) -> &str;
    fn get_client(&self) -> Result<Arc<Client>>;
}

pub struct AdapterRegistry {
    adapters: HashMap<String, Arc<dyn TransportAdapter>>,
}

impl AdapterRegistry {
    pub fn new() -> Self {
        Self {
            adapters: HashMap::new(),
        }
    }

    pub fn mount(&mut self, prefix: &str, adapter: Arc<dyn TransportAdapter>) {
        self.adapters.insert(prefix.to_string(), adapter);
    }

    pub fn unmount(&mut self, prefix: &str) {
        self.adapters.remove(prefix);
    }

    pub fn find(&self, url: &str) -> Option<Arc<dyn TransportAdapter>> {
        let mut best: Option<(usize, Arc<dyn TransportAdapter>)> = None;
        for (prefix, adapter) in &self.adapters {
            if url.starts_with(prefix) {
                let len = prefix.len();
                match &best {
                    Some((best_len, _)) if *best_len >= len => {}
                    _ => best = Some((len, adapter.clone())),
                }
            }
        }
        best.map(|(_, a)| a)
    }

    pub fn resolve_client(&self, url: &str, default: Arc<Client>) -> Result<Arc<Client>> {
        match self.find(url) {
            Some(adapter) => adapter.get_client(),
            None => Ok(default),
        }
    }

    pub fn clear(&mut self) {
        self.adapters.clear();
    }

    pub fn list(&self) -> Vec<(String, String)> {
        self.adapters
            .iter()
            .map(|(k, v)| (k.clone(), v.name().to_string()))
            .collect()
    }
}

impl Default for AdapterRegistry {
    fn default() -> Self {
        Self::new()
    }
}

pub struct DefaultAdapter {
    name: String,
    prefix: String,
    client: Arc<Client>,
}

impl DefaultAdapter {
    pub fn new(name: &str, prefix: &str, client: Arc<Client>) -> Self {
        Self {
            name: name.to_string(),
            prefix: prefix.to_string(),
            client,
        }
    }
}

impl TransportAdapter for DefaultAdapter {
    fn name(&self) -> &str {
        &self.name
    }

    fn prefix(&self) -> &str {
        &self.prefix
    }

    fn get_client(&self) -> Result<Arc<Client>> {
        Ok(self.client.clone())
    }
}