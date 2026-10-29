//! When to send a request again: `429`, and `503` with `Retry-After`, up to 4 times. Honors
//! `Retry-After`, else backs off exponentially with full jitter.

use std::future::Future;
use std::sync::Mutex;
use std::time::Duration;

use chrono::{DateTime, Utc};

use crate::net::{Answer, NetError};

pub const MAX_RETRIES: u32 = 4;
const BACKOFF_BASE: Duration = Duration::from_secs(1);
const BACKOFF_CAP: Duration = Duration::from_secs(30);
/// A longer `Retry-After` would stall the sync job; the next scheduled run tries again instead.
const RETRY_AFTER_CAP: Duration = Duration::from_secs(15 * 60);

/// Writes are never sent twice: Jira may have applied the first one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Retry {
    Allowed,
    Never,
}

/// How long to wait before retry number `retries_done + 1`; `None` gives up and returns the answer.
pub fn delay(
    retries_done: u32,
    status: u16,
    retry_after: Option<&str>,
    now: DateTime<Utc>,
    rng: &mut fastrand::Rng,
) -> Option<Duration> {
    if retries_done >= MAX_RETRIES {
        return None;
    }
    let asked = retry_after.and_then(|value| parse_retry_after(value, now));
    match (status, asked) {
        (429 | 503, Some(wait)) => Some(wait.min(RETRY_AFTER_CAP)),
        (429, None) => Some(backoff(retries_done, rng)),
        _ => None,
    }
}

/// Full jitter: anywhere between zero and the exponential ceiling.
fn backoff(retries_done: u32, rng: &mut fastrand::Rng) -> Duration {
    let ceiling = BACKOFF_BASE
        .saturating_mul(2u32.saturating_pow(retries_done))
        .min(BACKOFF_CAP);
    ceiling.mul_f64(rng.f64())
}

/// Seconds or an HTTP date; a date in the past means now.
fn parse_retry_after(value: &str, now: DateTime<Utc>) -> Option<Duration> {
    let value = value.trim();
    if let Ok(seconds) = value.parse::<u64>() {
        return Some(Duration::from_secs(seconds));
    }
    let at = DateTime::parse_from_rfc2822(value).ok()?;
    Some((at.with_timezone(&Utc) - now).to_std().unwrap_or_default())
}

