pub mod adapter;
pub mod auth;
pub mod client;
pub mod cookie;
pub mod encoding;
pub mod error;
pub mod ffi;
pub mod hooks;
pub mod http3;
pub mod json;
pub mod multipart;
pub mod netrc;
pub mod oauth;
pub mod pool;
pub mod proxy;
pub mod request;
pub mod response;
pub mod retry;
pub mod sse;
pub mod stream;
pub mod utils;
pub mod websocket;

pub use client::{Client, ClientConfig};
pub use error::{Result, SwpreqError};
pub use multipart::MultipartBuilder;
pub use request::RequestBuilder;
pub use response::Response;
pub use retry::RetryPolicy;
pub use sse::{SseEvent, SseStream};
pub use stream::ResponseStream;
pub use websocket::{WebSocketManager, WS_MANAGER};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

static RUNTIME: once_cell::sync::Lazy<tokio::runtime::Runtime> = once_cell::sync::Lazy::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .thread_name("swpreq-rt")
        .build()
        .expect("failed to build tokio runtime")
});

pub fn runtime() -> &'static tokio::runtime::Runtime {
    &RUNTIME
}

pub fn get(url: &str) -> Result<Response> {
    let client = Client::new(ClientConfig::default())?;
    client.get(url).send_sync()
}

pub fn post(url: &str) -> Result<Response> {
    let client = Client::new(ClientConfig::default())?;
    client.post(url).send_sync()
}

pub async fn aget(url: &str) -> Result<Response> {
    let client = Client::new(ClientConfig::default())?;
    client.get(url).send().await
}

pub async fn apost(url: &str) -> Result<Response> {
    let client = Client::new(ClientConfig::default())?;
    client.post(url).send().await
}