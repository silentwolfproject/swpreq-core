use crate::error::Result;

pub fn detect_encoding(data: &[u8], content_type: Option<&str>) -> String {
    if let Some(ct) = content_type {
        if let Some(enc) = parse_charset(ct) {
            return enc;
        }
    }

    if let Some(enc) = detect_bom(data) {
        return enc.to_string();
    }

    if is_valid_utf8(data) {
        return "utf-8".to_string();
    }

    "iso-8859-1".to_string()
}

pub fn decode_with_encoding(data: &[u8], encoding: &str) -> Result<String> {
    match encoding.to_lowercase().as_str() {
        "utf-8" | "utf8" => Ok(String::from_utf8_lossy(data).to_string()),
        "iso-8859-1" | "latin1" => Ok(data.iter().map(|&b| b as char).collect()),
        "ascii" | "us-ascii" => {
            if data.iter().all(|&b| b < 128) {
                Ok(String::from_utf8_lossy(data).to_string())
            } else {
                Ok(String::from_utf8_lossy(data).to_string())
            }
        }
        _ => Ok(String::from_utf8_lossy(data).to_string()),
    }
}

fn parse_charset(content_type: &str) -> Option<String> {
    for part in content_type.split(';') {
        let part = part.trim();
        if part.to_lowercase().starts_with("charset=") {
            let enc = part[8..].trim().trim_matches('"').trim_matches('\'');
            return Some(enc.to_lowercase());
        }
    }
    None
}

fn detect_bom(data: &[u8]) -> Option<&'static str> {
    if data.len() >= 3 && data[0] == 0xEF && data[1] == 0xBB && data[2] == 0xBF {
        return Some("utf-8");
    }
    if data.len() >= 2 && data[0] == 0xFF && data[1] == 0xFE {
        return Some("utf-16-le");
    }
    if data.len() >= 2 && data[0] == 0xFE && data[1] == 0xFF {
        return Some("utf-16-be");
    }
    None
}

fn is_valid_utf8(data: &[u8]) -> bool {
    std::str::from_utf8(data).is_ok()
}

pub fn strip_bom(data: &[u8]) -> &[u8] {
    if data.len() >= 3 && data[0] == 0xEF && data[1] == 0xBB && data[2] == 0xBF {
        &data[3..]
    } else {
        data
    }
}