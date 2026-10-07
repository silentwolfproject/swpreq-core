use crate::error::Result;
use crate::server_router::{ServerRequest, ServerResponse};
use std::future::Future;
use std::pin::Pin;

pub type MiddlewareNext = Box<
    dyn FnOnce(ServerRequest) -> Pin<Box<dyn Future<Output = Result<ServerResponse>> + Send>>
        + Send,
>;

pub trait Middleware {
    fn handle(
        &self,
        req: ServerRequest,
        next: MiddlewareNext,
    ) -> Pin<Box<dyn Future<Output = Result<ServerResponse>> + Send>>;
}

pub struct FnMiddleware<F> {
    f: F,
}

impl<F, Fut> FnMiddleware<F>
where
    F: Fn(ServerRequest, MiddlewareNext) -> Fut + Send + Sync,
    Fut: Future<Output = Result<ServerResponse>> + Send + 'static,
{
    pub fn new(f: F) -> Self {
        Self { f }
    }
}

impl<F, Fut> Middleware for FnMiddleware<F>
where
    F: Fn(ServerRequest, MiddlewareNext) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<ServerResponse>> + Send + 'static,
{
    fn handle(
        &self,
        req: ServerRequest,
        next: MiddlewareNext,
    ) -> Pin<Box<dyn Future<Output = Result<ServerResponse>> + Send>> {
        Box::pin((self.f)(req, next))
    }
}