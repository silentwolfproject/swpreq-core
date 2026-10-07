use crate::error::{Result, SwpreqError};
use serde_json::Value;

pub fn parse_json(data: &[u8]) -> Result<Value> {
    serde_json::from_slice(data).map_err(|e| SwpreqError::Decode(e.to_string()))
}

pub fn to_json(value: &Value) -> Result<Vec<u8>> {
    serde_json::to_vec(value).map_err(|e| SwpreqError::Encode(e.to_string()))
}

pub fn to_json_pretty(value: &Value) -> Result<String> {
    serde_json::to_string_pretty(value).map_err(|e| SwpreqError::Encode(e.to_string()))
}