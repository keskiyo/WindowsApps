use std::collections::HashSet;
use windows::core::BOOL;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumChildWindows, EnumWindows, GetClassNameW, GetWindowThreadProcessId, IsWindowVisible,
    PostMessageW, WM_CLOSE,
};

const APPLICATION_FRAME_CLASS: &str = "ApplicationFrameWindow";
const CORE_WINDOW_CLASS: &str = "Windows.UI.Core.CoreWindow";

fn class_name(window: HWND) -> String {
    let mut buffer = [0u16; 256];
    // SAFETY: `buffer` is a live, fully initialized array owned by this frame, and the length
    // passed is its element count, so the call cannot write past it. It returns the number of
    // characters written (0 on failure), which is what the slice is cut to.
    let written = unsafe { GetClassNameW(window, &mut buffer) } as usize;
    buffer
        .get(..written)
        .map(String::from_utf16_lossy)
        .unwrap_or_default()
}

fn process_of(window: HWND) -> u32 {
    let mut process = 0u32;
    // SAFETY: `window` comes from an enumeration callback and is valid for its duration; the
    // function only reads it. `process` is a live local that outlives the call.
    let _ = unsafe { GetWindowThreadProcessId(window, Some(&mut process)) };
    process
}

unsafe extern "system" fn collect_core_window(window: HWND, lparam: LPARAM) -> BOOL {
    // SAFETY: `lparam` is the `&mut Vec<u32>` that `core_window_processes` passed to
    // `EnumChildWindows`, which borrows it for the whole synchronous enumeration; the callback
    // runs on the calling thread only, so no other reference to it is live here.
    let Some(processes) = (unsafe { (lparam.0 as *mut Vec<u32>).as_mut() }) else {
        return BOOL(0);
    };
    if class_name(window) == CORE_WINDOW_CLASS {
        processes.push(process_of(window));
    }
    BOOL(1)
}

fn core_window_processes(frame: HWND) -> Vec<u32> {
    let mut processes: Vec<u32> = Vec::new();
    // SAFETY: `EnumChildWindows` is synchronous — it returns only after the last callback — so the
    // pointer handed over as `lparam` refers to `processes`, which is alive for the whole call and
    // borrowed exclusively until it returns. A failure return (no children) leaves it empty.
    let _ = unsafe {
        EnumChildWindows(
            Some(frame),
            Some(collect_core_window),
            LPARAM(&mut processes as *mut Vec<u32> as isize),
        )
    };
    processes
}

struct Enumeration {
    targets: HashSet<u32>,
    windows: Vec<HWND>,
}

impl Enumeration {
    fn owns_target(&self, window: HWND) -> bool {
        if self.targets.contains(&process_of(window)) {
            return true;
        }
        class_name(window) == APPLICATION_FRAME_CLASS
            && core_window_processes(window)
                .iter()
                .any(|process| self.targets.contains(process))
    }
}

unsafe extern "system" fn collect_window(window: HWND, lparam: LPARAM) -> BOOL {
    // SAFETY: `lparam` is the `&mut Enumeration` that `windows_of` passed to `EnumWindows`, which
    // borrows it for the whole synchronous enumeration; `EnumWindows` calls back on the calling
    // thread only, so no other reference to it is live here.
    let Some(state) = (unsafe { (lparam.0 as *mut Enumeration).as_mut() }) else {
        return BOOL(0);
    };
    // SAFETY: `window` is supplied by `EnumWindows` and valid for the callback; the call only
    // reads it.
    if unsafe { IsWindowVisible(window) }.as_bool() && state.owns_target(window) {
        state.windows.push(window);
    }
    BOOL(1)
}

pub(super) fn windows_of(targets: HashSet<u32>) -> Vec<HWND> {
    if targets.is_empty() {
        return Vec::new();
    }
    let mut state = Enumeration {
        targets,
        windows: Vec::new(),
    };
    // SAFETY: `EnumWindows` is synchronous — it returns only after the last callback — so the
    // pointer handed over as `lparam` refers to `state`, which is alive for the whole call and
    // borrowed exclusively until it returns. A failure return (no windows) leaves `state`
    // untouched and is deliberately ignored.
    let _ = unsafe {
        EnumWindows(
            Some(collect_window),
            LPARAM(&mut state as *mut Enumeration as isize),
        )
    };
    state.windows
}

pub(super) fn ask_to_close(window: HWND) {
    // SAFETY: `PostMessageW` copies the message into the target thread's queue and returns
    // immediately; it borrows nothing and the payload is two zero values. A window that closed
    // between enumeration and this call makes the call fail, which is the expected race and is
    // ignored.
    let _ = unsafe { PostMessageW(Some(window), WM_CLOSE, WPARAM(0), LPARAM(0)) };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn looks_for_no_window_without_a_target_process() {
        assert!(windows_of(HashSet::new()).is_empty());
    }

    #[test]
    fn collects_nothing_for_an_unknown_process() {
        assert!(windows_of(HashSet::from([u32::MAX])).is_empty());
    }
}
