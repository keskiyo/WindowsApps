use crate::catalog::sync::SyncRequest;
use crate::error::AppError;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};

pub(crate) type ScanResult<T> = Result<T, AppError>;

pub(crate) struct ScanJob<T> {
    pub request: SyncRequest,
    pub cancelled: Arc<AtomicBool>,
    waiters: Arc<Mutex<Vec<mpsc::Sender<ScanResult<T>>>>>,
}

struct Pending<T> {
    request: SyncRequest,
    waiters: Vec<mpsc::Sender<ScanResult<T>>>,
}

struct State<T> {
    active: Option<Active<T>>,
    pending: Option<Pending<T>>,
}

#[cfg(test)]
struct CompletionPause {
    entered: mpsc::Sender<()>,
    resume: mpsc::Receiver<()>,
}

struct Active<T> {
    request: SyncRequest,
    cancelled: Arc<AtomicBool>,
    waiters: Arc<Mutex<Vec<mpsc::Sender<ScanResult<T>>>>>,
}

pub(crate) enum Submission<T> {
    Start {
        job: ScanJob<T>,
        receiver: Option<mpsc::Receiver<ScanResult<T>>>,
    },
    Wait(mpsc::Receiver<ScanResult<T>>),
    Coalesced,
}

pub(crate) struct ScanCoordinator<T> {
    state: Mutex<State<T>>,
    #[cfg(test)]
    completion_pause: Mutex<Option<CompletionPause>>,
}

impl<T: Clone> Default for ScanCoordinator<T> {
    fn default() -> Self {
        Self {
            state: Mutex::new(State {
                active: None,
                pending: None,
            }),
            #[cfg(test)]
            completion_pause: Mutex::new(None),
        }
    }
}

impl<T: Clone> ScanCoordinator<T> {
    pub(crate) fn submit(&self, request: SyncRequest, wants_result: bool) -> Submission<T> {
        let (sender, receiver) = mpsc::channel();
        let mut state = self.state.lock().expect("scan coordinator poisoned");
        let waiters = wants_result
            .then_some(sender)
            .into_iter()
            .collect::<Vec<_>>();
        let Some(active) = state.active.as_mut() else {
            let cancelled = Arc::new(AtomicBool::new(false));
            let waiters = Arc::new(Mutex::new(waiters));
            state.active = Some(Active {
                request,
                cancelled: Arc::clone(&cancelled),
                waiters: Arc::clone(&waiters),
            });
            return Submission::Start {
                job: ScanJob {
                    request,
                    cancelled,
                    waiters,
                },
                receiver: wants_result.then_some(receiver),
            };
        };

        if request.is_interactive() && !active.request.is_interactive() {
            active.cancelled.store(true, Ordering::Relaxed);
            merge_pending(&mut state.pending, request, waiters);
            return if wants_result {
                Submission::Wait(receiver)
            } else {
                Submission::Coalesced
            };
        }

        if request > active.request && request.is_interactive() {
            active.cancelled.store(true, Ordering::Relaxed);
            let mut inherited = active.waiters.lock().expect("scan waiters poisoned");
            let mut combined = std::mem::take(&mut *inherited);
            combined.extend(waiters);
            drop(inherited);
            merge_pending(&mut state.pending, request, combined);
            return if wants_result {
                Submission::Wait(receiver)
            } else {
                Submission::Coalesced
            };
        }

        if request.is_interactive() && active.request.is_interactive() {
            active
                .waiters
                .lock()
                .expect("scan waiters poisoned")
                .extend(waiters);
            return Submission::Wait(receiver);
        }

        merge_pending(&mut state.pending, request, waiters);
        Submission::Coalesced
    }

    pub(crate) fn complete(&self, job: ScanJob<T>, result: ScanResult<T>) -> Option<ScanJob<T>> {
        let mut state = self.state.lock().expect("scan coordinator poisoned");
        for waiter in job.waiters.lock().expect("scan waiters poisoned").drain(..) {
            let _ = waiter.send(result.clone());
        }
        #[cfg(test)]
        self.pause_after_waiters_are_drained();
        state.active = None;
        let pending = state.pending.take()?;
        let cancelled = Arc::new(AtomicBool::new(false));
        let waiters = Arc::new(Mutex::new(pending.waiters));
        state.active = Some(Active {
            request: pending.request,
            cancelled: Arc::clone(&cancelled),
            waiters: Arc::clone(&waiters),
        });
        Some(ScanJob {
            request: pending.request,
            cancelled,
            waiters,
        })
    }

    #[cfg(test)]
    fn pause_after_waiters_are_drained_for_test(&self) -> (mpsc::Receiver<()>, mpsc::Sender<()>) {
        let (entered_sender, entered_receiver) = mpsc::channel();
        let (resume_sender, resume_receiver) = mpsc::channel();
        *self
            .completion_pause
            .lock()
            .expect("completion pause poisoned") = Some(CompletionPause {
            entered: entered_sender,
            resume: resume_receiver,
        });
        (entered_receiver, resume_sender)
    }

