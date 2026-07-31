use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, ReadDirectoryChangesW, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OVERLAPPED,
    FILE_LIST_DIRECTORY, FILE_NOTIFY_CHANGE_DIR_NAME, FILE_NOTIFY_CHANGE_FILE_NAME,
    FILE_NOTIFY_CHANGE_LAST_WRITE, FILE_NOTIFY_CHANGE_SIZE, FILE_SHARE_DELETE, FILE_SHARE_READ,
    FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows::Win32::System::Registry::{
    RegCloseKey, RegNotifyChangeKeyValue, RegOpenKeyExW, HKEY, HKEY_CURRENT_USER,
    HKEY_LOCAL_MACHINE, KEY_NOTIFY, REG_NOTIFY_CHANGE_LAST_SET, REG_NOTIFY_CHANGE_NAME,
};
use windows::Win32::System::Threading::{CreateEventW, SetEvent, WaitForMultipleObjects, INFINITE};
use windows::Win32::System::IO::{CancelIoEx, GetOverlappedResult, OVERLAPPED};

#[derive(Default)]
struct DebounceState {
    last_event: Option<Instant>,
}

impl DebounceState {
    fn push(&mut self, now: Instant) {
        self.last_event = Some(now);
    }

    fn take_if_ready(&mut self, now: Instant, delay: Duration) -> bool {
        let ready = self
            .last_event
            .is_some_and(|last| now.saturating_duration_since(last) >= delay);
        if ready {
            self.last_event = None;
        }
        ready
    }
}

pub(crate) struct WatcherGuard {
    stop: Arc<AtomicBool>,
    stop_event: isize,
    threads: Vec<JoinHandle<()>>,
}

#[derive(Clone, Copy)]
enum RegistryRoot {
    LocalMachine,
    CurrentUser,
}

impl Drop for WatcherGuard {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        let stop_event = HANDLE(self.stop_event as *mut _);
        // SAFETY: `stop_event` was created by `start` and is owned by this guard; it is not closed
        // until after the join below, so every watcher thread is still waiting on a live handle.
        // Setting a manual-reset event releases all of them at once.
        let _ = unsafe { SetEvent(stop_event) };
        for thread in self.threads.drain(..) {
            let _ = thread.join();
        }
        // SAFETY: the close happens only after every thread that borrowed this handle has been
        // joined, so no wait can still reference it. `Drop` runs once, so it is closed once.
        let _ = unsafe { CloseHandle(stop_event) };
    }
}

