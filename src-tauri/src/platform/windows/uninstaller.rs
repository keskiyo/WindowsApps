use crate::catalog::UninstallTarget;
use std::os::windows::process::CommandExt;
use std::process::Command;

const CREATE_NO_WINDOW: u32 = 0x08000000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UninstallDecision {
    Command {
        executable: String,
        arguments: String,
    },
    Msix {
        package_full_name: String,
    },
    Unavailable,
}

pub fn decide(target: Option<UninstallTarget>) -> UninstallDecision {
    match target {
        Some(UninstallTarget::Command {
            executable,
            arguments,
        }) => UninstallDecision::Command {
            executable,
            arguments,
        },
        Some(UninstallTarget::Msix { package_full_name }) => {
            UninstallDecision::Msix { package_full_name }
        }
        None => UninstallDecision::Unavailable,
    }
}

pub fn execute(target: Option<UninstallTarget>) -> Result<(), String> {
    match decide(target) {
        UninstallDecision::Command {
            executable,
            arguments,
        } => {
            let mut command = Command::new(executable);
            if !arguments.is_empty() {
                command.raw_arg(arguments);
            }
            let status = command
                .creation_flags(CREATE_NO_WINDOW)
                .status()
                .map_err(|error| format!("Could not start the registered uninstaller: {error}"))?;
            ensure_success(status.code(), status.success())
        }
        UninstallDecision::Msix { package_full_name } => {
            if !valid_package_name(&package_full_name) {
                return Err("The package identity is invalid".into());
            }
            let script = format!("Remove-AppxPackage -Package '{}'", package_full_name);
            let status = Command::new("powershell.exe")
                .args([
                    "-NoLogo",
                    "-NoProfile",
                    "-NonInteractive",
                    "-Command",
                    &script,
                ])
                .creation_flags(CREATE_NO_WINDOW)
                .status()
                .map_err(|error| format!("Could not start package removal: {error}"))?;
            ensure_success(status.code(), status.success())
        }
        UninstallDecision::Unavailable => {
            Err("Uninstall is unavailable for this application".into())
        }
    }
}

fn ensure_success(code: Option<i32>, success: bool) -> Result<(), String> {
    if success || matches!(code, Some(1641 | 3010)) {
        Ok(())
    } else {
        Err(format!(
            "The registered uninstaller exited with code {}",
            code.map_or_else(|| "unknown".into(), |value| value.to_string())
        ))
    }
}

fn valid_package_name(value: &str) -> bool {
    !value.is_empty()
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-' | '~')
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chooses_registered_uninstall_command() {
        assert!(matches!(
            decide(Some(UninstallTarget::Command {
                executable: "uninstall.exe".into(),
                arguments: "/remove".into()
            })),
            UninstallDecision::Command { .. }
        ));
    }

    #[test]
    fn missing_target_is_unavailable() {
        assert_eq!(decide(None), UninstallDecision::Unavailable);
    }

    #[test]
    fn validates_msix_package_names() {
        assert!(valid_package_name("OpenAI.Codex_1.2.3.0_x64__abc"));
        assert!(!valid_package_name("package'; Remove-Item C:\\"));
    }
}
