use crate::catalog::UninstallTarget;
use serde::{Deserialize, Serialize};
use std::os::windows::process::CommandExt;
use std::path::Path;
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

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UninstallMechanism {
    RegisteredCommand,
    Msi,
    Msix,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UninstallTargetPreview {
    pub mechanism: UninstallMechanism,
    pub command: String,
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

pub fn preview(target: &UninstallTarget) -> UninstallTargetPreview {
    match target {
        UninstallTarget::Command {
            executable,
            arguments,
        } => {
            let mechanism = if is_msiexec(executable) {
                UninstallMechanism::Msi
            } else {
                UninstallMechanism::RegisteredCommand
            };
            let command = if arguments.trim().is_empty() {
                executable.clone()
            } else {
                format!("{executable} {arguments}")
            };
            UninstallTargetPreview { mechanism, command }
        }
        UninstallTarget::Msix { package_full_name } => {
            let command = if valid_package_name(package_full_name) {
                format!(
                    "powershell.exe -NoLogo -NoProfile -NonInteractive -Command Remove-AppxPackage -Package '{}'",
                    package_full_name
                )
            } else {
                "<invalid package identity>".to_string()
            };
            UninstallTargetPreview {
                mechanism: UninstallMechanism::Msix,
                command,
            }
        }
    }
}

pub fn execute(target: Option<UninstallTarget>) -> Result<(), String> {
    match decide(target) {
        UninstallDecision::Command {
            executable,
            arguments,
        } => {
            if !is_local_executable(&executable) {
                return Err("Uninstaller path is not a local executable".into());
            }
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

fn is_msiexec(executable: &str) -> bool {
    let name = Path::new(executable)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(executable);
    name.eq_ignore_ascii_case("msiexec") || name.eq_ignore_ascii_case("msiexec.exe")
}

/// Reject empty and UNC (`\\server\share`) uninstaller paths so a tampered registry
/// entry cannot point the uninstall action at a network-hosted binary.
fn is_local_executable(value: &str) -> bool {
    let trimmed = value.trim().trim_matches('"');
    !trimmed.is_empty() && !trimmed.starts_with(r"\\")
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

    #[test]
    fn rejects_unc_and_empty_uninstaller_paths() {
        assert!(is_local_executable(r"C:\Program Files\App\uninstall.exe"));
        assert!(is_local_executable("msiexec.exe"));
        assert!(!is_local_executable(r"\\attacker\share\payload.exe"));
        assert!(!is_local_executable("   "));
        assert!(!is_local_executable(r#""\\attacker\share\x.exe""#));
    }

    #[test]
    fn preview_does_not_inject_invalid_package_identity() {
        let preview = preview(&UninstallTarget::Msix {
            package_full_name: "evil'; Remove-Item C:\\".into(),
        });
        assert_eq!(preview.command, "<invalid package identity>");
        assert!(!preview.command.contains("Remove-Item"));
    }

    #[test]
    fn formats_uninstall_preview_for_msi_and_msix() {
        assert_eq!(
            preview(&UninstallTarget::Command {
                executable: "msiexec.exe".into(),
                arguments: "/x {PRODUCT}".into(),
            })
            .mechanism,
            UninstallMechanism::Msi
        );
        assert_eq!(
            preview(&UninstallTarget::Msix {
                package_full_name: "OpenAI.Codex_1.0_x64__abc".into(),
            })
            .mechanism,
            UninstallMechanism::Msix
        );
    }
}
