pub mod adapter;
pub mod auth_digest;
pub mod client;
pub mod connector;
pub mod cookie;
pub mod encoding;
pub mod error;
pub mod ffi;
pub mod hooks;
pub mod json;
pub mod multipart;
pub mod netrc;
pub mod oauth;
pub mod pool;
pub mod proxy;
pub mod request;
pub mod resolver;
pub mod response;
pub mod retry;
pub mod server;
pub mod server_middleware;
pub mod server_router;
pub mod server_static;
pub mod server_ws;
pub mod signals;
pub mod sse;
pub mod stream;
pub mod stream_body;
pub mod trace;
pub mod unix_socket;
pub mod utils;
pub mod websocket;

pub use auth_digest::DigestAuth;
pub use client::{Client, ClientConfig};
pub use connector::{SocketFamily, TCPConnector};
pub use error::{Result, SwpreqError};
pub use multipart::MultipartBuilder;
pub use request::RequestBuilder;
pub use resolver::AsyncResolver;
pub use response::Response;
pub use retry::RetryPolicy;
pub use server::Server;
pub use sse::{SseEvent, SseStream};
pub use stream::ResponseStream;
pub use stream_body::StreamBody;
pub use trace::{TraceConfig, TraceContext, TraceEvent};
pub use websocket::{WebSocketManager, WS_MANAGER};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

static RUNTIME: once_cell::sync::Lazy<tokio::runtime::Runtime> = once_cell::sync::Lazy::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(num_cpus())
        .enable_all()
        .thread_name("swpreq-rt")
        .build()
        .expect("failed to build tokio runtime")
});

fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .max(2)
}

pub fn runtime() -> &'static tokio::runtime::Runtime {
    &RUNTIME
}