/// Sends until the answer is final under the retry policy.
pub async fn send_with_retries<F, Fut>(
    retry: Retry,
    rng: &Mutex<fastrand::Rng>,
    mut send: F,
) -> Result<Answer, NetError>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<Answer, NetError>>,
{
    let mut retries_done = 0;
    loop {
        let answer = send().await?;
        if retry == Retry::Never {
            return Ok(answer);
        }
        let wait = {
            let mut rng = rng.lock().unwrap_or_else(|e| e.into_inner());
            delay(
                retries_done,
                answer.status,
                answer.retry_after.as_deref(),
                Utc::now(),
                &mut rng,
            )
        };
        let Some(wait) = wait else {
            return Ok(answer);
        };
        let reason = answer.rate_limit_reason.as_deref().unwrap_or("none given");
        log::warn!(
            "Jira answered {} (reason: {reason}); retrying in {:.1}s",
            answer.status,
            wait.as_secs_f64()
        );
        tokio::time::sleep(wait).await;
        retries_done += 1;
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU32, Ordering};

    use chrono::TimeZone;
    use tokio::time::Instant;

    use super::*;

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 28, 10, 0, 0).unwrap()
    }

    fn wait(retries_done: u32, status: u16, retry_after: Option<&str>) -> Option<Duration> {
        delay(
            retries_done,
            status,
            retry_after,
            now(),
            &mut fastrand::Rng::with_seed(7),
        )
    }

    fn secs(n: u64) -> Option<Duration> {
        Some(Duration::from_secs(n))
    }

    fn answer(status: u16, retry_after: Option<&str>) -> Answer {
        Answer {
            status,
            retry_after: retry_after.map(Into::into),
            rate_limit_reason: None,
            body: Vec::new(),
        }
    }

    #[test]
    fn retry_after_in_seconds_is_honored_for_429_and_503() {
        assert_eq!(wait(0, 429, Some("2")), secs(2));
        assert_eq!(wait(0, 503, Some("3")), secs(3));
    }

    #[test]
    fn retry_after_as_an_http_date_is_the_time_until_then() {
        let later = (now() + chrono::Duration::seconds(10)).to_rfc2822();
        assert_eq!(wait(0, 429, Some(&later)), secs(10));
    }

    #[test]
    fn retry_after_date_in_the_past_means_no_wait() {
        let earlier = (now() - chrono::Duration::seconds(60)).to_rfc2822();
        assert_eq!(wait(0, 429, Some(&earlier)), Some(Duration::ZERO));
    }

    #[test]
    fn a_huge_retry_after_is_capped_at_fifteen_minutes() {
        assert_eq!(wait(0, 429, Some("86400")), secs(15 * 60));
    }

    #[test]
    fn a_503_without_retry_after_is_not_retried() {
        assert_eq!(wait(0, 503, None), None);
    }

    #[test]
    fn other_statuses_are_never_retried() {
        for status in [200, 400, 401, 403, 404, 500] {
            assert_eq!(wait(0, status, None), None, "{status}");
            assert_eq!(wait(0, status, Some("2")), None, "{status}");
        }
    }

    #[test]
    fn a_429_without_retry_after_backs_off_within_the_capped_exponential_ceiling() {
        for retries_done in 0..MAX_RETRIES {
            let ceiling = Duration::from_secs(2u64.pow(retries_done)).min(BACKOFF_CAP);
            for seed in 0..200 {
                let got = delay(
                    retries_done,
                    429,
                    None,
                    now(),
                    &mut fastrand::Rng::with_seed(seed),
                )
                .unwrap();
                assert!(got <= ceiling, "retry {retries_done}, seed {seed}: {got:?}");
            }
        }
    }

    #[test]
    fn jitter_varies_between_seeds() {
        let waits: Vec<Option<Duration>> = (0..20)
            .map(|seed| delay(3, 429, None, now(), &mut fastrand::Rng::with_seed(seed)))
            .collect();
        assert!(waits.iter().any(|w| *w != waits[0]));
    }

    #[test]
    fn it_gives_up_after_the_maximum_number_of_retries() {
        assert_eq!(wait(MAX_RETRIES, 429, Some("2")), None);
        assert_eq!(wait(MAX_RETRIES, 429, None), None);
    }

    #[tokio::test(start_paused = true)]
    async fn a_429_with_retry_after_is_retried_after_that_wait() {
        let calls = AtomicU32::new(0);
        let rng = Mutex::new(fastrand::Rng::with_seed(1));
        let start = Instant::now();
        let result = send_with_retries(Retry::Allowed, &rng, || {
            let call = calls.fetch_add(1, Ordering::SeqCst);
            async move {
                Ok(if call == 0 {
                    answer(429, Some("2"))
                } else {
                    answer(200, None)
                })
            }
        })
        .await;
        assert_eq!(result, Ok(answer(200, None)));
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert!(
            start.elapsed() >= Duration::from_secs(2),
            "{:?}",
            start.elapsed()
        );
    }

    #[tokio::test(start_paused = true)]
    async fn the_last_429_is_returned_after_four_retries() {
        let calls = AtomicU32::new(0);
        let rng = Mutex::new(fastrand::Rng::with_seed(1));
        let result = send_with_retries(Retry::Allowed, &rng, || {
            calls.fetch_add(1, Ordering::SeqCst);
            async { Ok(answer(429, None)) }
        })
        .await;
        assert_eq!(result, Ok(answer(429, None)));
        assert_eq!(calls.load(Ordering::SeqCst), 5);
    }

    #[tokio::test(start_paused = true)]
    async fn writes_are_sent_once_and_the_429_is_returned() {
        let calls = AtomicU32::new(0);
        let rng = Mutex::new(fastrand::Rng::with_seed(1));
        let result = send_with_retries(Retry::Never, &rng, || {
            calls.fetch_add(1, Ordering::SeqCst);
            async { Ok(answer(429, Some("2"))) }
        })
        .await;
        assert_eq!(result, Ok(answer(429, Some("2"))));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn a_network_error_is_returned_at_once() {
        let calls = AtomicU32::new(0);
        let rng = Mutex::new(fastrand::Rng::with_seed(1));
        let result = send_with_retries(Retry::Allowed, &rng, || {
            calls.fetch_add(1, Ordering::SeqCst);
            async { Err(NetError::Offline("unreachable".into())) }
        })
        .await;
        assert_eq!(result, Err(NetError::Offline("unreachable".into())));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}
