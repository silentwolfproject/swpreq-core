use crate::error::Result;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct OAuth1Params {
    pub consumer_key: String,
    pub consumer_secret: String,
    pub token: Option<String>,
    pub token_secret: Option<String>,
    pub signature_method: String,
    pub timestamp: Option<u64>,
    pub nonce: Option<String>,
    pub version: String,
}

impl Default for OAuth1Params {
    fn default() -> Self {
        Self {
            consumer_key: String::new(),
            consumer_secret: String::new(),
            token: None,
            token_secret: None,
            signature_method: "HMAC-SHA1".to_string(),
            timestamp: None,
            nonce: None,
            version: "1.0".to_string(),
        }
    }
}

pub struct OAuth1;

impl OAuth1 {
    pub fn generate_header(
        params: &OAuth1Params,
        method: &str,
        url: &str,
        extra: Option<&HashMap<String, String>>,
    ) -> Result<String> {
        let timestamp = params
            .timestamp
            .unwrap_or_else(|| chrono_now());
        let nonce = params
            .nonce
            .clone()
            .unwrap_or_else(|| random_nonce());

        let mut oauth_params = HashMap::new();
        oauth_params.insert("oauth_consumer_key".to_string(), params.consumer_key.clone());
        oauth_params.insert("oauth_nonce".to_string(), nonce);
        oauth_params.insert("oauth_signature_method".to_string(), params.signature_method.clone());
        oauth_params.insert("oauth_timestamp".to_string(), timestamp.to_string());
        oauth_params.insert("oauth_version".to_string(), params.version.clone());

        if let Some(ref token) = params.token {
            oauth_params.insert("oauth_token".to_string(), token.clone());
        }

        if let Some(extra) = extra {
            for (k, v) in extra {
                oauth_params.insert(k.clone(), v.clone());
            }
        }

        let signature = Self::compute_signature(
            method,
            url,
            &oauth_params,
            &params.consumer_secret,
            params.token_secret.as_deref(),
        )?;

        oauth_params.insert("oauth_signature".to_string(), signature);

        let mut parts: Vec<String> = oauth_params
            .iter()
            .map(|(k, v)| format!("{}=\"{}\"", url_encode(k), url_encode(v)))
            .collect();
        parts.sort();

        Ok(format!("OAuth {}", parts.join(", ")))
    }

    fn compute_signature(
        method: &str,
        url: &str,
        params: &HashMap<String, String>,
        consumer_secret: &str,
        token_secret: Option<&str>,
    ) -> Result<String> {
        let mut sorted: Vec<(&String, &String)> = params.iter().collect();
        sorted.sort();

        let param_string: String = sorted
            .iter()
            .map(|(k, v)| format!("{}={}", url_encode(k), url_encode(v)))
            .collect::<Vec<_>>()
            .join("&");

        let base_string = format!(
            "{}&{}&{}",
            method.to_uppercase(),
            url_encode(url),
            url_encode(&param_string)
        );

        let signing_key = format!(
            "{}&{}",
            url_encode(consumer_secret),
            url_encode(token_secret.unwrap_or(""))
        );

        let signature = hmac_sha1(signing_key.as_bytes(), base_string.as_bytes());

        Ok(URL_SAFE_NO_PAD.encode(signature))
    }
}

fn chrono_now() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn random_nonce() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    format!("{:x}", now)
}

fn url_encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(b as char);
            }
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

fn hmac_sha1(key: &[u8], data: &[u8]) -> Vec<u8> {
    use hmac::{Hmac, Mac};
    use sha1::Sha1;
    type HmacSha1 = Hmac<Sha1>;

    let mut mac = HmacSha1::new_from_slice(key).expect("HMAC key");
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}