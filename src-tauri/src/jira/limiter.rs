//! Taskboard's own rate limit, shared by every Jira request: a burst of 5, then 5 requests per second.

use std::sync::Mutex;
use std::time::Duration;

use tokio::time::Instant;

pub struct Limiter {
    interval: Duration,
    burst: u32,
    /// When the next request would be due if requests were evenly spaced (GCRA).
    next_due: Mutex<Option<Instant>>,
}

impl Default for Limiter {
    fn default() -> Self {
        Self::new(5, 5)
    }
}

impl Limiter {
    pub fn new(per_second: u32, burst: u32) -> Self {
        Self {
            interval: Duration::from_secs(1) / per_second.max(1),
            burst: burst.max(1),
            next_due: Mutex::new(None),
        }
    }

    /// Waits until the next request may go out.
    pub async fn acquire(&self) {
        let start_at = {
            let now = Instant::now();
            let mut next_due = self.next_due.lock().unwrap_or_else(|e| e.into_inner());
            let due = next_due.map_or(now, |due| due.max(now));
            *next_due = Some(due + self.interval);
            let tolerance = self.interval * (self.burst - 1);
            due.checked_sub(tolerance).unwrap_or(now)
        };
        tokio::time::sleep_until(start_at).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn acquire_times(limiter: &Limiter, count: u32) -> Duration {
        let start = Instant::now();
        for _ in 0..count {
            limiter.acquire().await;
        }
        start.elapsed()
    }

    #[tokio::test(start_paused = true)]
    async fn the_first_five_requests_go_out_immediately() {
        assert_eq!(acquire_times(&Limiter::default(), 5).await, Duration::ZERO);
    }

    #[tokio::test(start_paused = true)]
    async fn after_the_burst_requests_are_spaced_five_per_second() {
        let elapsed = acquire_times(&Limiter::default(), 20).await;
        assert!(
            elapsed >= Duration::from_millis(3000) && elapsed < Duration::from_millis(3200),
            "{elapsed:?}"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn the_burst_is_available_again_after_idling() {
        let limiter = Limiter::default();
        acquire_times(&limiter, 10).await;
        tokio::time::sleep(Duration::from_secs(10)).await;
        assert_eq!(acquire_times(&limiter, 5).await, Duration::ZERO);
    }
}