    #[cfg(test)]
    fn pause_after_waiters_are_drained(&self) {
        let Some(pause) = self
            .completion_pause
            .lock()
            .expect("completion pause poisoned")
            .take()
        else {
            return;
        };
        let _ = pause.entered.send(());
        let _ = pause.resume.recv();
    }

    pub(crate) fn cancel_all(&self) {
        let mut state = self.state.lock().expect("scan coordinator poisoned");
        if let Some(active) = &state.active {
            active.cancelled.store(true, Ordering::Relaxed);
        }
        if let Some(pending) = state.pending.take() {
            for waiter in pending.waiters {
                let _ = waiter.send(Err(AppError::ScanCancelled));
            }
        }
    }
}

fn merge_pending<T>(
    pending: &mut Option<Pending<T>>,
    request: SyncRequest,
    mut waiters: Vec<mpsc::Sender<ScanResult<T>>>,
) {
    match pending {
        Some(current) => {
            current.request = current.request.max(request);
            current.waiters.append(&mut waiters);
        }
        None => {
            *pending = Some(Pending { request, waiters });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn completion_keeps_the_state_locked_while_finishing_waiters() {
        let coordinator = Arc::new(ScanCoordinator::<u32>::default());
        let Submission::Start { job, .. } = coordinator.submit(SyncRequest::Refresh, false) else {
            panic!("refresh should start");
        };
        let (entered, resume) = coordinator.pause_after_waiters_are_drained_for_test();
        let completing = {
            let coordinator = Arc::clone(&coordinator);
            thread::spawn(move || coordinator.complete(job, Ok(7)))
        };

        entered
            .recv_timeout(Duration::from_secs(1))
            .expect("completion should pause after draining waiters");
        let state_is_locked = coordinator.state.try_lock().is_err();
        resume.send(()).expect("completion should still be waiting");
        assert!(completing
            .join()
            .expect("completion should not panic")
            .is_none());
        assert!(state_is_locked);
    }

    #[test]
    fn interactive_refresh_cancels_background_scan_and_runs_next() {
        let coordinator = ScanCoordinator::<u32>::default();
        let Submission::Start {
            job: background, ..
        } = coordinator.submit(SyncRequest::Startup, false)
        else {
            panic!("background scan should start");
        };
        let Submission::Wait(refresh) = coordinator.submit(SyncRequest::Refresh, true) else {
            panic!("refresh should wait for replacement scan");
        };
        assert!(background.cancelled.load(Ordering::Relaxed));

        let next = coordinator
            .complete(background, Err("background superseded".to_string().into()))
            .expect("refresh should be pending");
        assert_eq!(next.request, SyncRequest::Refresh);
        assert!(refresh.try_recv().is_err());
        assert!(coordinator.complete(next, Ok(7)).is_none());
        assert_eq!(refresh.recv().unwrap(), Ok(7));
    }

    #[test]
    fn repeated_watch_requests_are_coalesced() {
        let coordinator = ScanCoordinator::<u32>::default();
        let Submission::Start { job: active, .. } = coordinator.submit(SyncRequest::Watch, false)
        else {
            panic!("watch should start");
        };
        assert!(matches!(
            coordinator.submit(SyncRequest::Watch, false),
            Submission::Coalesced
        ));
        let next = coordinator.complete(active, Ok(1)).unwrap();
        assert_eq!(next.request, SyncRequest::Watch);
        assert!(coordinator.complete(next, Ok(1)).is_none());
    }

    #[test]
    fn force_scan_replaces_pending_refresh() {
        let coordinator = ScanCoordinator::<u32>::default();
        let Submission::Start { job: active, .. } = coordinator.submit(SyncRequest::Startup, false)
        else {
            panic!("startup should start");
        };
        let Submission::Wait(refresh) = coordinator.submit(SyncRequest::Refresh, true) else {
            panic!("refresh should wait");
        };
        let Submission::Wait(force) = coordinator.submit(SyncRequest::Force, true) else {
            panic!("force should wait");
        };
        let next = coordinator
            .complete(active, Err("superseded".to_string().into()))
            .unwrap();
        assert_eq!(next.request, SyncRequest::Force);
        coordinator.complete(next, Ok(9));
        assert_eq!(refresh.recv().unwrap(), Ok(9));
        assert_eq!(force.recv().unwrap(), Ok(9));
    }

    #[test]
    fn cancelling_a_pending_scan_returns_the_typed_cancellation_error() {
        let coordinator = ScanCoordinator::<u32>::default();
        let Submission::Start { .. } = coordinator.submit(SyncRequest::Startup, false) else {
            panic!("startup scan should start");
        };
        let Submission::Wait(refresh) = coordinator.submit(SyncRequest::Refresh, true) else {
            panic!("refresh should wait");
        };

        coordinator.cancel_all();

        assert!(matches!(
            refresh.recv().unwrap(),
            Err(AppError::ScanCancelled)
        ));
    }
}
