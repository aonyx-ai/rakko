//! The wait between two requests to a registry

use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

/// The wait between two requests to a registry
///
/// Each call of [`wait`][Pace::wait] reserves the next free moment, at least
/// one interval after the moment before it, and sleeps until that moment.
/// The first call does not sleep. The reservation happens under a lock that
/// no sleep holds, so requests that run at the same time get moments one
/// interval apart.
#[derive(Debug)]
pub(crate) struct Pace {
    /// The shortest time between two requests
    interval: Duration,

    /// The moment that the last call reserved, or `None` before the first
    /// call
    last: Mutex<Option<Instant>>,
}

impl Pace {
    /// Creates a pace with the shortest time between two requests
    pub(crate) fn new(interval: Duration) -> Self {
        Self {
            interval,
            last: Mutex::new(None),
        }
    }

    /// Sleeps until one interval has passed since the moment that the last
    /// call reserved
    ///
    /// The sleep needs a Tokio runtime with its timer.
    pub(crate) async fn wait(&self) {
        let delay = {
            let mut last = self.last.lock().unwrap_or_else(PoisonError::into_inner);
            let now = Instant::now();
            let moment = match *last {
                Some(previous) => (previous + self.interval).max(now),
                None => now,
            };
            *last = Some(moment);

            moment - now
        };

        if !delay.is_zero() {
            tokio::time::sleep(delay).await;
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use super::*;

    /// An interval that a test can wait for without slowing the suite
    const INTERVAL: Duration = Duration::from_millis(50);

    #[test]
    fn pace_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Pace>();
    }

    #[tokio::test]
    async fn wait_after_a_wait_sleeps_for_the_interval() {
        let pace = Pace::new(INTERVAL);
        pace.wait().await;
        let start = Instant::now();

        pace.wait().await;

        assert!(start.elapsed() >= INTERVAL, "got {:?}", start.elapsed());
    }

    #[tokio::test]
    async fn wait_for_the_first_time_does_not_sleep() {
        let pace = Pace::new(Duration::from_secs(60));
        let start = Instant::now();

        pace.wait().await;

        assert!(start.elapsed() < Duration::from_secs(60));
    }
}
