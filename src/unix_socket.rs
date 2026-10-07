use crate::error::{Result, SwpreqError};

#[cfg(unix)]
pub async fn connect_unix(path: &str) -> Result<tokio::net::UnixStream> {
    tokio::net::UnixStream::connect(path)
        .await
        .map_err(|e| SwpreqError::Network(format!("unix socket connect: {}", e)))
}

#[cfg(not(unix))]
pub async fn connect_unix(_path: &str) -> Result<()> {
    Err(SwpreqError::Network(
        "unix sockets are only available on Unix-like systems".to_string(),
    ))
}

#[cfg(unix)]
pub async fn request_unix(
    socket_path: &str,
    method: &str,
    path: &str,
    _headers: Vec<(String, String)>,
    body: Vec<u8>,
) -> Result<Vec<u8>> {
    use http_body_util::{BodyExt, Full};
    use hyper::body::Bytes;
    use hyper_util::rt::TokioIo;

    let stream = connect_unix(socket_path).await?;
    let io = TokioIo::new(stream);

    let (mut sender, conn) = hyper::client::conn::http1::handshake(io)
        .await
        .map_err(|e| SwpreqError::Network(e.to_string()))?;

    tokio::spawn(async move {
        let _ = conn.await;
    });

    let req = hyper::Request::builder()
        .method(method)
        .uri(path)
        .body(Full::new(Bytes::from(body)))
        .map_err(|e| SwpreqError::Internal(e.to_string()))?;

    let response = sender
        .send_request(req)
        .await
        .map_err(|e| SwpreqError::Network(e.to_string()))?;

    let body_bytes = response
        .into_body()
        .collect()
        .await
        .map_err(|e| SwpreqError::Network(e.to_string()))?
        .to_bytes()
        .to_vec();

    Ok(body_bytes)
}

#[cfg(not(unix))]
pub async fn request_unix(
    _socket_path: &str,
    _method: &str,
    _path: &str,
    _headers: Vec<(String, String)>,
    _body: Vec<u8>,
) -> Result<Vec<u8>> {
    Err(SwpreqError::Network(
        "unix sockets are only available on Unix-like systems".to_string(),
    ))
}