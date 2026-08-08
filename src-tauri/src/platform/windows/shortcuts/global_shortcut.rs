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

pub(crate) struct RegisteredShortcut {
    pub status: Status,
    pub guard: Option<ShortcutGuard>,
}

pub(crate) struct ShortcutGuard {
    thread: Option<JoinHandle<()>>,
    thread_id: u32,
}

impl Drop for ShortcutGuard {
    fn drop(&mut self) {
        // SAFETY: `PostThreadMessageW` takes no pointer — both parameters are integers. The
        // thread id was reported by the hotkey thread itself and a `ShortcutGuard` is only built
        // while that thread is running; the join below is what guarantees it is still alive here,
        // because `Drop` runs before the `JoinHandle` is released. Posting to a thread that has
        // already exited fails with an error rather than being unsound, and the result is ignored.
        let _ = unsafe { PostThreadMessageW(self.thread_id, WM_QUIT, WPARAM(0), LPARAM(0)) };
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

pub(crate) fn register(app: AppHandle) -> RegisteredShortcut {
    let (id_sender, id_receiver) = mpsc::channel();
    let (status_sender, status_receiver) = mpsc::channel();
    // SAFETY: every call in this block is thread-affine, and all of them run on this one spawned
    // thread — which is exactly why the whole body is the unsafe region. `RegisterHotKey(None,…)`
    // associates the hotkey with the calling thread, `GetMessageW(None,…)` pumps that thread's own
    // queue, and `UnregisterHotKey` must be called from the registering thread; none of them take
    // a pointer from us except `&mut message`, a live local the callee fills. The loop exits only
    // on `WM_QUIT`, and unregistration is the last statement, so the hotkey is released before the
    // thread ends — including on the late-registration path, where the guard's `WM_QUIT` may
    // already be waiting in the queue when the loop starts.
    let thread = std::thread::spawn(move || unsafe {
        let _ = id_sender.send(GetCurrentThreadId());
        let registration = RegisterHotKey(None, HOTKEY_ID, MOD_WIN | MOD_SHIFT, VK_Q.0 as u32);
        let _ = status_sender.send(registration.is_ok());
        if registration.is_err() {
            return;
        }
        let mut message = MSG::default();
        while GetMessageW(&mut message, None, 0, 0).as_bool() {
            if message.message == WM_HOTKEY && message.wParam.0 == HOTKEY_ID as usize {
                crate::lifecycle::show_main_window(&app);
            }
        }
        let _ = UnregisterHotKey(None, HOTKEY_ID);
    });
    let Ok(thread_id) = id_receiver.recv_timeout(Duration::from_secs(1)) else {
        let _ = thread.join();
        return RegisteredShortcut {
            status: Status {
                available: false,
                label: LABEL,
                error: Some(
                    "The shortcut could not be registered. Another application may already use it.",
                ),
            },
            guard: None,
        };
    };
    let available = status_receiver
        .recv_timeout(Duration::from_secs(1))
        .unwrap_or(false);
    RegisteredShortcut {
        status: Status {
            available,
            label: LABEL,
            error: (!available).then_some(
                "The shortcut could not be registered. Another application may already use it.",
            ),
        },
        guard: Some(ShortcutGuard {
            thread: Some(thread),
            thread_id,
        }),
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

    #[test]
    fn dropping_the_guard_stops_the_message_loop_promptly() {
        let Some(guard) = register_for_test() else {
            return;
        };
        let started = std::time::Instant::now();

        drop(guard);

        assert!(started.elapsed() < Duration::from_secs(2));
    }

    #[test]
    fn a_thread_whose_registration_failed_is_still_owned_and_joined() {
        let blocker = register_for_test();
        let started = std::time::Instant::now();

        let guard = spawn_hotkey_thread(false);

        assert!(
            guard.is_some(),
            "the thread must be owned even when it failed"
        );
        drop(guard);
        assert!(started.elapsed() < Duration::from_secs(4));
        drop(blocker);
    }

    fn register_for_test() -> Option<ShortcutGuard> {
        spawn_hotkey_thread(true)
    }

    fn spawn_hotkey_thread(require_registration: bool) -> Option<ShortcutGuard> {
        let (id_sender, id_receiver) = mpsc::channel();
        let (status_sender, status_receiver) = mpsc::channel();
        // SAFETY: same thread-affinity contract as `register` — registration, message pump and
        // unregistration all run on this one spawned thread, and the only pointer handed to the
        // API is `&mut message`, a live local.
        let thread = std::thread::spawn(move || unsafe {
            let _ = id_sender.send(GetCurrentThreadId());
            let registration = RegisterHotKey(None, HOTKEY_ID, MOD_WIN | MOD_SHIFT, VK_Q.0 as u32);
            let _ = status_sender.send(registration.is_ok());
            if registration.is_err() {
                return;
            }
            let mut message = MSG::default();
            while GetMessageW(&mut message, None, 0, 0).as_bool() {}
            let _ = UnregisterHotKey(None, HOTKEY_ID);
        });
        let thread_id = id_receiver.recv_timeout(Duration::from_secs(1)).ok()?;
        let registered = status_receiver
            .recv_timeout(Duration::from_secs(1))
            .unwrap_or(false);
        if require_registration && !registered {
            let _ = thread.join();
            return None;
        }
        Some(ShortcutGuard {
            thread: Some(thread),
            thread_id,
        })
    }
}
