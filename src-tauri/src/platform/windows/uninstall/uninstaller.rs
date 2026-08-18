use super::validate::{is_msiexec, validate, Validated};
use crate::catalog::UninstallTarget;
use serde::{Deserialize, Serialize};
use std::os::windows::process::CommandExt;
use std::process::Command;

const CREATE_NO_WINDOW: u32 = 0x08000000;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum UninstallMechanism {
    RegisteredCommand,
    Msi,
    Msix,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UninstallTargetPreview {
    pub mechanism: UninstallMechanism,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum UninstallOutcome {
    Completed,
    Cancelled,
}

pub(crate) fn preview(target: &UninstallTarget) -> UninstallTargetPreview {
    let mechanism = match target {
        UninstallTarget::Command { executable, .. } if is_msiexec(executable) => {
            UninstallMechanism::Msi
        }
        UninstallTarget::Command { .. } => UninstallMechanism::RegisteredCommand,
        UninstallTarget::Msix { .. } => UninstallMechanism::Msix,
    };
    UninstallTargetPreview { mechanism }
}

pub(crate) fn execute(target: Option<UninstallTarget>) -> Result<UninstallOutcome, String> {
    let Some(target) = target else {
        return Err("Uninstall is unavailable for this application".into());
    };
    match validate(&target)? {
        Validated::Process { program, args } => {
            let status = Command::new(&program)
                .args(&args)
                .creation_flags(CREATE_NO_WINDOW)
                .status()
                .map_err(|error| format!("Could not start the uninstaller: {error}"))?;
            classify_exit(status.code(), status.success())
        }
        Validated::Msix { package_full_name } => super::msix::remove(&package_full_name),
    }
}

fn classify_exit(code: Option<i32>, success: bool) -> Result<UninstallOutcome, String> {
    if success || matches!(code, Some(1641 | 3010)) {
        return Ok(UninstallOutcome::Completed);
    }
    if matches!(code, Some(1602 | 2)) {
        return Ok(UninstallOutcome::Cancelled);
    }
    Err(format!(
        "The registered uninstaller exited with code {}",
        code.map_or_else(|| "unknown".into(), |value| value.to_string())
    ))
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;

    pub(in crate::platform::windows::uninstall) fn command(
        executable: &str,
        arguments: &str,
    ) -> UninstallTarget {
        UninstallTarget::Command {
            executable: executable.into(),
            arguments: arguments.into(),
        }
    }

    pub(in crate::platform::windows::uninstall) fn msix(package: &str) -> UninstallTarget {
        UninstallTarget::Msix {
            package_full_name: package.into(),
        }
    }

    // Closing the wizard is a decision, not a fault: reporting it as a failed uninstall told the
    // user something went wrong when nothing did.
    #[test]
    fn a_user_who_closed_the_wizard_is_not_a_failure() {
        assert_eq!(
            classify_exit(Some(0), true),
            Ok(UninstallOutcome::Completed)
        );
        assert_eq!(
            classify_exit(Some(3010), false),
            Ok(UninstallOutcome::Completed)
        );
        assert_eq!(
            classify_exit(Some(1641), false),
            Ok(UninstallOutcome::Completed)
        );
        assert_eq!(
            classify_exit(Some(1602), false),
            Ok(UninstallOutcome::Cancelled)
        );
        assert_eq!(
            classify_exit(Some(2), false),
            Ok(UninstallOutcome::Cancelled)
        );
        assert!(classify_exit(Some(1), false).is_err());
        assert!(classify_exit(None, false).is_err());
    }

    #[test]
    fn preview_exposes_only_the_removal_mechanism() {
        let exe = preview(&command(r"C:\App\uninstall.exe", "/S"));
        assert_eq!(exe.mechanism, UninstallMechanism::RegisteredCommand);
        assert_eq!(
            serde_json::to_value(exe).unwrap(),
            serde_json::json!({ "mechanism": "registered_command" })
        );

        assert_eq!(
            preview(&command(
                "msiexec.exe",
                "/x {2C4E5A1B-9F0D-4A7B-8C3E-1122334455AA}"
            ))
            .mechanism,
            UninstallMechanism::Msi
        );
        assert_eq!(
            preview(&msix("OpenAI.Codex_1.0_x64__abc")).mechanism,
            UninstallMechanism::Msix
        );
    }
}
