use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Cookie {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
}

pub struct CookieJar {
    cookies: HashMap<String, Cookie>,
}

impl CookieJar {
    pub fn new() -> Self {
        Self {
            cookies: HashMap::new(),
        }
    }

    pub fn set(&mut self, name: &str, value: &str, domain: &str) {
        let key = format!("{}@{}", domain, name);
        self.cookies.insert(
            key,
            Cookie {
                name: name.to_string(),
                value: value.to_string(),
                domain: domain.to_string(),
                path: "/".to_string(),
            },
        );
    }

    pub fn get(&self, name: &str, domain: &str) -> Option<&Cookie> {
        let key = format!("{}@{}", domain, name);
        self.cookies.get(&key)
    }

    pub fn get_for_domain(&self, domain: &str) -> Vec<&Cookie> {
        self.cookies.values().filter(|c| c.domain == domain).collect()
    }

    pub fn remove(&mut self, name: &str, domain: &str) {
        let key = format!("{}@{}", domain, name);
        self.cookies.remove(&key);
    }

    pub fn clear(&mut self) {
        self.cookies.clear();
    }

    pub fn len(&self) -> usize {
        self.cookies.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cookies.is_empty()
    }
}

impl Default for CookieJar {
    fn default() -> Self {
        Self::new()
    }
}