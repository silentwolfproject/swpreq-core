use crate::error::{Result, SwpreqError};
use futures_util::{SinkExt, StreamExt};
use std::os::raw::{c_char, c_int, c_void};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

pub type WsStream = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

pub type MessageCallback = extern "C" fn(
    user_data: *mut c_void,
    msg_type: c_int,
    data: *const u8,
    data_len: usize,
);

pub type EventCallback =
    extern "C" fn(user_data: *mut c_void, code: c_int, reason: *const c_char);

pub struct WebSocketHandle {
    pub id: usize,
    pub sender: mpsc::UnboundedSender<WsMessage>,
    pub running: Arc<AtomicBool>,
    pub callbacks: Arc<Mutex<Callbacks>>,
    pub state: Arc<Mutex<WsState>>,
}

#[derive(Default)]
pub struct Callbacks {
    pub on_open: Option<(EventCallback, usize)>,
    pub on_message: Option<(MessageCallback, usize)>,
    pub on_error: Option<(MessageCallback, usize)>,
    pub on_close: Option<(EventCallback, usize)>,
    pub user_data: usize,
}

#[derive(Default)]
pub struct WsState {
    pub connected: bool,
    pub closed: bool,
}

pub struct WebSocketManager {
    next_id: Arc<AtomicUsize>,
    handles: Arc<Mutex<std::collections::HashMap<usize, Arc<WebSocketHandle>>>>,
}

