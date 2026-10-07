use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::routing::get;
use axum::Router as AxumRouter;
use std::sync::Arc;

pub type WsHandlerFn = Arc<
    dyn Fn(WsConnection) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>
        + Send
        + Sync,
>;

pub struct WsConnection {
    pub id: String,
    pub path: String,
    inner: Arc<tokio::sync::Mutex<WebSocket>>,
}

impl WsConnection {
    pub fn new(id: String, path: String, ws: WebSocket) -> Self {
        Self {
            id,
            path,
            inner: Arc::new(tokio::sync::Mutex::new(ws)),
        }
    }

    pub async fn send_text(&self, text: &str) -> Result<(), String> {
        let mut ws = self.inner.lock().await;
        ws.send(Message::Text(text.to_string()))
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn send_binary(&self, data: Vec<u8>) -> Result<(), String> {
        let mut ws = self.inner.lock().await;
        ws.send(Message::Binary(data))
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn recv(&self) -> Option<WsMessage> {
        let mut ws = self.inner.lock().await;
        match ws.recv().await {
            Some(Ok(msg)) => Some(WsMessage::from(msg)),
            _ => None,
        }
    }

    pub async fn close(&self) -> Result<(), String> {
        let mut ws = self.inner.lock().await;

        // axum's WebSocket::close() takes self by value.
        // We can't move out of a MutexGuard, so we use a workaround:
        // send a close frame instead, then let the socket drop naturally.
        ws.send(Message::Close(None))
            .await
            .map_err(|e| e.to_string())
    }
}

#[derive(Debug, Clone)]
pub enum WsMessage {
    Text(String),
    Binary(Vec<u8>),
    Ping(Vec<u8>),
    Pong(Vec<u8>),
    Close,
}

impl From<Message> for WsMessage {
    fn from(msg: Message) -> Self {
        match msg {
            Message::Text(s) => WsMessage::Text(s),
            Message::Binary(b) => WsMessage::Binary(b),
            Message::Ping(p) => WsMessage::Ping(p),
            Message::Pong(p) => WsMessage::Pong(p),
            Message::Close(_) => WsMessage::Close,
        }
    }
}

pub struct WebSocketRoute {
    pub path: String,
    pub handler: WsHandlerFn,
}

impl WebSocketRoute {
    pub fn new<F, Fut>(path: &str, handler: F) -> Self
    where
        F: Fn(WsConnection) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        Self {
            path: path.to_string(),
            handler: Arc::new(move |conn| Box::pin(handler(conn))),
        }
    }

    pub fn attach(self, router: AxumRouter) -> AxumRouter {
        let handler = self.handler.clone();
        let path = self.path.clone();

        let path_for_closure = path.clone();

        router.route(
            &path,
            get(move |ws: WebSocketUpgrade| {
                let handler = handler.clone();
                let path = path_for_closure.clone();
                async move {
                    ws.on_upgrade(move |socket| async move {
                        let id = uuid_v4();
                        let conn = WsConnection::new(id, path, socket);
                        handler(conn).await;
                    })
                }
            }),
        )
    }
}

fn uuid_v4() -> String {
    use rand::Rng;
    let bytes: [u8; 16] = rand::thread_rng().gen();
    hex::encode(bytes)
}