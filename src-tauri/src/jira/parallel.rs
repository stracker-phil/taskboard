//! Asking Jira several things at once: up to the rate limit's burst in flight, answers in the order asked.

use std::future::Future;

use futures_util::stream::{self, StreamExt};

use super::limiter::BURST;
use crate::error::AppResult;

/// Runs `run` for every item, `BURST` at a time, and keeps the results in the items' order.
pub async fn run_all<I, F, Fut>(items: I, run: F) -> Vec<Fut::Output>
where
    I: IntoIterator,
    F: FnMut(I::Item) -> Fut,
    Fut: Future,
{
    stream::iter(items).map(run).buffered(BURST).collect().await
}

/// `run_all` for questions: the answers in order, or the first failure.
pub async fn ask_all<I, T, F, Fut>(items: I, ask: F) -> AppResult<Vec<T>>
where
    I: IntoIterator,
    F: FnMut(I::Item) -> Fut,
    Fut: Future<Output = AppResult<T>>,
{
    run_all(items, ask).await.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    use tokio::time::{Instant, sleep};

    use super::*;
    use crate::error::{AppError, ErrorKind};

    async fn after(ms: u64) {
        sleep(Duration::from_millis(ms)).await;
    }

    async fn elapsed_running(items: usize) -> Duration {
        let start = Instant::now();
        run_all(0..items, |_| after(100)).await;
        start.elapsed()
    }

    #[tokio::test(start_paused = true)]
    async fn results_keep_the_items_order_when_later_items_finish_first() {
        let found = run_all([300_u64, 200, 100], |ms| async move {
            after(ms).await;
            ms
        })
        .await;

        assert_eq!(found, [300, 200, 100]);
    }

    #[tokio::test(start_paused = true)]
    async fn no_items_give_no_results() {
        let found = run_all(Vec::<u64>::new(), |ms| async move { ms }).await;

        assert_eq!(found, Vec::<u64>::new());
    }

    #[tokio::test(start_paused = true)]
    async fn up_to_the_burst_of_items_run_together() {
        let elapsed = elapsed_running(BURST).await;

        assert!(
            elapsed >= Duration::from_millis(100) && elapsed < Duration::from_millis(150),
            "{elapsed:?}"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn items_beyond_the_burst_wait_for_a_free_slot() {
        let elapsed = elapsed_running(2 * BURST).await;

        assert!(
            elapsed >= Duration::from_millis(200) && elapsed < Duration::from_millis(250),
            "{elapsed:?}"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn never_more_than_the_burst_run_at_once() {
        let active = AtomicUsize::new(0);
        let peak = AtomicUsize::new(0);
        let (active, peak) = (&active, &peak);

        run_all(0..3 * BURST, |_| async move {
            let now = active.fetch_add(1, Ordering::SeqCst) + 1;
            peak.fetch_max(now, Ordering::SeqCst);
            after(100).await;
            active.fetch_sub(1, Ordering::SeqCst);
        })
        .await;

        assert_eq!(peak.load(Ordering::SeqCst), BURST);
    }

    #[tokio::test(start_paused = true)]
    async fn ask_all_returns_the_answers_in_the_items_order() {
        let found = ask_all([300_u64, 100, 200], |ms| async move {
            after(ms).await;
            Ok(ms)
        })
        .await
        .unwrap();

        assert_eq!(found, [300, 100, 200]);
    }

    #[tokio::test(start_paused = true)]
    async fn ask_all_returns_the_failure_when_one_item_fails() {
        let err = ask_all([1_u64, 2, 3], |n| async move {
            if n == 2 {
                Err(AppError::NotFound("two".into()))
            } else {
                Ok(n)
            }
        })
        .await
        .unwrap_err();

        assert_eq!(err.kind(), ErrorKind::NotFound);
    }

    #[tokio::test(start_paused = true)]
    async fn ask_all_returns_the_first_failure_in_the_items_order() {
        let err = ask_all(
            [("slow", 300_u64), ("quick", 100)],
            |(name, ms)| async move {
                after(ms).await;
                Err::<u64, _>(AppError::InvalidInput(name.into()))
            },
        )
        .await
        .unwrap_err();

        assert!(
            matches!(&err, AppError::InvalidInput(message) if message == "slow"),
            "{err:?}"
        );
    }
}
