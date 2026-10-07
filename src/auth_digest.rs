use crate::error::{Result, SwpreqError};
use md5::{Digest as Md5Digest, Md5};
use rand::Rng;
use sha2::Sha256;

#[derive(Debug, Clone)]
pub struct DigestAuth {
    pub username: String,
    pub password: String,
    pub realm: String,
    pub nonce: String,
    pub qop: Option<String>,
    pub nc: u32,
    pub cnonce: String,
    pub opaque: Option<String>,
    pub algorithm: String,
}

impl DigestAuth {
    pub fn new(username: &str, password: &str) -> Self {
        Self {
            username: username.to_string(),
            password: password.to_string(),
            realm: String::new(),
            nonce: String::new(),
            qop: None,
            nc: 0,
            cnonce: Self::generate_cnonce(),
            opaque: None,
            algorithm: "MD5".to_string(),
        }
    }

    pub fn from_challenge(
        username: &str,
        password: &str,
        www_authenticate: &str,
    ) -> Result<Self> {
        let mut auth = Self::new(username, password);
        auth.parse_challenge(www_authenticate)?;
        Ok(auth)
    }

    pub fn build_header(&mut self, method: &str, uri: &str) -> Result<String> {
        if self.realm.is_empty() || self.nonce.is_empty() {
            return Err(SwpreqError::Auth("digest challenge not parsed".to_string()));
        }

        self.nc += 1;
        let nc_str = format!("{:08x}", self.nc);

        let ha1 = self.hash(&format!(
            "{}:{}:{}",
            self.username, self.realm, self.password
        ));
        let ha2 = self.hash(&format!("{}:{}", method, uri));

        let response = if let Some(ref qop) = self.qop {
            self.hash(&format!(
                "{}:{}:{}:{}:{}:{}",
                ha1, self.nonce, nc_str, self.cnonce, qop, ha2
            ))
        } else {
            self.hash(&format!("{}:{}:{}", ha1, self.nonce, ha2))
        };

        let mut parts = vec![
            format!("username=\"{}\"", self.username),
            format!("realm=\"{}\"", self.realm),
            format!("nonce=\"{}\"", self.nonce),
            format!("uri=\"{}\"", uri),
            format!("response=\"{}\"", response),
        ];

        if self.algorithm != "MD5" {
            parts.push(format!("algorithm={}", self.algorithm));
        }

        if let Some(ref opaque) = self.opaque {
            parts.push(format!("opaque=\"{}\"", opaque));
        }

        if let Some(ref qop) = self.qop {
            parts.push(format!("qop={}", qop));
            parts.push(format!("nc={}", nc_str));
            parts.push(format!("cnonce=\"{}\"", self.cnonce));
        }

        Ok(format!("Digest {}", parts.join(", ")))
    }

    fn hash(&self, input: &str) -> String {
        match self.algorithm.to_uppercase().as_str() {
            "MD5" | "MD5-SESS" => {
                let mut h = Md5::new();
                h.update(input.as_bytes());
                hex::encode(h.finalize())
            }
            "SHA-256" | "SHA-256-SESS" => {
                let mut h = Sha256::new();
                h.update(input.as_bytes());
                hex::encode(h.finalize())
            }
            _ => {
                let mut h = Md5::new();
                h.update(input.as_bytes());
                hex::encode(h.finalize())
            }
        }
    }

    fn parse_challenge(&mut self, header: &str) -> Result<()> {
        let header = header.trim_start_matches("Digest").trim();

        for part in split_digest_params(header) {
            if let Some((key, value)) = part.split_once('=') {
                let key = key.trim().to_lowercase();
                let value = value.trim().trim_matches('"').trim_matches('\'');

                match key.as_str() {
                    "realm" => self.realm = value.to_string(),
                    "nonce" => self.nonce = value.to_string(),
                    "qop" => {
                        let opts: Vec<&str> = value.split(',').map(|s| s.trim()).collect();
                        self.qop = opts.first().map(|s| s.to_string());
                    }
                    "opaque" => self.opaque = Some(value.to_string()),
                    "algorithm" => self.algorithm = value.to_string(),
                    _ => {}
                }
            }
        }

        Ok(())
    }

    fn generate_cnonce() -> String {
        let bytes: [u8; 16] = rand::thread_rng().gen();
        hex::encode(bytes)
    }
}

fn split_digest_params(s: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;

    for c in s.chars() {
        match c {
            '"' => {
                in_quotes = !in_quotes;
                current.push(c);
            }
            ',' if !in_quotes => {
                parts.push(current.trim().to_string());
                current = String::new();
            }
            _ => current.push(c),
        }
    }

    if !current.trim().is_empty() {
        parts.push(current.trim().to_string());
    }

    parts
}