/// Start watching. Returns `None` when the stop event cannot be created: without it the worker
/// threads could never be woken to exit, so running unwatched is the correct degradation —
/// freshness is lost until the next manual scan, but the app keeps working. Panicking here
/// would take down a scan-settings save or the whole startup.
pub(crate) fn start(
    paths: Vec<PathBuf>,
    on_change: Arc<dyn Fn() + Send + Sync>,
) -> Option<WatcherGuard> {
    let stop = Arc::new(AtomicBool::new(false));
    // SAFETY: `CreateEventW` takes no caller-owned memory here — default security attributes, and
    // a null name. It is created manual-reset (`true`) and unsignalled (`false`) so a single
    // `SetEvent` in `Drop` releases every watcher thread at once. Ownership stays with
    // `WatcherGuard`, which closes it after joining; failure returns `None` rather than leaving
    // threads that could never be woken.
    let Ok(stop_event) = (unsafe { CreateEventW(None, true, false, PCWSTR::null()) }) else {
        return None;
    };
    let (sender, receiver) = mpsc::channel::<()>();
    let mut threads = Vec::new();
    let debounce_stop = Arc::clone(&stop);
    threads.push(std::thread::spawn(move || {
        let mut state = DebounceState::default();
        while !debounce_stop.load(Ordering::Acquire) {
            match receiver.recv_timeout(Duration::from_millis(200)) {
                Ok(()) => state.push(Instant::now()),
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
            if state.take_if_ready(Instant::now(), Duration::from_secs(8)) {
                on_change();
            }
        }
    }));

    for path in paths {
        if path.is_dir() {
            threads.push(spawn_directory_watcher(
                path,
                sender.clone(),
                Arc::clone(&stop),
                stop_event.0 as isize,
            ));
        }
    }
    for (root, subkey) in [
        (
            RegistryRoot::LocalMachine,
            r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
        ),
        (
            RegistryRoot::LocalMachine,
            r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
        ),
        (
            RegistryRoot::CurrentUser,
            r"Software\Microsoft\Windows\CurrentVersion\Uninstall",
        ),
    ] {
        threads.push(spawn_registry_watcher(
            root,
            subkey,
            sender.clone(),
            Arc::clone(&stop),
            stop_event.0 as isize,
        ));
    }
    Some(WatcherGuard {
        stop,
        stop_event: stop_event.0 as isize,
        threads,
    })
}

fn spawn_directory_watcher(
    path: PathBuf,
    sender: mpsc::Sender<()>,
    stop: Arc<AtomicBool>,
    stop_event: isize,
) -> JoinHandle<()> {
    std::thread::spawn(move || {
        let stop_event = HANDLE(stop_event as *mut _);
        let wide = wide(path.as_os_str());
        // SAFETY: `wide` is a NUL-terminated UTF-16 buffer alive for the call; the two `None`
        // arguments are the optional security attributes and template handle.
        // `FILE_FLAG_BACKUP_SEMANTICS` is what makes opening a *directory* handle legal, and
        // `FILE_FLAG_OVERLAPPED` is required by the asynchronous `ReadDirectoryChangesW` below.
        // The handle is closed on every exit path of this thread.
        let handle = unsafe {
            CreateFileW(
                PCWSTR(wide.as_ptr()),
                FILE_LIST_DIRECTORY.0,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                None,
                OPEN_EXISTING,
                FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OVERLAPPED,
                None,
            )
        };
        let Ok(handle) = handle else {
            return;
        };
        // SAFETY: as for the stop event, but auto-reset: it is signalled once per completed read
        // and consumed by the wait below. On failure the directory handle opened above is closed
        // before returning, so the early exit leaks nothing.
        let change_event = match unsafe { CreateEventW(None, false, false, PCWSTR::null()) } {
            Ok(event) => event,
            Err(_) => {
                // SAFETY: `handle` is the live directory handle from this thread, closed once.
                let _ = unsafe { CloseHandle(handle) };
                return;
            }
        };
        let mut buffer = vec![0u8; 16 * 1024];
        // `overlapped` and `buffer` are the kernel's write targets for the whole lifetime of a
        // pending `ReadDirectoryChangesW`, so both must outlive every exit path of the loop —
        // declaring `overlapped` per iteration would drop it off the stack while the I/O could
        // still be in flight.
        let mut overlapped = OVERLAPPED {
            hEvent: change_event,
            ..Default::default()
        };
        while !stop.load(Ordering::Acquire) {
            // SAFETY: this starts an *asynchronous* read, so the kernel keeps writing into
            // `buffer` and `overlapped` after the call returns. Both are declared outside the loop
            // and outlive every exit path: the `WAIT_OBJECT_0` branch cancels the operation and
            // then blocks in `GetOverlappedResult` until it has really finished, and the error
            // branch only breaks after the call itself failed to queue anything. The declared
            // length matches `buffer`'s real length, so the kernel cannot write past it.
            // `overlapped.hEvent` is the auto-reset event waited on below.
            if unsafe {
                ReadDirectoryChangesW(
                    handle,
                    buffer.as_mut_ptr().cast(),
                    buffer.len() as u32,
                    true,
                    FILE_NOTIFY_CHANGE_FILE_NAME
                        | FILE_NOTIFY_CHANGE_DIR_NAME
                        | FILE_NOTIFY_CHANGE_LAST_WRITE
                        | FILE_NOTIFY_CHANGE_SIZE,
                    None,
                    Some(&mut overlapped),
                    None,
                )
            }
            .is_err()
            {
                break;
            }
            // SAFETY: both handles are live — the stop event is kept open by `WatcherGuard` until
            // after this thread is joined, and the change event is closed only after this loop.
            // The slice is a live local array of exactly the two handles being waited on.
            let wait =
                unsafe { WaitForMultipleObjects(&[stop_event, change_event], false, INFINITE) };
            if wait == WAIT_OBJECT_0 {
                // `CancelIoEx` only *requests* cancellation. Wait for the operation to actually
                // finish before the buffer and `overlapped` go away, otherwise the kernel may
                // still write into freed memory.
                // SAFETY: `handle` is live and `overlapped` identifies the read queued above and
                // is still at its original address — it is declared outside the loop precisely so
                // the kernel's pointer stays valid until the wait below confirms completion.
                let _ = unsafe { CancelIoEx(handle, Some(&overlapped)) };
                let mut transferred = 0_u32;
                // SAFETY: the `true` argument blocks until the cancelled I/O has actually
                // completed, which is what makes it safe for `buffer` and `overlapped` to be
                // dropped when this thread returns. `transferred` is a live local.
                let _ = unsafe { GetOverlappedResult(handle, &overlapped, &mut transferred, true) };
                break;
            }
            if wait.0 == WAIT_OBJECT_0.0 + 1 && sender.send(()).is_err() {
                break;
            }
        }
        // SAFETY: both handles were created by this thread, no I/O is still pending on them (the
        // loop either never queued a read or waited for its completion above), and this is the
        // single close for each.
        let _ = unsafe { CloseHandle(change_event) };
        let _ = unsafe { CloseHandle(handle) };
    })
}

fn spawn_registry_watcher(
    root: RegistryRoot,
    subkey: &'static str,
    sender: mpsc::Sender<()>,
    stop: Arc<AtomicBool>,
    stop_event: isize,
) -> JoinHandle<()> {
    std::thread::spawn(move || {
        let stop_event = HANDLE(stop_event as *mut _);
        let root = match root {
            RegistryRoot::LocalMachine => HKEY_LOCAL_MACHINE,
            RegistryRoot::CurrentUser => HKEY_CURRENT_USER,
        };
        let wide = wide(OsStr::new(subkey));
        let mut key = HKEY::default();
        // SAFETY: `root` is a predefined hive handle, `wide` is a NUL-terminated UTF-16 subkey
        // alive for the call, and `key` is a live local the callee writes. A missing key (the
        // WOW6432Node hive on a 32-bit-only system, for example) is an error return, and the
        // thread exits before `key` is used.
        if unsafe { RegOpenKeyExW(root, PCWSTR(wide.as_ptr()), None, KEY_NOTIFY, &mut key) }
            .is_err()
        {
            return;
        }
        // SAFETY: an auto-reset, unnamed event with default security, as above. On failure the
        // key opened above is closed before returning.
        let change_event = match unsafe { CreateEventW(None, false, false, PCWSTR::null()) } {
            Ok(event) => event,
            Err(_) => {
                // SAFETY: `key` was successfully opened above and is closed once.
                let _ = unsafe { RegCloseKey(key) };
                return;
            }
        };
        while !stop.load(Ordering::Acquire) {
            // SAFETY: `key` is open for the whole loop and `change_event` outlives it. The
            // notification is asynchronous, but unlike the directory watcher it writes nothing
            // into our memory — it only signals the event — so no buffer has to stay alive.
            // Re-arming each iteration is required: a registry notification is one-shot.
            if unsafe {
                RegNotifyChangeKeyValue(
                    key,
                    true,
                    REG_NOTIFY_CHANGE_NAME | REG_NOTIFY_CHANGE_LAST_SET,
                    Some(change_event),
                    true,
                )
            }
            .is_err()
            {
                break;
            }
            // SAFETY: both handles are live for the same reasons as in the directory watcher —
            // the stop event outlives every watcher thread, the change event outlives this loop.
            let wait =
                unsafe { WaitForMultipleObjects(&[stop_event, change_event], false, INFINITE) };
            if wait == WAIT_OBJECT_0 {
                break;
            }
            if wait.0 == WAIT_OBJECT_0.0 + 1 && sender.send(()).is_err() {
                break;
            }
        }
        // SAFETY: both were created/opened by this thread and are released once. A pending
        // registry notification does not reference caller memory, so no completion wait is needed
        // before closing — unlike the overlapped directory read.
        let _ = unsafe { CloseHandle(change_event) };
        let _ = unsafe { RegCloseKey(key) };
    })
}

fn wide(value: &OsStr) -> Vec<u16> {
    value.encode_wide().chain(Some(0)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeated_events_are_coalesced_until_debounce_expires() {
        let start = Instant::now();
        let mut state = DebounceState::default();
        state.push(start);
        state.push(start + Duration::from_millis(500));

        assert!(!state.take_if_ready(start + Duration::from_secs(2), Duration::from_secs(2)));
        assert!(state.take_if_ready(start + Duration::from_millis(2501), Duration::from_secs(2)));
        assert!(!state.take_if_ready(start + Duration::from_secs(5), Duration::from_secs(2)));
    }

    #[test]
    fn watcher_guard_stops_blocked_registry_watchers() {
        let started = Instant::now();
        let guard = start(Vec::new(), Arc::new(|| {}));
        // `start` is fallible now; the guard must still be created here, otherwise this test
        // would silently stop covering the shutdown path it exists for.
        assert!(guard.is_some());
        drop(guard);
        assert!(started.elapsed() < Duration::from_secs(2));
    }

    // The watcher is restarted on every scan-settings save, so the stop path runs repeatedly in
    // normal use: cancelling the pending overlapped read must complete before the buffer and
    // OVERLAPPED are released.
    #[test]
    fn repeated_start_and_stop_cycles_stay_responsive() {
        let directory = tempfile::tempdir().unwrap();
        for _ in 0..5 {
            let started = Instant::now();
            let guard = start(vec![directory.path().to_path_buf()], Arc::new(|| {}));
            assert!(guard.is_some());
            drop(guard);
            assert!(started.elapsed() < Duration::from_secs(5));
        }
    }
}
