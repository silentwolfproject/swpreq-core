use crate::error::Result;
use crate::request::RequestBuilder;
use crate::response::Response;
use std::sync::Arc;

pub type RequestHook = Arc<dyn Fn(RequestBuilder) -> Result<RequestBuilder> + Send + Sync>;
pub type ResponseHook = Arc<dyn Fn(Response) -> Result<Response> + Send + Sync>;

#[derive(Clone, Default)]
pub struct Hooks {
    pub request_hooks: Vec<RequestHook>,
    pub response_hooks: Vec<ResponseHook>,
}

impl Hooks {
    pub fn new() -> Self {
        Self {
            request_hooks: Vec::new(),
            response_hooks: Vec::new(),
        }
    }

    pub fn register_request(&mut self, hook: RequestHook) {
        self.request_hooks.push(hook);
    }

    pub fn register_response(&mut self, hook: ResponseHook) {
        self.response_hooks.push(hook);
    }

    pub fn apply_request(&self, mut req: RequestBuilder) -> Result<RequestBuilder> {
        for hook in &self.request_hooks {
            req = hook(req)?;
        }
        Ok(req)
    }

    pub fn apply_response(&self, mut res: Response) -> Result<Response> {
        for hook in &self.response_hooks {
            res = hook(res)?;
        }
        Ok(res)
    }

    pub fn clear(&mut self) {
        self.request_hooks.clear();
        self.response_hooks.clear();
    }
}