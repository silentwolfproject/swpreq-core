use crate::error::{Result, SwpreqError};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub max_retries: u32,
    pub base_delay: Duration,
    pub max_delay: Duration,
    pub retry_on_status: Vec<u16>,
    pub retry_on_error: bool,
    pub exponential: bool,
    pub jitter: bool,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
            retry_on_status: vec![408, 429, 500, 502, 503, 504],
            retry_on_error: true,
            exponential: true,
            jitter: true,
        }
    }
}

impl RetryPolicy {
    pub fn none() -> Self {
        Self {
            max_retries: 0,
            ..Default::default()
        }
    }

    pub fn aggressive() -> Self {
        Self {
            max_retries: 5,
            base_delay: Duration::from_millis(50),
            max_delay: Duration::from_secs(60),
            retry_on_status: vec![408, 425, 429, 500, 502, 503, 504, 522, 524],
            retry_on_error: true,
            exponential: true,
            jitter: true,
        }
    }

    pub fn delay_for(&self, attempt: u32) -> Duration {
        let base = if !self.exponential {
            self.base_delay
        } else {
            let multiplier = 2u32.pow(attempt.min(10));
            let d = self.base_delay * multiplier;
            if d > self.max_delay {
                self.max_delay
            } else {
                d
            }
        };

        if self.jitter {
            let jitter_ms = (rand_jitter() % (base.as_millis().max(1) as u64)) as u64;
            Duration::from_millis(base.as_millis() as u64 / 2 + jitter_ms / 2)
        } else {
            base
        }
    }

    pub fn should_retry_status(&self, status: u16) -> bool {
        self.retry_on_status.contains(&status)
    }

    pub fn should_retry_error(&self) -> bool {
        self.retry_on_error
    }
}

fn rand_jitter() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(12345)
}

pub async fn retry_async<F, Fut, T>(policy: &RetryPolicy, mut f: F) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    let mut last_err: Option<SwpreqError> = None;

    for attempt in 0..=policy.max_retries {
        match f().await {
            Ok(v) => return Ok(v),
            Err(e) => {
                if attempt >= policy.max_retries {
                    return Err(e);
                }
                if !policy.should_retry_error() {
                    return Err(e);
                }
                last_err = Some(e);
                let delay = policy.delay_for(attempt);
                tokio::time::sleep(delay).await;
            }
        }
    }

    Err(last_err.unwrap_or_else(|| SwpreqError::Internal("retry exhausted".to_string())))
}