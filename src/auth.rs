use base64::engine::general_purpose::STANDARD;
use base64::Engine;

pub fn basic_auth_header(username: &str, password: &str) -> String {
    let credentials = format!("{}:{}", username, password);
    let encoded = STANDARD.encode(credentials.as_bytes());
    format!("Basic {}", encoded)
}

pub fn bearer_auth_header(token: &str) -> String {
    format!("Bearer {}", token)
}

pub fn digest_auth_header(
    username: &str,
    password: &str,
    realm: &str,
    nonce: &str,
    uri: &str,
    method: &str,
) -> String {
    let ha1_input = format!("{}:{}:{}", username, realm, password);
    let ha1 = blake3::hash(ha1_input.as_bytes()).to_hex().to_string();
    let ha2_input = format!("{}:{}", method, uri);
    let ha2 = blake3::hash(ha2_input.as_bytes()).to_hex().to_string();
    let response_input = format!("{}:{}:{}", ha1, nonce, ha2);
    let response = blake3::hash(response_input.as_bytes()).to_hex().to_string();

    format!(
        "Digest username=\"{}\", realm=\"{}\", nonce=\"{}\", uri=\"{}\", response=\"{}\"",
        username, realm, nonce, uri, response
    )
}