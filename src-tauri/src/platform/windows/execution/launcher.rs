use crate::catalog::LaunchKind;
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::Threading::WaitForInputIdle;
use windows::Win32::UI::Shell::{
    ShellExecuteExW, ShellExecuteW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW,
};
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

pub(crate) struct OwnedProcessHandle(isize);

impl Drop for OwnedProcessHandle {
    fn drop(&mut self) {
        // SAFETY: `CloseHandle` requires a handle this process owns and has not already closed.
        // The only constructor is `shell_execute_with_handle`, which stores the process handle
        // `ShellExecuteExW` returned under `SEE_MASK_NOCLOSEPROCESS` — ownership transferred to
        // us — and only after `is_invalid()` ruled out the null case. The field is private and
        // never copied out, so this is the single close, and `Drop` runs once.
        let _ = unsafe { CloseHandle(HANDLE(self.0 as *mut core::ffi::c_void)) };
    }
}

pub(crate) fn wait_for_input_idle(handle: &OwnedProcessHandle, timeout_ms: u32) {
    let handle = HANDLE(handle.0 as *mut core::ffi::c_void);
    // SAFETY: the handle is borrowed from a live `OwnedProcessHandle`, so it is still open for
    // the whole call — `Drop` cannot run while the borrow is held. `WaitForInputIdle` only reads
    // the handle; a process without a message queue or an elapsed timeout is reported through the
    // return value, which the caller deliberately ignores in favour of its own ceiling timer.
    unsafe { WaitForInputIdle(handle, timeout_ms) };
}

pub(crate) fn launch(
    kind: LaunchKind,
    target: &str,
    arguments: Option<&str>,
) -> Result<Option<OwnedProcessHandle>, String> {
    if kind != LaunchKind::AppUserModelId && !is_launch_uri(target) && !Path::new(target).exists() {
        return Err(format!("File not found: {target}"));
    }
    let shell_target = match kind {
        LaunchKind::AppUserModelId => apps_folder_target(target),
        LaunchKind::Executable | LaunchKind::Shortcut => target.to_string(),
    };
    shell_execute_with_handle(&shell_target, parameters_for(kind, target, arguments))
}

fn shell_execute_with_handle(
    target: &str,
    parameters: Option<&str>,
) -> Result<Option<OwnedProcessHandle>, String> {
    let operation: Vec<u16> = OsStr::new("open").encode_wide().chain(Some(0)).collect();
    let file: Vec<u16> = OsStr::new(target).encode_wide().chain(Some(0)).collect();
    let parameters = parameters.map(|value| {
        OsStr::new(value)
            .encode_wide()
            .chain(Some(0))
            .collect::<Vec<_>>()
    });
    let mut info = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOCLOSEPROCESS,
        lpVerb: PCWSTR(operation.as_ptr()),
        lpFile: PCWSTR(file.as_ptr()),
        lpParameters: parameters
            .as_ref()
            .map_or(PCWSTR::null(), |value| PCWSTR(value.as_ptr())),
        nShow: SW_SHOWNORMAL.0,
        ..Default::default()
    };
    // SAFETY: `info` is fully initialized (`cbSize` set to its own size, remaining fields
    // defaulted), and its `lpVerb`/`lpFile`/`lpParameters` point at `operation`, `file`, and the
    // optional NUL-terminated UTF-16 parameter buffer owned by this frame that outlive the call.
    // `ShellExecuteExW`
    // writes back into `info` through the exclusive `&mut`, and `SEE_MASK_NOCLOSEPROCESS` makes
    // any returned `hProcess` ours to close — which `OwnedProcessHandle` then does exactly once.
    unsafe { ShellExecuteExW(&mut info) }.map_err(|_| {
        format!(
            "Windows Shell could not launch the application (code {})",
            info.hInstApp.0 as isize
        )
    })?;
    if info.hProcess.is_invalid() {
        Ok(None)
    } else {
        Ok(Some(OwnedProcessHandle(info.hProcess.0 as isize)))
    }
}

