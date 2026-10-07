use crate::error::{Result, SwpreqError};
use crate::server_router::Router;
use crate::server_static::StaticFiles;
use std::sync::Arc;

pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub workers: usize,
    pub keep_alive: bool,
    pub max_body_size: usize,
    pub request_timeout_secs: u64,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8080,
            workers: 4,
            keep_alive: true,
            max_body_size: 16 * 1024 * 1024,
            request_timeout_secs: 30,
        }
    }
}

pub struct Server {
    pub config: ServerConfig,
    pub router: Arc<Router>,
    pub static_files: Vec<StaticFiles>,
}

impl Server {
    pub fn new(host: &str, port: u16) -> Self {
        Self {
            config: ServerConfig {
                host: host.to_string(),
                port,
                ..Default::default()
            },
            router: Arc::new(Router::new()),
            static_files: Vec::new(),
        }
    }

    pub fn with_config(config: ServerConfig) -> Self {
        Self {
            config,
            router: Arc::new(Router::new()),
            static_files: Vec::new(),
        }
    }

    pub fn router(&self) -> Arc<Router> {
        self.router.clone()
    }

    pub fn mount_static(&mut self, prefix: &str, directory: &str) {
        self.static_files.push(StaticFiles::new(prefix, directory));
    }

    pub async fn run(self) -> Result<()> {
        use axum::Router as AxumRouter;
        use tokio::net::TcpListener;

        let mut app: AxumRouter = self.router.build();

        for sf in self.static_files {
            app = sf.attach(app);
        }

        let addr = format!("{}:{}", self.config.host, self.config.port);
        let listener = TcpListener::bind(&addr)
            .await
            .map_err(|e| SwpreqError::Network(e.to_string()))?;

        tracing::info!("swpreq server listening on {}", addr);

        axum::serve(listener, app)
            .await
            .map_err(|e| SwpreqError::Internal(e.to_string()))?;

        Ok(())
    }
}