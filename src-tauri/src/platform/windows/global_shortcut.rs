use serde::Serialize;
use std::sync::mpsc;
use std::thread::JoinHandle;
use std::time::Duration;
use tauri::AppHandle;
use windows::Win32::Foundation::{LPARAM, WPARAM};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, UnregisterHotKey, MOD_SHIFT, MOD_WIN, VK_Q,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetMessageW, PostThreadMessageW, MSG, WM_HOTKEY, WM_QUIT,
};

const HOTKEY_ID: i32 = 0x5741;
pub(crate) const LABEL: &str = "Win+Shift+Q";

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Status {
    pub available: bool,
    pub label: &'static str,
    // A static message, not the upstream HRESULT text: only fixed, safe strings cross IPC
    // (root §7 / `AGENTS_backend.md` §3).
    pub error: Option<&'static str>,
}

impl Default for Status {
    fn default() -> Self {
        Self {
            available: false,
            label: LABEL,
            error: Some("The shortcut has not been registered yet."),
        }
    }
}

/// Outcome of `register`: the status the settings page shows, plus the guard that owns the
/// hotkey thread. Both live in `AppState` — there is no process-wide singleton (§8).
pub(crate) struct RegisteredShortcut {
    pub status: Status,
    pub guard: Option<ShortcutGuard>,
}

/// Owns the hotkey thread. Dropping it unregisters the hotkey, ends the message loop and joins
/// the thread, so the registration and the `AppHandle` the thread holds do not outlive it —
/// `AGENTS_backend.md` §5 requires background work started at setup to have an owner and a
/// shutdown, and this was the one piece that was simply detached.
pub(crate) struct ShortcutGuard {
    thread: Option<JoinHandle<()>>,
    thread_id: u32,
}

impl Drop for ShortcutGuard {
    fn drop(&mut self) {
        // `WM_QUIT` ends `GetMessageW`; the thread unregisters the hotkey on its way out.
        let _ = unsafe { PostThreadMessageW(self.thread_id, WM_QUIT, WPARAM(0), LPARAM(0)) };
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

pub(crate) fn register(app: AppHandle) -> RegisteredShortcut {
    let (sender, receiver) = mpsc::channel();
    let thread = std::thread::spawn(move || unsafe {
        let registration = RegisterHotKey(None, HOTKEY_ID, MOD_WIN | MOD_SHIFT, VK_Q.0 as u32);
        let _ = sender.send(registration.as_ref().map(|_| GetCurrentThreadId()).ok());
        if registration.is_err() {
            return;
        }
        let mut message = MSG::default();
        while GetMessageW(&mut message, None, 0, 0).as_bool() {
            if message.message == WM_HOTKEY && message.wParam.0 == HOTKEY_ID as usize {
                crate::lifecycle::show_main_window(&app);
            }
        }
        // The hotkey is owned by this thread, so it must be released here.
        let _ = UnregisterHotKey(None, HOTKEY_ID);
    });
    // A slow `RegisterHotKey` (past the timeout) still leaves the thread running and the key
    // working; the status just reports unavailable — a false negative, never a false positive.
    let thread_id = receiver.recv_timeout(Duration::from_secs(1)).ok().flatten();
    match thread_id {
        Some(thread_id) => RegisteredShortcut {
            status: Status {
                available: true,
                label: LABEL,
                error: None,
            },
            guard: Some(ShortcutGuard {
                thread: Some(thread),
                thread_id,
            }),
        },
        None => RegisteredShortcut {
            status: Status {
                available: false,
                label: LABEL,
                error: Some(
                    "The shortcut could not be registered. Another application may already use it.",
                ),
            },
            guard: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shortcut_uses_the_physical_q_virtual_key() {
        assert_eq!(VK_Q.0, 0x51);
        assert_eq!(LABEL, "Win+Shift+Q");
    }

    // The message loop blocks in `GetMessageW` forever, so dropping the guard has to actually
    // deliver `WM_QUIT` and join. If it did not, the thread — and the `AppHandle` it holds —
    // would outlive the application, and the hotkey would stay registered.
    #[test]
    fn dropping_the_guard_stops_the_message_loop_promptly() {
        let Some(guard) = register_for_test() else {
            // Another process already owns Win+Shift+Q on this machine; nothing to join.
            return;
        };
        let started = std::time::Instant::now();

        drop(guard);

        assert!(started.elapsed() < Duration::from_secs(2));
    }

    /// `register` needs an `AppHandle`, which a unit test cannot build, so the guard is
    /// constructed from the same primitives the real path uses.
    fn register_for_test() -> Option<ShortcutGuard> {
        let (sender, receiver) = mpsc::channel();
        let thread = std::thread::spawn(move || unsafe {
            let registration = RegisterHotKey(None, HOTKEY_ID, MOD_WIN | MOD_SHIFT, VK_Q.0 as u32);
            let _ = sender.send(registration.as_ref().map(|_| GetCurrentThreadId()).ok());
            if registration.is_err() {
                return;
            }
            let mut message = MSG::default();
            while GetMessageW(&mut message, None, 0, 0).as_bool() {}
            let _ = UnregisterHotKey(None, HOTKEY_ID);
        });
        let thread_id = receiver
            .recv_timeout(Duration::from_secs(1))
            .ok()
            .flatten()?;
        Some(ShortcutGuard {
            thread: Some(thread),
            thread_id,
        })
    }
}
