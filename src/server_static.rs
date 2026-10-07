use axum::Router as AxumRouter;
use tower_http::services::ServeDir;

pub struct StaticFiles {
    pub prefix: String,
    pub directory: String,
}

impl StaticFiles {
    pub fn new(prefix: &str, directory: &str) -> Self {
        Self {
            prefix: prefix.to_string(),
            directory: directory.to_string(),
        }
    }

    pub fn attach(self, router: AxumRouter) -> AxumRouter {
        router.nest_service(&self.prefix, ServeDir::new(&self.directory))
    }
}