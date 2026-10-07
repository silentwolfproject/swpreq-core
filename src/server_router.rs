use crate::error::Result;
use axum::{
    body::Body,
    http::{Request, Response, StatusCode},
    routing::any,
    Router as AxumRouter,
};
use std::collections::HashMap;
use std::sync::Arc;

pub type HandlerFn = Arc<
    dyn Fn(ServerRequest) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<ServerResponse>> + Send>>
        + Send
        + Sync,
>;

pub struct ServerRequest {
    pub method: String,
    pub path: String,
    pub query: HashMap<String, String>,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
    pub params: HashMap<String, String>,
}

pub struct ServerResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl ServerResponse {
    pub fn json(value: serde_json::Value) -> Self {
        let body = serde_json::to_vec(&value).unwrap_or_default();
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());

        Self {
            status: 200,
            headers,
            body,
        }
    }

    pub fn text(value: &str) -> Self {
        let mut headers = HashMap::new();
        headers.insert(
            "Content-Type".to_string(),
            "text/plain; charset=utf-8".to_string(),
        );

        Self {
            status: 200,
            headers,
            body: value.as_bytes().to_vec(),
        }
    }

    pub fn html(value: &str) -> Self {
        let mut headers = HashMap::new();
        headers.insert(
            "Content-Type".to_string(),
            "text/html; charset=utf-8".to_string(),
        );

        Self {
            status: 200,
            headers,
            body: value.as_bytes().to_vec(),
        }
    }

    pub fn status(mut self, code: u16) -> Self {
        self.status = code;
        self
    }
}

pub struct RouteEntry {
    pub method: String,
    pub path: String,
    pub handler: HandlerFn,
}

pub struct Router {
    routes: Vec<RouteEntry>,
}

impl Router {
    pub fn new() -> Self {
        Self { routes: Vec::new() }
    }

    pub fn add<F, Fut>(&mut self, method: &str, path: &str, handler: F)
    where
        F: Fn(ServerRequest) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<ServerResponse>> + Send + 'static,
    {
        let handler_arc: HandlerFn = Arc::new(move |req| Box::pin(handler(req)));

        self.routes.push(RouteEntry {
            method: method.to_uppercase(),
            path: path.to_string(),
            handler: handler_arc,
        });
    }

    pub fn get<F, Fut>(&mut self, path: &str, handler: F)
    where
        F: Fn(ServerRequest) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<ServerResponse>> + Send + 'static,
    {
        self.add("GET", path, handler);
    }

    pub fn post<F, Fut>(&mut self, path: &str, handler: F)
    where
        F: Fn(ServerRequest) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<ServerResponse>> + Send + 'static,
    {
        self.add("POST", path, handler);
    }

    pub fn put<F, Fut>(&mut self, path: &str, handler: F)
    where
        F: Fn(ServerRequest) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<ServerResponse>> + Send + 'static,
    {
        self.add("PUT", path, handler);
    }

    pub fn delete<F, Fut>(&mut self, path: &str, handler: F)
    where
        F: Fn(ServerRequest) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<ServerResponse>> + Send + 'static,
    {
        self.add("DELETE", path, handler);
    }

    pub fn build(&self) -> AxumRouter {
        let mut router = AxumRouter::new();

        for route in &self.routes {
            let handler = route.handler.clone();
            let path = route.path.clone();

            router = router.route(
                &path,
                any(move |req: Request<Body>| {
                    let handler = handler.clone();
                    async move {
                        let (parts, body) = req.into_parts();
                        let body_bytes = axum::body::to_bytes(body, 1024 * 1024)
                            .await
                            .unwrap_or_default()
                            .to_vec();

                        let query = parts
                            .uri
                            .query()
                            .map(|q| {
                                url::form_urlencoded::parse(q.as_bytes())
                                    .into_owned()
                                    .collect::<HashMap<String, String>>()
                            })
                            .unwrap_or_default();

                        let headers: HashMap<String, String> = parts
                            .headers
                            .iter()
                            .filter_map(|(k, v)| {
                                v.to_str()
                                    .ok()
                                    .map(|s| (k.as_str().to_string(), s.to_string()))
                            })
                            .collect();

                        let server_req = ServerRequest {
                            method: parts.method.to_string(),
                            path: parts.uri.path().to_string(),
                            query,
                            headers,
                            body: body_bytes,
                            params: HashMap::new(),
                        };

                        match handler(server_req).await {
                            Ok(res) => {
                                let mut response = Response::builder().status(
                                    StatusCode::from_u16(res.status).unwrap_or(StatusCode::OK),
                                );

                                for (k, v) in res.headers {
                                    response = response.header(k, v);
                                }

                                response.body(Body::from(res.body)).unwrap()
                            }
                            Err(e) => {
                                let body = format!("{{\"error\": \"{}\"}}", e);
                                Response::builder()
                                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                                    .header("Content-Type", "application/json")
                                    .body(Body::from(body))
                                    .unwrap()
                            }
                        }
                    }
                }),
            );
        }

        router
    }
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}