impl WebSocketManager {
    pub fn new() -> Self {
        Self {
            next_id: Arc::new(AtomicUsize::new(1)),
            handles: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
    }

    pub async fn connect(
        self: &Arc<Self>,
        url: &str,
        headers: Vec<(String, String)>,
    ) -> Result<usize> {
        let mut request = url
            .into_client_request()
            .map_err(|e| SwpreqError::InvalidUrl(format!("{}", e)))?;

        {
            let req_headers = request.headers_mut();
            for (k, v) in headers {
                if let (Ok(name), Ok(value)) = (
                    tokio_tungstenite::tungstenite::http::header::HeaderName::from_bytes(
                        k.as_bytes(),
                    ),
                    tokio_tungstenite::tungstenite::http::header::HeaderValue::from_str(&v),
                ) {
                    req_headers.insert(name, value);
                }
            }
        }

        let (stream, _) = connect_async(request)
            .await
            .map_err(|e| SwpreqError::Network(format!("{}", e)))?;

        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let (sender, mut receiver) = mpsc::unbounded_channel::<WsMessage>();
        let running = Arc::new(AtomicBool::new(true));
        let callbacks = Arc::new(Mutex::new(Callbacks::default()));
        let state = Arc::new(Mutex::new(WsState {
            connected: true,
            closed: false,
        }));

        let (mut write, mut read) = stream.split();

        let callbacks_clone = callbacks.clone();
        let running_clone = running.clone();
        let state_clone = state.clone();

        tokio::spawn(async move {
            while let Some(msg) = receiver.recv().await {
                if write.send(msg).await.is_err() {
                    break;
                }
            }
        });

        tokio::spawn(async move {
            while running_clone.load(Ordering::SeqCst) {
                match read.next().await {
                    Some(Ok(msg)) => {
                        handle_incoming(msg, &callbacks_clone).await;
                    }
                    Some(Err(e)) => {
                        let cbs = callbacks_clone.lock().await;
                        if let Some((cb, ud)) = cbs.on_error {
                            let err_msg = format!("{}", e);
                            let c_str = std::ffi::CString::new(err_msg.clone())
                                .unwrap_or_else(|_| std::ffi::CString::new("").unwrap());
                            cb(
                                ud as *mut c_void,
                                1,
                                c_str.as_ptr() as *const u8,
                                err_msg.len(),
                            );
                        }
                        break;
                    }
                    None => break,
                }
            }

            {
                let mut st = state_clone.lock().await;
                st.closed = true;
                st.connected = false;
            }

            let cbs = callbacks_clone.lock().await;
            if let Some((cb, ud)) = cbs.on_close {
                let reason = std::ffi::CString::new("").unwrap();
                cb(ud as *mut c_void, 1000, reason.as_ptr());
            }
        });

        let handle = Arc::new(WebSocketHandle {
            id,
            sender,
            running,
            callbacks,
            state,
        });

        {
            let cbs = handle.callbacks.lock().await;
            if let Some((cb, ud)) = cbs.on_open {
                let reason = std::ffi::CString::new("").unwrap();
                cb(ud as *mut c_void, 0, reason.as_ptr());
            }
        }

        self.handles.lock().await.insert(id, handle);

        Ok(id)
    }

    pub async fn get(&self, id: usize) -> Option<Arc<WebSocketHandle>> {
        self.handles.lock().await.get(&id).cloned()
    }

    pub async fn remove(&self, id: usize) {
        self.handles.lock().await.remove(&id);
    }

    pub async fn send_text(&self, id: usize, text: &str) -> Result<()> {
        let handle = self
            .get(id)
            .await
            .ok_or_else(|| SwpreqError::NotFound(format!("ws id {}", id)))?;
        handle
            .sender
            .send(WsMessage::Text(text.to_string()))
            .map_err(|e| SwpreqError::Network(format!("{}", e)))?;
        Ok(())
    }

    pub async fn send_binary(&self, id: usize, data: Vec<u8>) -> Result<()> {
        let handle = self
            .get(id)
            .await
            .ok_or_else(|| SwpreqError::NotFound(format!("ws id {}", id)))?;
        handle
            .sender
            .send(WsMessage::Binary(data))
            .map_err(|e| SwpreqError::Network(format!("{}", e)))?;
        Ok(())
    }

    pub async fn send_ping(&self, id: usize) -> Result<()> {
        let handle = self
            .get(id)
            .await
            .ok_or_else(|| SwpreqError::NotFound(format!("ws id {}", id)))?;
        handle
            .sender
            .send(WsMessage::Ping(Vec::new()))
            .map_err(|e| SwpreqError::Network(format!("{}", e)))?;
        Ok(())
    }

    pub async fn close(&self, id: usize) -> Result<()> {
        let handle = self
            .get(id)
            .await
            .ok_or_else(|| SwpreqError::NotFound(format!("ws id {}", id)))?;
        handle.running.store(false, Ordering::SeqCst);
        handle
            .sender
            .send(WsMessage::Close(None))
            .map_err(|e| SwpreqError::Network(format!("{}", e)))?;
        Ok(())
    }

    pub async fn set_callbacks(
        &self,
        id: usize,
        on_open: Option<(EventCallback, usize)>,
        on_message: Option<(MessageCallback, usize)>,
        on_error: Option<(MessageCallback, usize)>,
        on_close: Option<(EventCallback, usize)>,
        user_data: usize,
    ) -> Result<()> {
        let handle = self
            .get(id)
            .await
            .ok_or_else(|| SwpreqError::NotFound(format!("ws id {}", id)))?;

        let mut cbs = handle.callbacks.lock().await;
        cbs.on_open = on_open;
        cbs.on_message = on_message;
        cbs.on_error = on_error;
        cbs.on_close = on_close;
        cbs.user_data = user_data;

        Ok(())
    }
}

async fn handle_incoming(msg: WsMessage, callbacks: &Arc<Mutex<Callbacks>>) {
    let cbs = callbacks.lock().await;

    match msg {
        WsMessage::Text(text) => {
            if let Some((cb, ud)) = cbs.on_message {
                let bytes = text.as_bytes();
                cb(ud as *mut c_void, 0, bytes.as_ptr(), bytes.len());
            }
        }
        WsMessage::Binary(data) => {
            if let Some((cb, ud)) = cbs.on_message {
                cb(ud as *mut c_void, 1, data.as_ptr(), data.len());
            }
        }
        WsMessage::Ping(_) => {}
        WsMessage::Pong(_) => {}
        WsMessage::Close(c) => {
            if let Some((cb, ud)) = cbs.on_close {
                let (code, reason) = match c {
                    Some(frame) => (u16::from(frame.code) as c_int, frame.reason.to_string()),
                    None => (1000, String::new()),
                };
                let reason_c = std::ffi::CString::new(reason)
                    .unwrap_or_else(|_| std::ffi::CString::new("").unwrap());
                cb(ud as *mut c_void, code, reason_c.as_ptr());
            }
        }
        WsMessage::Frame(_) => {}
    }
}

pub static WS_MANAGER: once_cell::sync::Lazy<Arc<WebSocketManager>> =
    once_cell::sync::Lazy::new(|| Arc::new(WebSocketManager::new()));