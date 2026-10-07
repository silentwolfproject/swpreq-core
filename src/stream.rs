use crate::error::{Result, SwpreqError};
use futures_util::StreamExt;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct ResponseStream {
    inner: Arc<Mutex<Box<dyn futures_util::Stream<Item = Result<Vec<u8>>> + Send + Unpin>>>,
}

impl ResponseStream {
    pub async fn new(response: reqwest::Response) -> Self {
        let stream = response
            .bytes_stream()
            .map(|chunk| chunk.map(|b| b.to_vec()).map_err(|e| SwpreqError::Network(e.to_string())));

        Self {
            inner: Arc::new(Mutex::new(Box::new(stream))),
        }
    }

    pub async fn next_chunk(&self) -> Option<Result<Vec<u8>>> {
        let mut s = self.inner.lock().await;
        s.next().await
    }

    pub async fn read_all(&self) -> Result<Vec<u8>> {
        let mut out = Vec::new();
        let mut s = self.inner.lock().await;
        while let Some(chunk) = s.next().await {
            out.extend_from_slice(&chunk?);
        }
        Ok(out)
    }

    pub async fn read_text(&self) -> Result<String> {
        let bytes = self.read_all().await?;
        Ok(String::from_utf8_lossy(&bytes).to_string())
    }

    pub async fn read_json(&self) -> Result<Value> {
        let text = self.read_text().await?;
        serde_json::from_str(&text).map_err(|e| SwpreqError::Decode(e.to_string()))
    }
}

pub async fn stream_lines(response: reqwest::Response) -> Result<Vec<String>> {
    let stream = ResponseStream::new(response).await;
    let text = stream.read_text().await?;
    Ok(text.lines().map(|s| s.to_string()).collect())
}