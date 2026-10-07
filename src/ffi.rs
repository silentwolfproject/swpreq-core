use crate::client::{Client, ClientConfig};
use crate::error::{from_c_string, to_c_string, Result, SwpreqError};
use crate::response::Response;
use crate::websocket::{EventCallback, MessageCallback, WS_MANAGER};
use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_void};
use std::sync::Arc;
use std::time::Duration;
use crate::auth_digest::DigestAuth;
use crate::connector::{SocketFamily, TCPConnector};
use crate::resolver::AsyncResolver;
use crate::server::Server;
use crate::stream_body::StreamBody;
use crate::trace::TraceConfig;

pub const SWPREQ_OK: c_int = 0;
pub const SWPREQ_ERR_INVALID_ARG: c_int = 1;
pub const SWPREQ_ERR_NETWORK: c_int = 2;
pub const SWPREQ_ERR_TIMEOUT: c_int = 3;
pub const SWPREQ_ERR_TLS: c_int = 4;
pub const SWPREQ_ERR_PROXY: c_int = 5;
pub const SWPREQ_ERR_DECODE: c_int = 6;
pub const SWPREQ_ERR_INTERNAL: c_int = 99;
pub const SWPREQ_ERR_NOT_FOUND: c_int = 10;

#[repr(C)]
pub struct SwpreqResponse {
    pub status_code: u16,
    pub _pad: u16,
    pub body: *mut u8,
    pub body_len: usize,
    pub headers_json: *mut c_char,
    pub url: *mut c_char,
    pub error: *mut c_char,
    pub elapsed_ms: u64,
    pub http_version: u32,
}

#[repr(C)]
pub struct SwpreqResponseCallback {
    pub user_data: *mut c_void,
    pub on_success: extern "C" fn(*mut c_void, *const SwpreqResponse),
    pub on_error: extern "C" fn(*mut c_void, c_int, *const c_char),
}



#[no_mangle]
pub extern "C" fn swpreq_digest_auth_new(
    username: *const c_char,
    password: *const c_char,
) -> *mut c_void {
    let username = from_c_string(username).unwrap_or_default();
    let password = from_c_string(password).unwrap_or_default();

    let auth = DigestAuth::new(&username, &password);
    Box::into_raw(Box::new(auth)) as *mut c_void
}

