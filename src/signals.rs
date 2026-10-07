use crate::error::{Result, SwpreqError};
use std::sync::Arc;
use tokio::sync::Notify;

pub struct SignalHandler {
    notify: Arc<Notify>,
}

impl SignalHandler {
    pub fn new() -> Self {
        Self {
            notify: Arc::new(Notify::new()),
        }
    }

    pub async fn wait(&self) {
        self.notify.notified().await;
    }

    pub fn trigger(&self) {
        self.notify.notify_waiters();
    }

    pub async fn install(self: Arc<Self>) -> Result<()> {
        #[cfg(unix)]
        {
            use tokio::signal::unix::{signal, SignalKind};

            let mut sigint = signal(SignalKind::interrupt())
                .map_err(|e| SwpreqError::Internal(e.to_string()))?;
            let mut sigterm = signal(SignalKind::terminate())
                .map_err(|e| SwpreqError::Internal(e.to_string()))?;

            let me = self.clone();
            tokio::spawn(async move {
                tokio::select! {
                    _ = sigint.recv() => me.trigger(),
                    _ = sigterm.recv() => me.trigger(),
                }
            });
        }

        #[cfg(windows)]
        {
            use tokio::signal::windows::ctrl_c;

            let mut ctrlc = ctrl_c().map_err(|e| SwpreqError::Internal(e.to_string()))?;
            let me = self.clone();
            tokio::spawn(async move {
                let _ = ctrlc.recv().await;
                me.trigger();
            });
        }

        Ok(())
    }
}

impl Default for SignalHandler {
    fn default() -> Self {
        Self::new()
    }
}