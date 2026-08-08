use std::collections::HashSet;
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows::Win32::System::Threading::{
    GetCurrentProcessId, OpenProcess, QueryFullProcessImageNameW, TerminateProcess,
    PROCESS_ACCESS_RIGHTS, PROCESS_NAME_FORMAT, PROCESS_QUERY_LIMITED_INFORMATION,
    PROCESS_TERMINATE,
};

struct OwnedHandle(HANDLE);

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        // SAFETY: the only two constructors are `open_process` and the snapshot below, and both
        // return `None`/return early unless the Win32 call succeeded — so the handle is one this
        // process owns and has not closed. The field is private and never copied out, `Drop` runs
        // once, and no other code path closes it, so this is the single `CloseHandle` for it.
        let _ = unsafe { CloseHandle(self.0) };
    }
}

fn open_process(access: PROCESS_ACCESS_RIGHTS, pid: u32) -> Option<OwnedHandle> {
    // SAFETY: `OpenProcess` takes only plain values — no pointers, no buffers — and reports
    // failure (access denied, exited process, protected process) through the `Result`, which is
    // mapped to `None` rather than unwrapped. Ownership of a successful handle transfers to us and
    // is released by `OwnedHandle::drop`.
    unsafe { OpenProcess(access, false, pid) }
        .ok()
        .map(OwnedHandle)
}

pub(super) fn image_path_of(pid: u32) -> Option<String> {
    let handle = open_process(PROCESS_QUERY_LIMITED_INFORMATION, pid)?;
    let mut buffer = [0u16; 32768];
    let mut length = buffer.len() as u32;
    // SAFETY: `buffer` is a live, fully initialized array owned by this frame, and `length` is
    // initialized to its element count — the contract `QueryFullProcessImageNameW` requires — and
    // is rewritten by the call with the number of characters actually written, which is what the
    // slice below is cut to. The handle is borrowed from a live `OwnedHandle`, so it stays open
    // for the whole call. Failure (the process exited between enumeration and this call) is
    // reported through the `Result` and turned into `None`.
    let written = unsafe {
        QueryFullProcessImageNameW(
            handle.0,
            PROCESS_NAME_FORMAT(0),
            windows::core::PWSTR(buffer.as_mut_ptr()),
            &mut length,
        )
    }
    .ok()
    .map(|()| length as usize)?;
    buffer.get(..written).map(String::from_utf16_lossy)
}

fn entry_file_name(entry: &PROCESSENTRY32W) -> String {
    let name = entry
        .szExeFile
        .split(|character| *character == 0)
        .next()
        .unwrap_or_default();
    String::from_utf16_lossy(name).to_lowercase()
}

pub(super) fn running_images(file_names: &HashSet<String>) -> Vec<(u32, String)> {
    if file_names.is_empty() {
        return Vec::new();
    }
    // SAFETY: `GetCurrentProcessId` takes no arguments and cannot fail.
    let own_pid = unsafe { GetCurrentProcessId() };
    // SAFETY: `CreateToolhelp32Snapshot` takes only plain values and reports failure through the
    // `Result`. A successful handle is owned by us and released by `OwnedHandle::drop`, including
    // on every early return below.
    let Ok(snapshot) = (unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) }) else {
        return Vec::new();
    };
    let snapshot = OwnedHandle(snapshot);
    let mut entry = PROCESSENTRY32W {
        dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
        ..Default::default()
    };
    // SAFETY: `entry` is a live, fully initialized local whose `dwSize` states its own size, and
    // the snapshot handle is borrowed from a live `OwnedHandle`. Both calls only write into
    // `entry`; the end of the list is reported through the `Result` and ends the loop.
    if unsafe { Process32FirstW(snapshot.0, &mut entry) }.is_err() {
        return Vec::new();
    }
    let mut processes = Vec::new();
    loop {
        let pid = entry.th32ProcessID;
        if pid != 0 && pid != own_pid && file_names.contains(&entry_file_name(&entry)) {
            if let Some(image) = image_path_of(pid) {
                processes.push((pid, image));
            }
        }
        // SAFETY: same invariants as `Process32FirstW` above; `entry` stays live for the loop.
        if unsafe { Process32NextW(snapshot.0, &mut entry) }.is_err() {
            break;
        }
    }
    processes
}

pub(super) fn terminate(pid: u32) -> bool {
    let Some(handle) = open_process(PROCESS_TERMINATE, pid) else {
        return false;
    };
    // SAFETY: the handle is borrowed from a live `OwnedHandle` opened with `PROCESS_TERMINATE`, so
    // it stays open for the call. A process that already exited makes this fail, which is reported
    // through the `Result`.
    unsafe { TerminateProcess(handle.0, 1) }.is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_executable_name_up_to_its_terminator() {
        let mut entry = PROCESSENTRY32W::default();
        for (slot, character) in entry.szExeFile.iter_mut().zip("Editor.EXE".encode_utf16()) {
            *slot = character;
        }

        assert_eq!(entry_file_name(&entry), "editor.exe");
    }

    #[test]
    fn never_lists_the_current_process() {
        // SAFETY: `GetCurrentProcessId` takes no arguments and cannot fail.
        let own_pid = unsafe { GetCurrentProcessId() };
        let own_name = std::env::current_exe()
            .ok()
            .and_then(|path| {
                path.file_name()
                    .map(|name| name.to_string_lossy().to_lowercase())
            })
            .unwrap_or_default();

        let running = running_images(&HashSet::from([own_name]));

        assert!(running.iter().all(|(pid, _)| *pid != own_pid));
    }

    #[test]
    fn lists_nothing_without_a_name_to_look_for() {
        assert!(running_images(&HashSet::new()).is_empty());
    }
}