#[no_mangle]
pub extern "C" fn swpreq_digest_auth_from_challenge(
    username: *const c_char,
    password: *const c_char,
    challenge: *const c_char,
) -> *mut c_void {
    let username = from_c_string(username).unwrap_or_default();
    let password = from_c_string(password).unwrap_or_default();
    let challenge = from_c_string(challenge).unwrap_or_default();

    match DigestAuth::from_challenge(&username, &password, &challenge) {
        Ok(auth) => Box::into_raw(Box::new(auth)) as *mut c_void,
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn swpreq_digest_auth_header(
    auth: *mut c_void,
    method: *const c_char,
    uri: *const c_char,
) -> *mut c_char {
    if auth.is_null() {
        return to_c_string(String::new());
    }

    let auth = unsafe { &mut *(auth as *mut DigestAuth) };
    let method = from_c_string(method).unwrap_or_default();
    let uri = from_c_string(uri).unwrap_or_default();

    match auth.build_header(&method, &uri) {
        Ok(header) => to_c_string(header),
        Err(_) => to_c_string(String::new()),
    }
}

#[no_mangle]
pub extern "C" fn swpreq_digest_auth_free(auth: *mut c_void) {
    if !auth.is_null() {
        unsafe {
            let _ = Box::from_raw(auth as *mut DigestAuth);
        }
    }
}


#[no_mangle]
pub extern "C" fn swpreq_tcp_connector_new(config_json: *const c_char) -> *mut c_void {
    let config_str = from_c_string(config_json).unwrap_or_default();
    let config: serde_json::Value =
        serde_json::from_str(&config_str).unwrap_or(serde_json::json!({}));

    let mut connector = TCPConnector::new();

    if let Some(v) = config.get("limit").and_then(|v| v.as_u64()) {
        connector.limit = v as usize;
    }
    if let Some(v) = config.get("limit_per_host").and_then(|v| v.as_u64()) {
        connector.limit_per_host = v as usize;
    }
    if let Some(v) = config.get("ttl_dns_cache").and_then(|v| v.as_u64()) {
        connector.ttl_dns_cache = Duration::from_secs(v);
    }
    if let Some(v) = config.get("force_close").and_then(|v| v.as_bool()) {
        connector.force_close = v;
    }
    if let Some(v) = config.get("keepalive_timeout").and_then(|v| v.as_f64()) {
        connector.keepalive_timeout = Duration::from_secs_f64(v);
    }
    if let Some(v) = config.get("ssl_verify").and_then(|v| v.as_bool()) {
        connector.ssl_verify = v;
    }
    if let Some(v) = config.get("family").and_then(|v| v.as_str()) {
        connector.family = match v {
            "ipv4" => SocketFamily::IPv4,
            "ipv6" => SocketFamily::IPv6,
            _ => SocketFamily::Any,
        };
    }

    Box::into_raw(Box::new(connector)) as *mut c_void
}

#[no_mangle]
pub extern "C" fn swpreq_tcp_connector_free(connector: *mut c_void) {
    if !connector.is_null() {
        unsafe {
            let _ = Box::from_raw(connector as *mut TCPConnector);
        }
    }
}


#[no_mangle]
pub extern "C" fn swpreq_resolver_new(nameservers_json: *const c_char) -> *mut c_void {
    let json_str = from_c_string(nameservers_json).unwrap_or_default();

    let nameservers: Vec<String> = serde_json::from_str(&json_str).unwrap_or_else(|_| {
        vec!["8.8.8.8".to_string(), "1.1.1.1".to_string()]
    });

    let resolver = AsyncResolver::new(nameservers);
    Box::into_raw(Box::new(resolver)) as *mut c_void
}

#[no_mangle]
pub extern "C" fn swpreq_resolver_free(resolver: *mut c_void) {
    if !resolver.is_null() {
        unsafe {
            let _ = Box::from_raw(resolver as *mut AsyncResolver);
        }
    }
}


#[no_mangle]
pub extern "C" fn swpreq_stream_body_from_file(path: *const c_char) -> *mut c_void {
    let path = from_c_string(path).unwrap_or_default();

    match StreamBody::from_file(&path) {
        Ok(body) => Box::into_raw(Box::new(body)) as *mut c_void,
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn swpreq_stream_body_from_bytes(
    data: *const u8,
    data_len: usize,
) -> *mut c_void {
    if data.is_null() || data_len == 0 {
        return std::ptr::null_mut();
    }

    let bytes = unsafe { std::slice::from_raw_parts(data, data_len).to_vec() };
    let body = StreamBody::from_bytes(bytes);

    Box::into_raw(Box::new(body)) as *mut c_void
}

#[no_mangle]
pub extern "C" fn swpreq_stream_body_free(body: *mut c_void) {
    if !body.is_null() {
        unsafe {
            let _ = Box::from_raw(body as *mut StreamBody);
        }
    }
}


#[no_mangle]
pub extern "C" fn swpreq_trace_config_new() -> *mut c_void {
    Box::into_raw(Box::new(TraceConfig::new())) as *mut c_void
}

#[no_mangle]
pub extern "C" fn swpreq_trace_config_free(config: *mut c_void) {
    if !config.is_null() {
        unsafe {
            let _ = Box::from_raw(config as *mut TraceConfig);
        }
    }
}


#[no_mangle]
pub extern "C" fn swpreq_server_new(config_json: *const c_char) -> *mut c_void {
    let config_str = from_c_string(config_json).unwrap_or_default();
    let config: serde_json::Value =
        serde_json::from_str(&config_str).unwrap_or(serde_json::json!({}));

    let host = config
        .get("host")
        .and_then(|v| v.as_str())
        .unwrap_or("0.0.0.0");
    let port = config.get("port").and_then(|v| v.as_u64()).unwrap_or(8080) as u16;

    let server = Server::new(host, port);
    Box::into_raw(Box::new(server)) as *mut c_void
}

#[no_mangle]
pub extern "C" fn swpreq_server_free(server: *mut c_void) {
    if !server.is_null() {
        unsafe {
            let _ = Box::from_raw(server as *mut Server);
        }
    }
}

#[no_mangle]
pub extern "C" fn swpreq_server_run(server: *mut c_void, error_out: *mut *mut c_char) -> c_int {
    if server.is_null() {
        return SWPREQ_ERR_INVALID_ARG;
    }

    let server = unsafe { Box::from_raw(server as *mut Server) };
    let rt = crate::runtime();

    match rt.block_on(server.run()) {
        Ok(_) => SWPREQ_OK,
        Err(e) => {
            if !error_out.is_null() {
                unsafe {
                    *error_out = to_c_string(e.to_string());
                }
            }
            e.code()
        }
    }
}


#[no_mangle]
pub extern "C" fn swpreq_signal_handler_new() -> *mut c_void {
    let handler = crate::signals::SignalHandler::new();
    Box::into_raw(Box::new(handler)) as *mut c_void
}

#[no_mangle]
pub extern "C" fn swpreq_signal_handler_free(handler: *mut c_void) {
    if !handler.is_null() {
        unsafe {
            let _ = Box::from_raw(handler as *mut crate::signals::SignalHandler);
        }
    }
}

#[no_mangle]
pub extern "C" fn swpreq_ws_connect(
    url: *const c_char,
    headers_json: *const c_char,
    handle_out: *mut c_int,
    error_out: *mut *mut c_char,
) -> c_int {
    if url.is_null() || handle_out.is_null() {
        return SWPREQ_ERR_INVALID_ARG;
    }

    let url_str = match from_c_string(url) {
        Some(s) => s,
        None => return SWPREQ_ERR_INVALID_ARG,
    };

    let headers = if let Some(hj) = from_c_string(headers_json) {
        if !hj.is_empty() {
            parse_headers_json(&hj).unwrap_or_default()
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    let rt = crate::runtime();
    let result = rt.block_on(async { WS_MANAGER.connect(&url_str, headers).await });

    match result {
        Ok(id) => {
            unsafe {
                *handle_out = id as c_int;
            }
            SWPREQ_OK
        }
        Err(e) => {
            if !error_out.is_null() {
                unsafe {
                    *error_out = to_c_string(e.to_string());
                }
            }
            e.code()
        }
    }
}

#[no_mangle]
pub extern "C" fn swpreq_ws_set_callbacks(
    handle: c_int,
    on_open: Option<EventCallback>,
    on_open_user_data: *mut c_void,
    on_message: Option<MessageCallback>,
    on_message_user_data: *mut c_void,
    on_error: Option<MessageCallback>,
    on_error_user_data: *mut c_void,
    on_close: Option<EventCallback>,
    on_close_user_data: *mut c_void,
) -> c_int {
    if handle <= 0 {
        return SWPREQ_ERR_INVALID_ARG;
    }

    let rt = crate::runtime();
    let result = rt.block_on(async {
        WS_MANAGER
            .set_callbacks(
                handle as usize,
                on_open.map(|cb| (cb, on_open_user_data as usize)),
                on_message.map(|cb| (cb, on_message_user_data as usize)),
                on_error.map(|cb| (cb, on_error_user_data as usize)),
                on_close.map(|cb| (cb, on_close_user_data as usize)),
                on_message_user_data as usize,
            )
            .await
    });

    match result {
        Ok(_) => SWPREQ_OK,
        Err(_) => SWPREQ_ERR_NOT_FOUND,
    }
}

#[no_mangle]
pub extern "C" fn swpreq_ws_send_text(
    handle: c_int,
    text: *const c_char,
) -> c_int {
    if handle <= 0 {
        return SWPREQ_ERR_INVALID_ARG;
    }
    let text_str = match from_c_string(text) {
        Some(s) => s,
        None => return SWPREQ_ERR_INVALID_ARG,
    };

    let rt = crate::runtime();
    match rt.block_on(async { WS_MANAGER.send_text(handle as usize, &text_str).await }) {
        Ok(_) => SWPREQ_OK,
        Err(e) => e.code(),
    }
}

#[no_mangle]
pub extern "C" fn swpreq_ws_send_binary(
    handle: c_int,
    data: *const u8,
    data_len: usize,
) -> c_int {
    if handle <= 0 {
        return SWPREQ_ERR_INVALID_ARG;
    }
    if data.is_null() || data_len == 0 {
        return SWPREQ_ERR_INVALID_ARG;
    }

    let bytes = unsafe { std::slice::from_raw_parts(data, data_len).to_vec() };

    let rt = crate::runtime();
    match rt.block_on(async { WS_MANAGER.send_binary(handle as usize, bytes).await }) {
        Ok(_) => SWPREQ_OK,
        Err(e) => e.code(),
    }
}

#[no_mangle]
pub extern "C" fn swpreq_ws_send_ping(handle: c_int) -> c_int {
    if handle <= 0 {
        return SWPREQ_ERR_INVALID_ARG;
    }

    let rt = crate::runtime();
    match rt.block_on(async { WS_MANAGER.send_ping(handle as usize).await }) {
        Ok(_) => SWPREQ_OK,
        Err(e) => e.code(),
    }
}

#[no_mangle]
pub extern "C" fn swpreq_ws_close(handle: c_int) -> c_int {
    if handle <= 0 {
        return SWPREQ_ERR_INVALID_ARG;
    }

    let rt = crate::runtime();
    let _ = rt.block_on(async { WS_MANAGER.close(handle as usize).await });
    let _ = rt.block_on(async { WS_MANAGER.remove(handle as usize).await });
    SWPREQ_OK
}

#[no_mangle]
pub extern "C" fn swpreq_ws_free(handle: c_int) {
    if handle <= 0 {
        return;
    }
    let rt = crate::runtime();
    let _ = rt.block_on(async {
        let _ = WS_MANAGER.close(handle as usize).await;
        WS_MANAGER.remove(handle as usize).await;
    });
}

#[no_mangle]
pub extern "C" fn swpreq_version() -> *mut c_char {
    to_c_string(env!("CARGO_PKG_VERSION").to_string())
}

#[no_mangle]
pub extern "C" fn swpreq_string_free(s: *mut c_char) {
    if !s.is_null() {
        unsafe {
            let _ = CString::from_raw(s);
        }
    }
}

#[no_mangle]
pub extern "C" fn swpreq_response_free(response: *mut SwpreqResponse) {
    if response.is_null() {
        return;
    }
    unsafe {
        let r = Box::from_raw(response);
        if !r.body.is_null() && r.body_len > 0 {
            let slice = std::ptr::slice_from_raw_parts_mut(r.body, r.body_len);
            let _ = Box::from_raw(slice);
        }
        if !r.headers_json.is_null() {
            let _ = CString::from_raw(r.headers_json);
        }
        if !r.url.is_null() {
            let _ = CString::from_raw(r.url);
        }
        if !r.error.is_null() {
            let _ = CString::from_raw(r.error);
        }
    }
}

#[no_mangle]
pub extern "C" fn swpreq_client_new(config_json: *const c_char) -> *mut c_void {
    let config_str = from_c_string(config_json).unwrap_or_default();
    let config = parse_config(&config_str).unwrap_or_default();
    match Client::new(config) {
        Ok(client) => Box::into_raw(Box::new(client)) as *mut c_void,
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn swpreq_client_free(client: *mut c_void) {
    if client.is_null() {
        return;
    }
    unsafe {
        let _ = Box::from_raw(client as *mut Client);
    }
}

#[no_mangle]
pub extern "C" fn swpreq_set_default_header(
    client: *mut c_void,
    key: *const c_char,
    value: *const c_char,
) -> c_int {
    if client.is_null() {
        return SWPREQ_ERR_INVALID_ARG;
    }
    let key = match from_c_string(key) {
        Some(s) => s,
        None => return SWPREQ_ERR_INVALID_ARG,
    };
    let value = match from_c_string(value) {
        Some(s) => s,
        None => return SWPREQ_ERR_INVALID_ARG,
    };
    let client_ref = unsafe { &*(client as *const Client) };
    match client_ref.set_default_header(&key, &value) {
        Ok(_) => SWPREQ_OK,
        Err(_) => SWPREQ_ERR_INVALID_ARG,
    }
}

#[no_mangle]
pub extern "C" fn swpreq_remove_default_header(
    client: *mut c_void,
    key: *const c_char,
) -> c_int {
    if client.is_null() {
        return SWPREQ_ERR_INVALID_ARG;
    }
    let key = match from_c_string(key) {
        Some(s) => s,
        None => return SWPREQ_ERR_INVALID_ARG,
    };
    let client_ref = unsafe { &*(client as *const Client) };
    match client_ref.remove_default_header(&key) {
        Ok(_) => SWPREQ_OK,
        Err(_) => SWPREQ_ERR_INVALID_ARG,
    }
}

#[no_mangle]
pub extern "C" fn swpreq_clear_default_headers(client: *mut c_void) -> c_int {
    if client.is_null() {
        return SWPREQ_ERR_INVALID_ARG;
    }
    let client_ref = unsafe { &*(client as *const Client) };
    client_ref.clear_default_headers();
    SWPREQ_OK
}

#[no_mangle]
pub extern "C" fn swpreq_set_cookie(
    client: *mut c_void,
    name: *const c_char,
    value: *const c_char,
    domain: *const c_char,
) -> c_int {
    if client.is_null() {
        return SWPREQ_ERR_INVALID_ARG;
    }
    let name = match from_c_string(name) {
        Some(s) => s,
        None => return SWPREQ_ERR_INVALID_ARG,
    };
    let value = match from_c_string(value) {
        Some(s) => s,
        None => return SWPREQ_ERR_INVALID_ARG,
    };
    let domain = match from_c_string(domain) {
        Some(s) => s,
        None => return SWPREQ_ERR_INVALID_ARG,
    };
    let client_ref = unsafe { &*(client as *const Client) };
    client_ref.set_cookie(&name, &value, &domain);
    SWPREQ_OK
}

#[no_mangle]
pub extern "C" fn swpreq_clear_cookies(client: *mut c_void) -> c_int {
    if client.is_null() {
        return SWPREQ_ERR_INVALID_ARG;
    }
    let client_ref = unsafe { &*(client as *const Client) };
    client_ref.clear_cookies();
    SWPREQ_OK
}

#[no_mangle]
pub extern "C" fn swpreq_request_sync(
    client: *mut c_void,
    method: *const c_char,
    url: *const c_char,
    headers_json: *const c_char,
    body: *const u8,
    body_len: usize,
    timeout_ms: u64,
    proxy: *const c_char,
    redirect_limit: i64,
    follow_redirects: c_int,
) -> *mut SwpreqResponse {
    if client.is_null() {
        return error_response(SWPREQ_ERR_INVALID_ARG, "client is null");
    }
    let method = from_c_string(method).unwrap_or_else(|| "GET".to_string());
    let url = match from_c_string(url) {
        Some(s) => s,
        None => return error_response(SWPREQ_ERR_INVALID_ARG, "url is null"),
    };

    let client_ref = unsafe { &*(client as *const Client) };
    let mut builder = client_ref.request(&method, &url);

    if let Some(hj) = from_c_string(headers_json) {
        if !hj.is_empty() {
            if let Ok(headers) = parse_headers_json(&hj) {
                if !headers.is_empty() {
                    if let Err(e) = builder.headers(headers) {
                        return error_response(e.code(), &e.to_string());
                    }
                }
            }
        }
    }

    if !body.is_null() && body_len > 0 {
        let body_vec = unsafe { std::slice::from_raw_parts(body, body_len).to_vec() };
        builder.body(body_vec);
    }

    if timeout_ms > 0 {
        builder.timeout(Duration::from_millis(timeout_ms));
    }

    if let Some(p) = from_c_string(proxy) {
        if !p.is_empty() {
            builder.proxy(&p);
        }
    }

    if follow_redirects == 0 {
        builder.follow_redirects(false);
    } else if follow_redirects == 1 {
        builder.follow_redirects(true);
    }

    if redirect_limit >= 0 {
        builder.max_redirects(redirect_limit as usize);
    }

    match builder.send_sync() {
        Ok(response) => response_to_ffi(response),
        Err(e) => error_response(e.code(), &e.to_string()),
    }
}

#[no_mangle]
pub extern "C" fn swpreq_request_async(
    client: *mut c_void,
    method: *const c_char,
    url: *const c_char,
    headers_json: *const c_char,
    body: *const u8,
    body_len: usize,
    timeout_ms: u64,
    proxy: *const c_char,
    redirect_limit: i64,
    follow_redirects: c_int,
    callback: *mut SwpreqResponseCallback,
) -> c_int {
    if client.is_null() || callback.is_null() {
        return SWPREQ_ERR_INVALID_ARG;
    }

    let client_owned = unsafe { &*(client as *const Client) }.clone();
    let method = from_c_string(method).unwrap_or_else(|| "GET".to_string());
    let url = match from_c_string(url) {
        Some(s) => s,
        None => return SWPREQ_ERR_INVALID_ARG,
    };
    let headers = from_c_string(headers_json);
    let proxy = from_c_string(proxy);
    let body_vec = if !body.is_null() && body_len > 0 {
        Some(unsafe { std::slice::from_raw_parts(body, body_len).to_vec() })
    } else {
        None
    };
    let cb_addr = callback as usize;

    std::thread::spawn(move || {
        let client_ref = &client_owned;
        let mut builder = client_ref.request(&method, &url);

        if let Some(hj) = headers {
            if !hj.is_empty() {
                if let Ok(h) = parse_headers_json(&hj) {
                    if !h.is_empty() {
                        let _ = builder.headers(h);
                    }
                }
            }
        }

        if let Some(bv) = body_vec {
            builder.body(bv);
        }

        if timeout_ms > 0 {
            builder.timeout(Duration::from_millis(timeout_ms));
        }

        if let Some(p) = proxy {
            if !p.is_empty() {
                builder.proxy(&p);
            }
        }

        if follow_redirects == 0 {
            builder.follow_redirects(false);
        } else if follow_redirects == 1 {
            builder.follow_redirects(true);
        }

        if redirect_limit >= 0 {
            builder.max_redirects(redirect_limit as usize);
        }

        let result = builder.send_sync();
        let cb = unsafe { &*(cb_addr as *const SwpreqResponseCallback) };

        match result {
            Ok(resp) => {
                let ffi = response_to_ffi(resp);
                (cb.on_success)(cb.user_data, ffi);
                swpreq_response_free(ffi);
            }
            Err(e) => {
                let msg = to_c_string(e.to_string());
                (cb.on_error)(cb.user_data, e.code(), msg);
                swpreq_string_free(msg);
            }
        }
    });

    SWPREQ_OK
}

fn parse_config(json: &str) -> Option<ClientConfig> {
    if json.is_empty() {
        return Some(ClientConfig::default());
    }
    let value: serde_json::Value = serde_json::from_str(json).ok()?;
    let mut config = ClientConfig::default();

    if let Some(t) = value.get("timeout_ms").and_then(|v| v.as_u64()) {
        config.timeout = Duration::from_millis(t);
    }
    if let Some(t) = value.get("connect_timeout_ms").and_then(|v| v.as_u64()) {
        config.connect_timeout = Duration::from_millis(t);
    }
    if let Some(p) = value.get("pool_size").and_then(|v| v.as_u64()) {
        config.pool_size = p as usize;
    }
    if let Some(h2) = value.get("http2").and_then(|v| v.as_bool()) {
        config.http2 = h2;
    }
    if let Some(rl) = value.get("redirect_limit").and_then(|v| v.as_u64()) {
        config.redirect_limit = rl as usize;
    }
    if let Some(p) = value.get("proxy").and_then(|v| v.as_str()) {
        config.proxy = Some(p.to_string());
    }
    if let Some(ua) = value.get("user_agent").and_then(|v| v.as_str()) {
        config.user_agent = ua.to_string();
    }
    if let Some(d) = value
        .get("danger_accept_invalid_certs")
        .and_then(|v| v.as_bool())
    {
        config.danger_accept_invalid_certs = d;
    }
    Some(config)
}

fn parse_headers_json(json: &str) -> Result<Vec<(String, String)>> {
    let value: serde_json::Value =
        serde_json::from_str(json).map_err(|e| SwpreqError::Decode(e.to_string()))?;
    let mut headers = Vec::new();
    if let Some(obj) = value.as_object() {
        for (k, v) in obj {
            if let Some(s) = v.as_str() {
                headers.push((k.clone(), s.to_string()));
            } else {
                headers.push((k.clone(), v.to_string()));
            }
        }
    }
    Ok(headers)
}

fn response_to_ffi(response: Response) -> *mut SwpreqResponse {
    let body_len = response.body.len();
    let body_ptr = if body_len > 0 {
        let boxed = response.body.into_boxed_slice();
        Box::into_raw(boxed) as *mut u8
    } else {
        std::ptr::null_mut()
    };

    let headers_json = serde_json::to_string(&response.headers).unwrap_or_default();
    let headers_ptr = to_c_string(headers_json);
    let url_ptr = to_c_string(response.url);

    let ffi_resp = SwpreqResponse {
        status_code: response.status_code,
        _pad: 0,
        body: body_ptr,
        body_len,
        headers_json: headers_ptr,
        url: url_ptr,
        error: std::ptr::null_mut(),
        elapsed_ms: response.elapsed.as_millis() as u64,
        http_version: version_to_num(&response.http_version),
    };

    Box::into_raw(Box::new(ffi_resp))
}

fn version_to_num(v: &str) -> u32 {
    match v {
        "http/0.9" => 9,
        "http/1.0" => 10,
        "http/1.1" => 11,
        "http/2.0" => 20,
        "http/3.0" => 30,
        _ => 11,
    }
}

fn error_response(code: i32, message: &str) -> *mut SwpreqResponse {
    let ffi_resp = SwpreqResponse {
        status_code: 0,
        _pad: 0,
        body: std::ptr::null_mut(),
        body_len: 0,
        headers_json: std::ptr::null_mut(),
        url: std::ptr::null_mut(),
        error: to_c_string(format!("{}: {}", code, message)),
        elapsed_ms: 0,
        http_version: 0,
    };
    Box::into_raw(Box::new(ffi_resp))
}
#[no_mangle]
pub extern "C" fn swpreq_client_set_retry(
    client: *mut c_void,
    max_retries: u32,
    base_delay_ms: u64,
    exponential: c_int,
    jitter: c_int,
) -> c_int {
    if client.is_null() {
        return SWPREQ_ERR_INVALID_ARG;
    }

    let client_mut = unsafe { &mut *(client as *mut Client) };
    let policy = crate::retry::RetryPolicy {
        max_retries,
        base_delay: Duration::from_millis(base_delay_ms),
        max_delay: Duration::from_secs(30),
        retry_on_status: vec![408, 429, 500, 502, 503, 504],
        retry_on_error: true,
        exponential: exponential != 0,
        jitter: jitter != 0,
    };
    client_mut.set_retry_policy(policy);
    SWPREQ_OK
}

#[no_mangle]
pub extern "C" fn swpreq_client_register_adapter(
    client: *mut c_void,
    prefix: *const c_char,
    name: *const c_char,
) -> c_int {
    if client.is_null() {
        return SWPREQ_ERR_INVALID_ARG;
    }
    let prefix_str = match from_c_string(prefix) {
        Some(s) => s,
        None => return SWPREQ_ERR_INVALID_ARG,
    };
    let name_str = from_c_string(name).unwrap_or_else(|| "adapter".to_string());

    let client_ref = unsafe { &*(client as *const Client) };
    let adapter = Arc::new(crate::adapter::DefaultAdapter::new(
        &name_str,
        &prefix_str,
        Arc::new(client_ref.clone()),
    ));
    client_ref.mount(&prefix_str, adapter);
    SWPREQ_OK
}

#[no_mangle]
pub extern "C" fn swpreq_netrc_lookup(
    client: *mut c_void,
    host: *const c_char,
    login_out: *mut *mut c_char,
    password_out: *mut *mut c_char,
) -> c_int {
    if client.is_null() || login_out.is_null() || password_out.is_null() {
        return SWPREQ_ERR_INVALID_ARG;
    }

    let host_str = match from_c_string(host) {
        Some(s) => s,
        None => return SWPREQ_ERR_INVALID_ARG,
    };

    let client_ref = unsafe { &*(client as *const Client) };
    match client_ref.netrc_lookup(&host_str) {
        Some((login, password)) => {
            unsafe {
                *login_out = to_c_string(login);
                *password_out = to_c_string(password);
            }
            SWPREQ_OK
        }
        None => SWPREQ_ERR_NOT_FOUND,
    }
}

#[no_mangle]
pub extern "C" fn swpreq_oauth1_header(
    method: *const c_char,
    url: *const c_char,
    consumer_key: *const c_char,
    consumer_secret: *const c_char,
    token: *const c_char,
    token_secret: *const c_char,
) -> *mut c_char {
    let method_str = from_c_string(method).unwrap_or_else(|| "GET".to_string());
    let url_str = match from_c_string(url) {
        Some(s) => s,
        None => return to_c_string(String::new()),
    };

    let params = crate::oauth::OAuth1Params {
        consumer_key: from_c_string(consumer_key).unwrap_or_default(),
        consumer_secret: from_c_string(consumer_secret).unwrap_or_default(),
        token: from_c_string(token),
        token_secret: from_c_string(token_secret),
        ..Default::default()
    };

    match crate::oauth::OAuth1::generate_header(&params, &method_str, &url_str, None) {
        Ok(header) => to_c_string(header),
        Err(_) => to_c_string(String::new()),
    }
}