fn parameters_for<'a>(
    kind: LaunchKind,
    target: &str,
    arguments: Option<&'a str>,
) -> Option<&'a str> {
    (kind == LaunchKind::Executable && !is_launch_uri(target))
        .then_some(arguments?.trim())
        .filter(|value| !value.is_empty())
}

pub(crate) fn shell_execute(target: &str) -> Result<(), String> {
    let operation: Vec<u16> = OsStr::new("open").encode_wide().chain(Some(0)).collect();
    let file: Vec<u16> = OsStr::new(target).encode_wide().chain(Some(0)).collect();
    // SAFETY: both `PCWSTR`s point at NUL-terminated UTF-16 buffers owned by this frame that
    // outlive the call; the remaining pointer arguments are explicitly null, which
    // `ShellExecuteW` documents as "no parameters" and "no working directory". No handle is
    // returned to own — this variant does not set `SEE_MASK_NOCLOSEPROCESS` — so the result is
    // only an error code, checked by `validate_shell_result`.
    let result = unsafe {
        ShellExecuteW(
            None,
            PCWSTR(operation.as_ptr()),
            PCWSTR(file.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    let code = result.0 as isize;
    validate_shell_result(code)
}

fn apps_folder_target(app_id: &str) -> String {
    format!(r"shell:AppsFolder\{app_id}")
}

const LAUNCH_URI_SCHEMES: &[&str] = &["steam:"];

fn is_launch_uri(target: &str) -> bool {
    let target = target.trim().to_ascii_lowercase();
    LAUNCH_URI_SCHEMES
        .iter()
        .any(|scheme| target.starts_with(scheme))
}

fn validate_shell_result(result: isize) -> Result<(), String> {
    if result > 32 {
        Ok(())
    } else {
        Err(format!(
            "Windows Shell could not launch the application (code {result})"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_shell_execute_success_values() {
        assert!(validate_shell_result(33).is_ok());
    }

    #[test]
    fn rejects_shell_execute_error_values() {
        assert!(validate_shell_result(31).is_err());
    }

    #[test]
    fn steam_targets_are_recognized_as_protocol_uris() {
        assert!(is_launch_uri("steam://rungameid/2183900"));
        assert!(is_launch_uri("  STEAM://rungameid/1  "));
    }

    #[test]
    fn ordinary_targets_are_not_treated_as_protocol_uris() {
        for target in [
            r"C:\Games\game.exe",
            r"C:\Menu\Game.lnk",
            r"\\server\share\game.exe",
            "Microsoft.WindowsCamera_8wekyb3d8bbwe!App",
            "https://example.invalid",
            "",
        ] {
            assert!(!is_launch_uri(target), "{target}");
        }
    }

    #[test]
    fn builds_apps_folder_target() {
        assert_eq!(
            apps_folder_target("OpenAI.Codex_abc!App"),
            r"shell:AppsFolder\OpenAI.Codex_abc!App"
        );
    }

    #[test]
    fn keeps_arguments_for_executables() {
        assert_eq!(
            parameters_for(
                LaunchKind::Executable,
                r"C:\Editor.exe",
                Some(" --safe-mode ")
            ),
            Some("--safe-mode")
        );
    }

    #[test]
    fn never_adds_parameters_to_shortcuts_aumids_or_uris() {
        assert_eq!(
            parameters_for(
                LaunchKind::Shortcut,
                r"C:\Menu\Editor.lnk",
                Some("--safe-mode")
            ),
            None
        );
        assert_eq!(
            parameters_for(
                LaunchKind::AppUserModelId,
                "Contoso.App!App",
                Some("--safe-mode")
            ),
            None
        );
        assert_eq!(
            parameters_for(
                LaunchKind::Executable,
                "steam://rungameid/1",
                Some("--safe-mode")
            ),
            None
        );
    }
}
