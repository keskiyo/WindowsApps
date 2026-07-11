use crate::catalog::LaunchKind;
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use windows::core::PCWSTR;
use windows::Win32::UI::Shell::{
    ShellExecuteExW, ShellExecuteW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW,
};
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

/// Launches an app and, when the shell hands back a process handle (regular exe/shortcut
/// launches), returns its raw value so the caller can wait for the window to be ready.
/// Store/UWP and shell hand-offs return `Ok(None)` — there is no handle to wait on.
pub fn launch(kind: LaunchKind, target: &str) -> Result<Option<isize>, String> {
    if kind != LaunchKind::AppUserModelId && !Path::new(target).exists() {
        return Err(format!("File not found: {target}"));
    }
    let shell_target = match kind {
        LaunchKind::AppUserModelId => apps_folder_target(target),
        LaunchKind::Executable | LaunchKind::Shortcut => target.to_string(),
    };
    shell_execute_with_handle(&shell_target)
}

fn shell_execute_with_handle(target: &str) -> Result<Option<isize>, String> {
    let operation: Vec<u16> = OsStr::new("open").encode_wide().chain(Some(0)).collect();
    let file: Vec<u16> = OsStr::new(target).encode_wide().chain(Some(0)).collect();
    let mut info = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOCLOSEPROCESS,
        lpVerb: PCWSTR(operation.as_ptr()),
        lpFile: PCWSTR(file.as_ptr()),
        nShow: SW_SHOWNORMAL.0,
        ..Default::default()
    };
    unsafe { ShellExecuteExW(&mut info) }.map_err(|_| {
        format!(
            "Windows Shell could not launch the application (code {})",
            info.hInstApp.0 as isize
        )
    })?;
    if info.hProcess.is_invalid() {
        Ok(None)
    } else {
        Ok(Some(info.hProcess.0 as isize))
    }
}

pub fn shell_execute(target: &str) -> Result<(), String> {
    let operation: Vec<u16> = OsStr::new("open").encode_wide().chain(Some(0)).collect();
    let file: Vec<u16> = OsStr::new(target).encode_wide().chain(Some(0)).collect();
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
    validate_shell_result(result.0 as isize)
}

fn apps_folder_target(app_id: &str) -> String {
    format!(r"shell:AppsFolder\{app_id}")
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
    fn builds_apps_folder_target() {
        assert_eq!(
            apps_folder_target("OpenAI.Codex_abc!App"),
            r"shell:AppsFolder\OpenAI.Codex_abc!App"
        );
    }
}
