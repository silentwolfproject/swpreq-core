use crate::error::Result;
use std::sync::Arc;

pub type TraceCallback = Arc<dyn Fn(TraceContext) -> Result<()> + Send + Sync>;

#[derive(Clone)]
pub struct TraceContext {
    pub event: TraceEvent,
    pub url: String,
    pub method: String,
    pub status: Option<u16>,
    pub error: Option<String>,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceEvent {
    RequestStart,
    RequestEnd,
    RequestRedirect,
    RequestRetry,
    ResponseStart,
    ResponseEnd,
    ResponseChunk,
    ConnectionCreate,
    ConnectionReuse,
    DnsResolve,
    DnsCacheHit,
    TlsHandshakeStart,
    TlsHandshakeEnd,
}

pub struct TraceConfig {
    pub on_request_start: Vec<TraceCallback>,
    pub on_request_end: Vec<TraceCallback>,
    pub on_request_redirect: Vec<TraceCallback>,
    pub on_request_retry: Vec<TraceCallback>,
    pub on_response_start: Vec<TraceCallback>,
    pub on_response_end: Vec<TraceCallback>,
    pub on_response_chunk: Vec<TraceCallback>,
    pub on_connection_create: Vec<TraceCallback>,
    pub on_connection_reuse: Vec<TraceCallback>,
    pub on_dns_resolve: Vec<TraceCallback>,
    pub on_dns_cache_hit: Vec<TraceCallback>,
    pub on_tls_handshake_start: Vec<TraceCallback>,
    pub on_tls_handshake_end: Vec<TraceCallback>,
}

impl Default for TraceConfig {
    fn default() -> Self {
        Self {
            on_request_start: Vec::new(),
            on_request_end: Vec::new(),
            on_request_redirect: Vec::new(),
            on_request_retry: Vec::new(),
            on_response_start: Vec::new(),
            on_response_end: Vec::new(),
            on_response_chunk: Vec::new(),
            on_connection_create: Vec::new(),
            on_connection_reuse: Vec::new(),
            on_dns_resolve: Vec::new(),
            on_dns_cache_hit: Vec::new(),
            on_tls_handshake_start: Vec::new(),
            on_tls_handshake_end: Vec::new(),
        }
    }
}

impl TraceConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn emit(&self, ctx: TraceContext) -> Result<()> {
        let callbacks = match ctx.event {
            TraceEvent::RequestStart => &self.on_request_start,
            TraceEvent::RequestEnd => &self.on_request_end,
            TraceEvent::RequestRedirect => &self.on_request_redirect,
            TraceEvent::RequestRetry => &self.on_request_retry,
            TraceEvent::ResponseStart => &self.on_response_start,
            TraceEvent::ResponseEnd => &self.on_response_end,
            TraceEvent::ResponseChunk => &self.on_response_chunk,
            TraceEvent::ConnectionCreate => &self.on_connection_create,
            TraceEvent::ConnectionReuse => &self.on_connection_reuse,
            TraceEvent::DnsResolve => &self.on_dns_resolve,
            TraceEvent::DnsCacheHit => &self.on_dns_cache_hit,
            TraceEvent::TlsHandshakeStart => &self.on_tls_handshake_start,
            TraceEvent::TlsHandshakeEnd => &self.on_tls_handshake_end,
        };

        for cb in callbacks {
            cb(ctx.clone())?;
        }

        Ok(())
    }
}