use crate::error::{Result, SwpreqError};

pub fn proxy_from_env() -> Option<String> {
    let keys = [
        "HTTPS_PROXY", "https_proxy",
        "HTTP_PROXY", "http_proxy",
        "ALL_PROXY", "all_proxy",
    ];

    for key in &keys {
        if let Ok(val) = std::env::var(key) {
            if !val.is_empty() {
                return Some(val);
            }
        }
    }

    None
}

pub fn no_proxy_from_env() -> Vec<String> {
    let keys = ["NO_PROXY", "no_proxy"];

    for key in &keys {
        if let Ok(val) = std::env::var(key) {
            if !val.is_empty() {
                return val
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
        }
    }

    Vec::new()
}

pub fn should_bypass_proxy(url: &str, no_proxy: &[String]) -> bool {
    if no_proxy.is_empty() {
        return false;
    }

    let host = match url::Url::parse(url) {
        Ok(u) => match u.host_str() {
            Some(h) => h.to_string(),
            None => return false,
        },
        Err(_) => return false,
    };

    for pattern in no_proxy {
        if pattern == "*" {
            return true;
        }
        if pattern.starts_with('.') {
            if host.ends_with(pattern) {
                return true;
            }
        } else if host == *pattern || host.ends_with(&format!(".{}", pattern)) {
            return true;
        }
    }

    false
}

pub fn resolve_proxy(explicit: Option<&str>, url: &str) -> Result<Option<String>> {
    if let Some(p) = explicit {
        return Ok(Some(p.to_string()));
    }

    let no_proxy = no_proxy_from_env();
    if should_bypass_proxy(url, &no_proxy) {
        return Ok(None);
    }

    Ok(proxy_from_env())
}

pub fn validate_proxy(proxy: &str) -> Result<()> {
    let valid_schemes = [
        "http://", "https://",
        "socks4://", "socks4a://",
        "socks5://", "socks5h://",
    ];

    if !valid_schemes.iter().any(|s| proxy.starts_with(s)) {
        return Err(SwpreqError::Proxy(format!(
            "unsupported proxy scheme: {}",
            proxy
        )));
    }

    Ok(())
}