use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};
use windows::core::PCWSTR;
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::Storage::FileSystem::{
    CreateFileW, ReadDirectoryChangesW, FILE_FLAG_BACKUP_SEMANTICS, FILE_LIST_DIRECTORY,
    FILE_NOTIFY_CHANGE_DIR_NAME, FILE_NOTIFY_CHANGE_FILE_NAME, FILE_NOTIFY_CHANGE_LAST_WRITE,
    FILE_NOTIFY_CHANGE_SIZE, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows::Win32::System::Registry::{
    RegCloseKey, RegNotifyChangeKeyValue, RegOpenKeyExW, HKEY, HKEY_CURRENT_USER,
    HKEY_LOCAL_MACHINE, KEY_NOTIFY, REG_NOTIFY_CHANGE_LAST_SET, REG_NOTIFY_CHANGE_NAME,
};

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

pub struct WatcherGuard {
    stop: Arc<AtomicBool>,
}

#[derive(Clone, Copy)]
enum RegistryRoot {
    LocalMachine,
    CurrentUser,
}

impl Drop for WatcherGuard {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

pub fn start(paths: Vec<PathBuf>, on_change: Arc<dyn Fn() + Send + Sync>) -> WatcherGuard {
    let stop = Arc::new(AtomicBool::new(false));
    let (sender, receiver) = mpsc::channel::<()>();
    let debounce_stop = Arc::clone(&stop);
    std::thread::spawn(move || {
        let mut state = DebounceState::default();
        while !debounce_stop.load(Ordering::Relaxed) {
            match receiver.recv_timeout(Duration::from_millis(200)) {
                Ok(()) => state.push(Instant::now()),
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
            if state.take_if_ready(Instant::now(), Duration::from_secs(2)) {
                on_change();
            }
        }
    });

    for path in paths {
        if !path.is_dir() {
            continue;
        }
        spawn_directory_watcher(path, sender.clone(), Arc::clone(&stop));
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
        spawn_registry_watcher(root, subkey, sender.clone(), Arc::clone(&stop));
    }
    WatcherGuard { stop }
}

fn spawn_directory_watcher(path: PathBuf, sender: mpsc::Sender<()>, stop: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        let wide = wide(path.as_os_str());
        let handle = unsafe {
            CreateFileW(
                PCWSTR(wide.as_ptr()),
                FILE_LIST_DIRECTORY.0,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                None,
                OPEN_EXISTING,
                FILE_FLAG_BACKUP_SEMANTICS,
                None,
            )
        };
        let Ok(handle) = handle else {
            return;
        };
        let mut buffer = vec![0u8; 16 * 1024];
        while !stop.load(Ordering::Relaxed) {
            let mut bytes = 0u32;
            let result = unsafe {
                ReadDirectoryChangesW(
                    handle,
                    buffer.as_mut_ptr().cast(),
                    buffer.len() as u32,
                    true,
                    FILE_NOTIFY_CHANGE_FILE_NAME
                        | FILE_NOTIFY_CHANGE_DIR_NAME
                        | FILE_NOTIFY_CHANGE_LAST_WRITE
                        | FILE_NOTIFY_CHANGE_SIZE,
                    Some(&mut bytes),
                    None,
                    None,
                )
            };
            if result.is_err() {
                break;
            }
            if bytes > 0 && sender.send(()).is_err() {
                break;
            }
        }
        let _ = unsafe { CloseHandle(handle) };
    });
}

fn spawn_registry_watcher(
    root: RegistryRoot,
    subkey: &'static str,
    sender: mpsc::Sender<()>,
    stop: Arc<AtomicBool>,
) {
    std::thread::spawn(move || {
        let root = match root {
            RegistryRoot::LocalMachine => HKEY_LOCAL_MACHINE,
            RegistryRoot::CurrentUser => HKEY_CURRENT_USER,
        };
        let wide = wide(OsStr::new(subkey));
        let mut key = HKEY::default();
        let status =
            unsafe { RegOpenKeyExW(root, PCWSTR(wide.as_ptr()), None, KEY_NOTIFY, &mut key) };
        if status.is_err() {
            return;
        }
        while !stop.load(Ordering::Relaxed) {
            let status = unsafe {
                RegNotifyChangeKeyValue(
                    key,
                    true,
                    REG_NOTIFY_CHANGE_NAME | REG_NOTIFY_CHANGE_LAST_SET,
                    None,
                    false,
                )
            };
            if status.is_err() || sender.send(()).is_err() {
                break;
            }
        }
        let _ = unsafe { RegCloseKey(key) };
    });
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
}
