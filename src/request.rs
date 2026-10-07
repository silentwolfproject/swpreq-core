use crate::client::Client;
use crate::error::{Result, SwpreqError};
use crate::response::Response;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::Method;
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;

pub struct RequestBuilder {
    client: Arc<Client>,
    method: Method,
    url: String,
    headers: HeaderMap,
    body: Option<Vec<u8>>,
    json_body: Option<Value>,
    form_body: Option<Vec<(String, String)>>,
    multipart: Option<reqwest::multipart::Form>,
    timeout: Option<Duration>,
    proxy: Option<String>,
    redirect_limit: Option<usize>,
    basic_auth: Option<(String, String)>,
    bearer_auth: Option<String>,
    follow_redirects: Option<bool>,
    query_params: Vec<(String, String)>,
    retry_policy: Option<crate::retry::RetryPolicy>,
    stream_mode: bool,
    force_encoding: Option<String>,
}

impl RequestBuilder {
    pub fn new(client: Arc<Client>, method: &str, url: &str) -> Self {
        let m = Method::from_bytes(method.as_bytes()).unwrap_or(Method::GET);
        Self {
            client,
            method: m,
            url: url.to_string(),
            headers: HeaderMap::new(),
            body: None,
            json_body: None,
            form_body: None,
            multipart: None,
            timeout: None,
            proxy: None,
            redirect_limit: None,
            basic_auth: None,
            bearer_auth: None,
            follow_redirects: None,
            query_params: Vec::new(),
            retry_policy: None,
            stream_mode: false,
            force_encoding: None,
        }
    }

    pub fn header(&mut self, key: &str, value: &str) -> Result<&mut Self> {
        let name = HeaderName::from_bytes(key.as_bytes())
            .map_err(|e| SwpreqError::InvalidArgument(e.to_string()))?;
        let val = HeaderValue::from_str(value)
            .map_err(|e| SwpreqError::InvalidArgument(e.to_string()))?;
        self.headers.insert(name, val);
        Ok(self)
    }

    pub fn headers(&mut self, headers: Vec<(String, String)>) -> Result<&mut Self> {
        for (k, v) in headers {
            let name = HeaderName::from_bytes(k.as_bytes())
                .map_err(|e| SwpreqError::InvalidArgument(e.to_string()))?;
            let val = HeaderValue::from_str(&v)
                .map_err(|e| SwpreqError::InvalidArgument(e.to_string()))?;
            self.headers.insert(name, val);
        }
        Ok(self)
    }

    pub fn query(&mut self, key: &str, value: &str) -> &mut Self {
        self.query_params.push((key.to_string(), value.to_string()));
        self
    }

    pub fn params(&mut self, params: Vec<(String, String)>) -> &mut Self {
        self.query_params.extend(params);
        self
    }

    pub fn body(&mut self, body: Vec<u8>) -> &mut Self {
        self.body = Some(body);
        self
    }

    pub fn text(&mut self, text: &str) -> &mut Self {
        self.body = Some(text.as_bytes().to_vec());
        self
    }

    pub fn json(&mut self, value: Value) -> &mut Self {
        self.json_body = Some(value);
        self
    }

    pub fn form(&mut self, fields: Vec<(String, String)>) -> &mut Self {
        self.form_body = Some(fields);
        self
    }

    pub fn multipart(&mut self, form: reqwest::multipart::Form) -> &mut Self {
        self.multipart = Some(form);
        self
    }

    pub fn timeout(&mut self, duration: Duration) -> &mut Self {
        self.timeout = Some(duration);
        self
    }

    pub fn proxy(&mut self, proxy: &str) -> &mut Self {
        self.proxy = Some(proxy.to_string());
        self
    }

    pub fn follow_redirects(&mut self, follow: bool) -> &mut Self {
        self.follow_redirects = Some(follow);
        self
    }

    pub fn max_redirects(&mut self, limit: usize) -> &mut Self {
        self.redirect_limit = Some(limit);
        self
    }

    pub fn basic_auth(&mut self, username: &str, password: &str) -> &mut Self {
        self.basic_auth = Some((username.to_string(), password.to_string()));
        self
    }

    pub fn bearer_auth(&mut self, token: &str) -> &mut Self {
        self.bearer_auth = Some(token.to_string());
        self
    }

    fn effective_redirect_limit(&self) -> Option<usize> {
        if let Some(false) = self.follow_redirects {
            return Some(0);
        }
        if let Some(limit) = self.redirect_limit {
            return Some(limit);
        }
        if let Some(true) = self.follow_redirects {
            return Some(self.client.config.redirect_limit);
        }
        None
    }

    pub async fn send(self) -> Result<Response> {
        let mut url = url::Url::parse(&self.url)
            .map_err(|e| SwpreqError::InvalidUrl(e.to_string()))?;

        if !self.query_params.is_empty() {
            let mut qp = url.query_pairs_mut();
            for (k, v) in &self.query_params {
                qp.append_pair(k, v);
            }
        }

        let redirect_limit = self.effective_redirect_limit();
        let client_arc = self
            .client
            .get_client_for_request(self.proxy.as_deref(), redirect_limit)?;

        let mut req = client_arc.request(self.method.clone(), url);

        for (k, v) in self.client.default_headers.read().iter() {
            req = req.header(k.clone(), v.clone());
        }

        for (k, v) in self.headers.iter() {
            req = req.header(k.clone(), v.clone());
        }

        if let Some(t) = self.timeout {
            req = req.timeout(t);
        }

        if let Some((ref u, ref pw)) = self.basic_auth {
            req = req.basic_auth(u.clone(), Some(pw.clone()));
        }

        if let Some(ref token) = self.bearer_auth {
            req = req.bearer_auth(token.clone());
        }

        if let Some(j) = self.json_body {
            req = req.json(&j);
        } else if let Some(f) = self.form_body {
            req = req.form(&f);
        } else if let Some(m) = self.multipart {
            req = req.multipart(m);
        } else if let Some(b) = self.body {
            req = req.body(b);
        }

        let start = std::time::Instant::now();
        let resp = req.send().await.map_err(|e| {
            if e.is_timeout() {
                SwpreqError::Timeout(e.to_string())
            } else {
                SwpreqError::Network(e.to_string())
            }
        })?;

        let mut response = Response::from_reqwest(resp).await?;
        response.elapsed = start.elapsed();
        Ok(response)
    }

    pub fn send_sync(self) -> Result<Response> {
        crate::runtime().block_on(self.send())
    }

        pub fn retry(mut self, policy: crate::retry::RetryPolicy) -> Self {
        self.retry_policy = Some(policy);
        self
    }

    pub fn stream(mut self, enabled: bool) -> Self {
        self.stream_mode = enabled;
        self
    }

    pub fn files(mut self, form: reqwest::multipart::Form) -> Self {
        self.multipart = Some(form);
        self
    }

    pub fn encoding(mut self, enc: &str) -> Self {
        self.force_encoding = Some(enc.to_string());
        self
    }
}