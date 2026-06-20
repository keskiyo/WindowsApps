use serde::Serialize;
use std::sync::{mpsc, Mutex, OnceLock};
use std::time::Duration;
use tauri::AppHandle;
use windows::Win32::UI::Input::KeyboardAndMouse::{RegisterHotKey, MOD_SHIFT, MOD_WIN, VK_Q};
use windows::Win32::UI::WindowsAndMessaging::{GetMessageW, MSG, WM_HOTKEY};

const HOTKEY_ID: i32 = 0x5741;
pub const LABEL: &str = "Win+Shift+Q";

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub available: bool,
    pub label: &'static str,
    pub error: Option<String>,
}

static STATUS: OnceLock<Mutex<Status>> = OnceLock::new();

fn status_cell() -> &'static Mutex<Status> {
    STATUS.get_or_init(|| Mutex::new(Status { available: false, label: LABEL, error: Some("Shortcut has not been registered".into()) }))
}

pub fn status() -> Status {
    status_cell().lock().map(|value| value.clone()).unwrap_or(Status { available: false, label: LABEL, error: Some("Shortcut status is unavailable".into()) })
}

pub fn register(app: AppHandle) {
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || unsafe {
        let registration = RegisterHotKey(None, HOTKEY_ID, MOD_WIN | MOD_SHIFT, VK_Q.0 as u32);
        let _ = sender.send(registration.as_ref().map(|_| ()).map_err(|error| error.to_string()));
        if registration.is_err() { return; }
        let mut message = MSG::default();
        while GetMessageW(&mut message, None, 0, 0).as_bool() {
            if message.message == WM_HOTKEY && message.wParam.0 == HOTKEY_ID as usize {
                crate::app_lifecycle::show_main_window(&app);
            }
        }
    });
    let result = receiver.recv_timeout(Duration::from_secs(1)).unwrap_or_else(|_| Err("Windows did not respond while registering the shortcut".into()));
    if let Ok(mut current) = status_cell().lock() {
        *current = match result {
            Ok(()) => Status { available: true, label: LABEL, error: None },
            Err(error) => Status { available: false, label: LABEL, error: Some(format!("{LABEL} is unavailable: {error}")) },
        };
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
}
