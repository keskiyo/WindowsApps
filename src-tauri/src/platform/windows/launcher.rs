use crate::catalog::LaunchKind;
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use windows::core::PCWSTR;
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

pub fn launch(kind: LaunchKind, target: &str) -> Result<(), String> {
    if kind != LaunchKind::AppUserModelId && !Path::new(target).exists() {
        return Err(format!("File not found: {target}"));
    }
    let shell_target = match kind {
        LaunchKind::AppUserModelId => apps_folder_target(target),
        LaunchKind::Executable | LaunchKind::Shortcut => target.to_string(),
    };
    shell_execute(&shell_target)
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

pub fn shell_execute_elevated(program: &std::path::Path, args: &[&str]) -> Result<(), String> {
    let verb: Vec<u16> = OsStr::new("runas").encode_wide().chain(Some(0)).collect();
    let file: Vec<u16> = program.as_os_str().encode_wide().chain(Some(0)).collect();
    let params = args.join(" ");
    let params_wide: Vec<u16> = OsStr::new(&params).encode_wide().chain(Some(0)).collect();
    let result = unsafe {
        ShellExecuteW(
            None,
            PCWSTR(verb.as_ptr()),
            PCWSTR(file.as_ptr()),
            PCWSTR(params_wide.as_ptr()),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    validate_shell_result(result.0 as isize)
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
