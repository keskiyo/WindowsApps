use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

const MAX_CONCURRENT_LAUNCH_WAITS: usize = 8;

pub(crate) struct LaunchWaitLimiter {
    active: AtomicUsize,
}

impl Default for LaunchWaitLimiter {
    fn default() -> Self {
        Self {
            active: AtomicUsize::new(0),
        }
    }
}

impl LaunchWaitLimiter {
    pub(crate) fn acquire(self: &Arc<Self>) -> Option<LaunchWaitPermit> {
        self.active
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |active| {
                (active < MAX_CONCURRENT_LAUNCH_WAITS).then_some(active + 1)
            })
            .ok()
            .map(|_| LaunchWaitPermit {
                limiter: Arc::clone(self),
            })
    }
}

pub(crate) struct LaunchWaitPermit {
    limiter: Arc<LaunchWaitLimiter>,
}

impl Drop for LaunchWaitPermit {
    fn drop(&mut self) {
        self.limiter.active.fetch_sub(1, Ordering::AcqRel);
    }
}

#[cfg(test)]
mod tests {
    use crate::app_state::AppState;

    #[test]
    fn launch_wait_capacity_is_app_scoped() {
        let first = AppState::default();
        let second = AppState::default();
        let permits = (0..8)
            .map(|_| first.launch_waits.acquire().unwrap())
            .collect::<Vec<_>>();

        assert!(first.launch_waits.acquire().is_none());
        assert!(second.launch_waits.acquire().is_some());
        drop(permits);
        assert!(first.launch_waits.acquire().is_some());
    }
}
