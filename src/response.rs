use crate::error::{Result, SwpreqError};
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;
use std::sync::Arc;

pub struct Response {
    pub status_code: u16,
    pub reason: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
    pub url: String,
    pub elapsed: Duration,
    pub http_version: String,
    pub history: Vec<Response>,
    pub cookies: HashMap<String, String>,
    pub links: HashMap<String, String>,
    pub encoding: String,
    pub stream: Option<Arc<crate::stream::ResponseStream>>,
    pub request_id: Option<String>,
}

impl Response {
    pub async fn from_reqwest(resp: reqwest::Response) -> Result<Self> {
        let status_code = resp.status().as_u16();
        let reason = resp.status().canonical_reason().unwrap_or("").to_string();
        let url = resp.url().to_string();
        let http_version = match resp.version() {
            reqwest::Version::HTTP_09 => "http/0.9".to_string(),
            reqwest::Version::HTTP_10 => "http/1.0".to_string(),
            reqwest::Version::HTTP_11 => "http/1.1".to_string(),
            reqwest::Version::HTTP_2 => "http/2.0".to_string(),
            reqwest::Version::HTTP_3 => "http/3.0".to_string(),
            _ => "http/1.1".to_string(),
        };

        let mut headers = HashMap::new();
        for (k, v) in resp.headers().iter() {
            if let Ok(s) = v.to_str() {
                headers.insert(k.as_str().to_string(), s.to_string());
            }
        }

        let mut cookies = HashMap::new();
        for cookie in resp.cookies() {
            cookies.insert(cookie.name().to_string(), cookie.value().to_string());
        }

        let mut links = HashMap::new();
        if let Some(link_header) = headers.get("link").or_else(|| headers.get("Link")) {
            for part in link_header.split(',') {
                if let (Some(url_start), Some(url_end)) =
                    (part.find('<'), part.find('>'))
                {
                    let url = &part[url_start + 1..url_end];
                    if let Some(rel_start) = part.find("rel=\"") {
                        let rel_end = part[rel_start + 5..]
                            .find('"')
                            .map(|i| rel_start + 5 + i)
                            .unwrap_or(part.len());
                        let rel = &part[rel_start + 5..rel_end];
                        links.insert(rel.to_string(), url.to_string());
                    }
                }
            }
        }

        let content_type = headers
            .get("Content-Type")
            .or_else(|| headers.get("content-type"))
            .map(|s| s.as_str());

        let encoding = content_type
            .map(|ct| crate::encoding::detect_encoding(&[], Some(ct)))
            .unwrap_or_else(|| "utf-8".to_string());

        let body = resp
            .bytes()
            .await
            .map_err(|e| SwpreqError::Network(e.to_string()))?
            .to_vec();

        let request_id = headers
            .get("X-Request-ID")
            .or_else(|| headers.get("x-request-id"))
            .or_else(|| headers.get("X-Amzn-Trace-Id"))
            .cloned();

        Ok(Self {
            status_code,
            reason,
            headers,
            body,
            url,
            elapsed: Duration::from_millis(0),
            http_version,
            history: Vec::new(),
            cookies,
            links,
            encoding,
            stream: None,
            request_id,
        })
    }

    pub fn text(&self) -> Result<String> {
        let data = crate::encoding::strip_bom(&self.body);
        crate::encoding::decode_with_encoding(data, &self.encoding)
    }

    pub fn json(&self) -> Result<Value> {
        serde_json::from_slice(&self.body).map_err(|e| SwpreqError::Decode(e.to_string()))
    }

    pub fn bytes(&self) -> &[u8] {
        &self.body
    }

    pub fn header(&self, name: &str) -> Option<&String> {
        for (k, v) in &self.headers {
            if k.eq_ignore_ascii_case(name) {
                return Some(v);
            }
        }
        None
    }

    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status_code)
    }

    pub fn is_redirect(&self) -> bool {
        (300..400).contains(&self.status_code)
    }

    pub fn is_client_error(&self) -> bool {
        (400..500).contains(&self.status_code)
    }

    pub fn is_server_error(&self) -> bool {
        self.status_code >= 500
    }

    pub fn ok(&self) -> bool {
        (200..400).contains(&self.status_code)
    }

    pub fn raise_for_status(&self) -> Result<()> {
        if self.is_success() || self.is_redirect() {
            Ok(())
        } else {
            Err(SwpreqError::Network(format!(
                "HTTP {} {}",
                self.status_code, self.reason
            )))
        }
    }
    pub fn cookie(&self, name: &str) -> Option<&String> {
        self.cookies.get(name)
    }

    pub fn link(&self, rel: &str) -> Option<&String> {
        self.links.get(rel)
    }

    pub fn has_next_page(&self) -> bool {
        self.links.contains_key("next")
    }

    pub fn next_page_url(&self) -> Option<&String> {
        self.links.get("next")
    }
}