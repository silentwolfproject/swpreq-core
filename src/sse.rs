use crate::client::Client;
use crate::error::{Result, SwpreqError};
use futures_util::StreamExt;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct SseEvent {
    pub event: Option<String>,
    pub data: String,
    pub id: Option<String>,
    pub retry: Option<u64>,
}

pub struct SseStream {
    inner: Box<dyn futures_util::Stream<Item = Result<SseEvent>> + Send + Unpin>,
}

impl SseStream {
    pub async fn connect(client: Arc<Client>, url: &str, headers: Vec<(String, String)>) -> Result<Self> {
        let mut req = client.inner.get(url);
        for (k, v) in headers {
            req = req.header(&k, &v);
        }
        req = req.header("Accept", "text/event-stream");
        req = req.header("Cache-Control", "no-cache");

        let response = req
            .send()
            .await
            .map_err(|e| SwpreqError::Network(e.to_string()))?;

        let stream = response.bytes_stream();

        let parsed = stream.map(|chunk| {
            chunk
                .map_err(|e| SwpreqError::Network(e.to_string()))
                .map(|bytes| String::from_utf8_lossy(&bytes).to_string())
        });

        let events = parse_sse_stream(parsed);

        Ok(Self {
            inner: Box::new(events),
        })
    }

    pub async fn next(&mut self) -> Option<Result<SseEvent>> {
        self.inner.next().await
    }
}

fn parse_sse_stream<S>(stream: S) -> impl futures_util::Stream<Item = Result<SseEvent>> + Send + Unpin
where
    S: futures_util::Stream<Item = Result<String>> + Send + Unpin + 'static,
{
    let buffer = String::new();
    let current_event: Option<String> = None;
    let current_data = String::new();
    let current_id: Option<String> = None;
    let current_retry: Option<u64> = None;
    let pending: Vec<SseEvent> = Vec::new();

    Box::pin(futures_util::stream::unfold(
        (stream, buffer, current_event, current_data, current_id, current_retry, pending),
        |mut state| async move {
            loop {
                if let Some(evt) = state.6.pop() {
                    return Some((Ok(evt), state));
                }

                let (stream, buffer, current_event, current_data, current_id, current_retry, pending) = &mut state;

                if let Some(idx) = buffer.find('\n') {
                    let line = buffer[..idx].trim_end_matches('\r').to_string();
                    buffer.drain(..idx + 1);

                    if line.is_empty() {
                        if !current_data.is_empty() || current_event.is_some() {
                            let evt = SseEvent {
                                event: current_event.take(),
                                data: std::mem::take(current_data),
                                id: current_id.take(),
                                retry: current_retry.take(),
                            };
                            pending.push(evt);
                        }
                        continue;
                    }

                    if let Some(rest) = line.strip_prefix("event:") {
                        *current_event = Some(rest.trim().to_string());
                    } else if let Some(rest) = line.strip_prefix("data:") {
                        if !current_data.is_empty() {
                            current_data.push('\n');
                        }
                        current_data.push_str(rest.trim_start());
                    } else if let Some(rest) = line.strip_prefix("id:") {
                        *current_id = Some(rest.trim().to_string());
                    } else if let Some(rest) = line.strip_prefix("retry:") {
                        if let Ok(n) = rest.trim().parse::<u64>() {
                            *current_retry = Some(n);
                        }
                    }
                    continue;
                }

                match stream.next().await {
                    Some(Ok(chunk)) => {
                        buffer.push_str(&chunk);
                    }
                    Some(Err(e)) => {
                        return Some((Err(e), state));
                    }
                    None => {
                        if !current_data.is_empty() {
                            let evt = SseEvent {
                                event: current_event.take(),
                                data: std::mem::take(current_data),
                                id: current_id.take(),
                                retry: current_retry.take(),
                            };
                            pending.push(evt);
                            continue;
                        }
                        return None;
                    }
                }
            }
        },
    ))
}