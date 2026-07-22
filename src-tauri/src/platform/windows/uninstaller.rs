use crate::catalog::UninstallTarget;
use serde::{Deserialize, Serialize};
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

const CREATE_NO_WINDOW: u32 = 0x08000000;

/// Living-off-the-land binaries whose behaviour is fully controlled by their arguments. A
/// tampered `UninstallString` such as `powershell.exe -Command <payload>` or `cmd.exe /C
/// <payload>` would otherwise run attacker code, so these are never accepted as uninstall
/// executables (msiexec is handled separately with a strict argument allowlist).
const INTERPRETER_EXECUTABLES: &[&str] = &[
    "cmd.exe",
    "powershell.exe",
    "pwsh.exe",
    "wscript.exe",
    "cscript.exe",
    "rundll32.exe",
    "regsvr32.exe",
    "mshta.exe",
    "conhost.exe",
    "wmic.exe",
    "forfiles.exe",
    "bash.exe",
    "sh.exe",
    "wsl.exe",
    "python.exe",
    "pythonw.exe",
    "py.exe",
    "node.exe",
    "java.exe",
    "javaw.exe",
    "msbuild.exe",
    "installutil.exe",
    "regsvcs.exe",
    "regasm.exe",
    "mavinject.exe",
];

/// Extra MSI switches accepted alongside the uninstall verb and product code. Anything else
/// (install verbs, properties like `EVIL=1`, transforms, log paths) is rejected.
const ALLOWED_MSI_SWITCHES: &[&str] = &["/quiet", "/qn", "/qb", "/qb-", "/passive", "/norestart"];

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

/// A validated, normalized uninstall action. Both `preview` and `execute` derive their
/// behaviour from this single value, so what the user sees and what runs cannot diverge, and
/// nothing here can be influenced by the frontend (only an application id crosses IPC).
#[derive(Clone, Debug, Eq, PartialEq)]
enum Validated {
    /// A concrete program plus already-split argument vector (no shell, no raw command line).
    Process {
        program: PathBuf,
        args: Vec<String>,
    },
    Msix {
        package_full_name: String,
    },
}

/// Validate an untrusted uninstall target from the catalog. Returns a normalized action or a
/// generic, path-free rejection reason.
fn validate(target: &UninstallTarget) -> Result<Validated, String> {
    match target {
        UninstallTarget::Command {
            executable,
            arguments,
        } => {
            if is_msiexec(executable) {
                validate_msi(arguments)
            } else {
                validate_process(executable, arguments)
            }
        }
        UninstallTarget::Msix { package_full_name } => {
            if valid_package_name(package_full_name) {
                Ok(Validated::Msix {
                    package_full_name: package_full_name.clone(),
                })
            } else {
                Err("The package identity is invalid".into())
            }
        }
    }
}

fn validate_process(executable: &str, arguments: &str) -> Result<Validated, String> {
    let expanded = expand_env(executable.trim());
    let candidate = expanded.trim().trim_matches('"').trim();
    if candidate.is_empty() {
        return Err("The uninstaller path is empty".into());
    }
    // Reject UNC (`\\server\share`) and device (`\\?\`, `\\.\`) paths, in both slash forms.
    if candidate.starts_with(r"\\") || candidate.starts_with("//") {
        return Err("Network or device uninstaller paths are not allowed".into());
    }
    let path = Path::new(candidate);
    let extension_is_exe = path
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case("exe"));
    if !extension_is_exe {
        return Err("The uninstaller must be an .exe".into());
    }
    if !path.is_absolute() {
        return Err("The uninstaller path must be absolute".into());
    }
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if INTERPRETER_EXECUTABLES.contains(&file_name.as_str()) {
        return Err("Script interpreters are not allowed as uninstallers".into());
    }
    let args = parse_arguments(arguments)?;
    Ok(Validated::Process {
        program: path.to_path_buf(),
        args,
    })
}

/// Validate `msiexec` uninstall arguments: exactly one product code (GUID) with the uninstall
/// verb and only allowlisted quiet/restart switches. The canonical System32 `msiexec.exe` is
/// used so a poisoned registry path cannot redirect execution.
fn validate_msi(arguments: &str) -> Result<Validated, String> {
    let tokens = parse_arguments(arguments)?;
    let mut product_code: Option<String> = None;
    let mut has_uninstall = false;
    let mut switches: Vec<String> = Vec::new();
    for token in &tokens {
        let lower = token.to_ascii_lowercase();
        if lower == "/x" || lower == "/uninstall" {
            has_uninstall = true;
            continue;
        }
        if is_product_code(token) {
            if product_code.replace(token.to_ascii_uppercase()).is_some() {
                return Err("Multiple MSI product codes".into());
            }
            continue;
        }
        // Glued form `MsiExec.exe /X{GUID}`.
        if lower.starts_with("/x") && is_product_code(&token[2..]) {
            if product_code
                .replace(token[2..].to_ascii_uppercase())
                .is_some()
            {
                return Err("Multiple MSI product codes".into());
            }
            has_uninstall = true;
            continue;
        }
        if ALLOWED_MSI_SWITCHES.contains(&lower.as_str()) {
            switches.push(lower);
            continue;
        }
        return Err("Unsupported MSI uninstall argument".into());
    }
    let Some(code) = product_code else {
        return Err("The MSI product code is missing or invalid".into());
    };
    if !has_uninstall {
        return Err("The MSI uninstall switch is required".into());
    }
    let mut args = vec!["/x".to_string(), code];
    args.extend(switches);
    Ok(Validated::Process {
        program: system_msiexec(),
        args,
    })
}

pub fn preview(target: &UninstallTarget) -> UninstallTargetPreview {
    let mechanism = match target {
        UninstallTarget::Command { executable, .. } if is_msiexec(executable) => {
            UninstallMechanism::Msi
        }
        UninstallTarget::Command { .. } => UninstallMechanism::RegisteredCommand,
        UninstallTarget::Msix { .. } => UninstallMechanism::Msix,
    };
    // Show the validated, normalized command — never the raw registry string. Blocked targets
    // show a safe message so a malicious command is not surfaced (or later stored) at all.
    let command = match validate(target) {
        Ok(Validated::Process { program, args }) => {
            if args.is_empty() {
                format!("\"{}\"", program.display())
            } else {
                format!("\"{}\" {}", program.display(), args.join(" "))
            }
        }
        Ok(Validated::Msix { package_full_name }) => format!(
            "powershell.exe -NoLogo -NoProfile -NonInteractive -Command Remove-AppxPackage -Package '{package_full_name}'"
        ),
        Err(_) => "This uninstaller was blocked for safety.".to_string(),
    };
    UninstallTargetPreview { mechanism, command }
}

pub fn execute(target: Option<UninstallTarget>) -> Result<(), String> {
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
            ensure_success(status.code(), status.success())
        }
        Validated::Msix { package_full_name } => {
            // Identity is strictly `[A-Za-z0-9._~-]` (no quotes/spaces/metacharacters), so it
            // cannot break out of the single-quoted PowerShell string.
            let script = format!("Remove-AppxPackage -Package '{package_full_name}'");
            let status = Command::new(system_powershell())
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
    }
}

/// Quote-aware argument splitter. Honors double quotes (with `""` as a literal quote inside a
/// quoted run) and rejects an unterminated quote instead of guessing. Empty input yields no
/// arguments. Does not invoke a shell.
fn parse_arguments(raw: &str) -> Result<Vec<String>, String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut has_token = false;
    let mut chars = raw.chars().peekable();
    while let Some(character) = chars.next() {
        match character {
            '"' => {
                has_token = true;
                if in_quotes {
                    if chars.peek() == Some(&'"') {
                        current.push('"');
                        chars.next();
                    } else {
                        in_quotes = false;
                    }
                } else {
                    in_quotes = true;
                }
            }
            character if character.is_whitespace() && !in_quotes => {
                if has_token {
                    args.push(std::mem::take(&mut current));
                    has_token = false;
                }
            }
            character => {
                has_token = true;
                current.push(character);
            }
        }
    }
    if in_quotes {
        return Err("Unterminated quote in uninstall arguments".into());
    }
    if has_token {
        args.push(current);
    }
    Ok(args)
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
    let candidate = expand_env(executable.trim());
    let name = Path::new(candidate.trim().trim_matches('"').trim())
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(executable);
    name.eq_ignore_ascii_case("msiexec") || name.eq_ignore_ascii_case("msiexec.exe")
}

fn is_product_code(value: &str) -> bool {
    let Some(inner) = value
        .strip_prefix('{')
        .and_then(|rest| rest.strip_suffix('}'))
    else {
        return false;
    };
    let segments: [&str; 5] = match inner.split('-').collect::<Vec<_>>().try_into() {
        Ok(segments) => segments,
        Err(_) => return false,
    };
    let lengths = [8usize, 4, 4, 4, 12];
    segments.iter().zip(lengths).all(|(segment, length)| {
        segment.len() == length
            && segment
                .chars()
                .all(|character| character.is_ascii_hexdigit())
    })
}

fn valid_package_name(value: &str) -> bool {
    !value.is_empty()
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-' | '~')
        })
}

fn system_msiexec() -> PathBuf {
    system_root().join("System32").join("msiexec.exe")
}

fn system_powershell() -> PathBuf {
    system_root()
        .join("System32")
        .join("WindowsPowerShell")
        .join("v1.0")
        .join("powershell.exe")
}

fn system_root() -> PathBuf {
    std::env::var("SystemRoot")
        .or_else(|_| std::env::var("windir"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(r"C:\Windows"))
}

/// Expand `%VAR%` environment references (registry `REG_EXPAND_SZ` values arrive unexpanded).
/// Unknown variables are left verbatim so validation still rejects them as non-absolute.
fn expand_env(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    let mut rest = value;
    while let Some(start) = rest.find('%') {
        result.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        let Some(end) = after.find('%') else {
            result.push_str(&rest[start..]);
            return result;
        };
        let name = &after[..end];
        match std::env::var(name) {
            Ok(replacement) => result.push_str(&replacement),
            Err(_) => {
                result.push('%');
                result.push_str(name);
                result.push('%');
            }
        }
        rest = &after[end + 1..];
    }
    result.push_str(rest);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn command(executable: &str, arguments: &str) -> UninstallTarget {
        UninstallTarget::Command {
            executable: executable.into(),
            arguments: arguments.into(),
        }
    }

    fn msix(package: &str) -> UninstallTarget {
        UninstallTarget::Msix {
            package_full_name: package.into(),
        }
    }

    // ---- positive cases -------------------------------------------------

    #[test]
    fn accepts_plain_executable_without_arguments() {
        let validated = validate(&command(r"C:\Program Files\App\uninstall.exe", "")).unwrap();
        assert_eq!(
            validated,
            Validated::Process {
                program: PathBuf::from(r"C:\Program Files\App\uninstall.exe"),
                args: vec![]
            }
        );
    }

    #[test]
    fn accepts_quoted_path_with_spaces_and_multiple_arguments() {
        let validated = validate(&command(
            r"C:\Program Files\App\unins000.exe",
            "/SILENT /NORESTART",
        ))
        .unwrap();
        assert_eq!(
            validated,
            Validated::Process {
                program: PathBuf::from(r"C:\Program Files\App\unins000.exe"),
                args: vec!["/SILENT".into(), "/NORESTART".into()]
            }
        );
    }

    #[test]
    fn accepts_uppercase_exe_extension() {
        assert!(validate(&command(r"C:\Apps\Uninstall.EXE", "/S")).is_ok());
    }

    #[test]
    fn accepts_valid_msi_uninstall() {
        let validated = validate(&command(
            "msiexec.exe",
            "/x {2C4E5A1B-9F0D-4A7B-8C3E-1122334455AA} /qn /norestart",
        ))
        .unwrap();
        let Validated::Process { program, args } = validated else {
            panic!("expected process");
        };
        assert!(program.ends_with("msiexec.exe"));
        assert_eq!(args[0], "/x");
        assert_eq!(args[1], "{2C4E5A1B-9F0D-4A7B-8C3E-1122334455AA}");
        assert!(args.contains(&"/qn".to_string()));
    }

    #[test]
    fn accepts_glued_msi_product_code() {
        let validated = validate(&command(
            r"C:\Windows\System32\msiexec.exe",
            "/X{2C4E5A1B-9F0D-4A7B-8C3E-1122334455AA}",
        ))
        .unwrap();
        let Validated::Process { args, .. } = validated else {
            panic!("expected process");
        };
        assert_eq!(args, vec!["/x", "{2C4E5A1B-9F0D-4A7B-8C3E-1122334455AA}"]);
    }

    #[test]
    fn accepts_valid_msix_identity() {
        assert!(validate(&msix("OpenAI.Codex_1.2.3.0_x64__abc")).is_ok());
    }

    // ---- negative cases -------------------------------------------------

    #[test]
    fn rejects_empty_relative_unc_and_device_paths() {
        assert!(validate(&command("   ", "")).is_err());
        assert!(validate(&command(r"relative\uninstall.exe", "")).is_err());
        assert!(validate(&command(r"\\attacker\share\payload.exe", "")).is_err());
        assert!(validate(&command(r"\\?\C:\payload.exe", "")).is_err());
        assert!(validate(&command(r"\\.\PhysicalDrive0", "")).is_err());
    }

    #[test]
    fn rejects_script_and_shortcut_targets() {
        for path in [
            r"C:\x\a.bat",
            r"C:\x\a.cmd",
            r"C:\x\a.com",
            r"C:\x\a.ps1",
            r"C:\x\a.vbs",
            r"C:\x\a.js",
            r"C:\x\a.jse",
            r"C:\x\a.wsf",
            r"C:\x\a.hta",
            r"C:\x\a.url",
            r"C:\x\a.lnk",
            r"C:\x\a.scr",
        ] {
            assert!(
                validate(&command(path, "")).is_err(),
                "{path} must be rejected"
            );
        }
    }

    #[test]
    fn rejects_interpreter_executables() {
        assert!(validate(&command(r"C:\Windows\System32\cmd.exe", "/C calc.exe")).is_err());
        assert!(validate(&command(
            r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe",
            "-Command Remove-Item C:\\ -Recurse"
        ))
        .is_err());
        assert!(validate(&command(
            r"C:\Windows\System32\rundll32.exe",
            "shell32,Control_RunDLL"
        ))
        .is_err());
    }

    #[test]
    fn command_injection_metacharacters_do_not_change_program() {
        // No shell is involved, so these are treated as literal arguments to the fixed program
        // (they cannot spawn a second process). The metacharacters survive as-is in argv.
        let validated = validate(&command(
            r"C:\App\uninstall.exe",
            r#"/S & calc.exe | whoami ; echo pwned"#,
        ))
        .unwrap();
        let Validated::Process { program, args } = validated else {
            panic!("expected process");
        };
        assert_eq!(program, PathBuf::from(r"C:\App\uninstall.exe"));
        assert!(args.contains(&"&".to_string()));
        assert!(args.contains(&"calc.exe".to_string()));
    }

    #[test]
    fn rejects_unterminated_quote_in_arguments() {
        assert!(validate(&command(
            r"C:\App\uninstall.exe",
            r#"/dir "C:\Program Files"#
        ))
        .is_err());
    }

    #[test]
    fn rejects_newline_in_arguments_is_split_not_executed() {
        // A newline just separates tokens; it can never inject a new command (no shell).
        let validated = validate(&command(r"C:\App\uninstall.exe", "/S\ncalc.exe")).unwrap();
        let Validated::Process { args, .. } = validated else {
            panic!("expected process");
        };
        assert_eq!(args, vec!["/S", "calc.exe"]);
    }

    #[test]
    fn rejects_msi_with_arbitrary_properties_and_bad_guid() {
        assert!(validate(&command("msiexec.exe", "/x NOT-A-GUID")).is_err());
        assert!(validate(&command(
            "msiexec.exe",
            "/i {2C4E5A1B-9F0D-4A7B-8C3E-1122334455AA}"
        ))
        .is_err());
        assert!(validate(&command(
            "msiexec.exe",
            "/x {2C4E5A1B-9F0D-4A7B-8C3E-1122334455AA} EVIL=1"
        ))
        .is_err());
        assert!(validate(&command(
            "msiexec.exe",
            "/x {2C4E5A1B-9F0D-4A7B-8C3E-1122334455AA} TRANSFORMS=evil.mst"
        ))
        .is_err());
        assert!(validate(&command("msiexec.exe", "/quiet /norestart")).is_err());
        // no product code
    }

    #[test]
    fn rejects_malicious_msix_identity() {
        assert!(validate(&msix("evil'; Remove-Item C:\\")).is_err());
        assert!(validate(&msix("pkg name with spaces")).is_err());
        assert!(validate(&msix("pkg&calc")).is_err());
        assert!(validate(&msix("")).is_err());
    }

    // ---- preview cannot leak a dangerous command ------------------------

    #[test]
    fn preview_blocks_dangerous_targets_without_showing_them() {
        let blocked = preview(&command(
            r"C:\Windows\System32\cmd.exe",
            "/C rmdir /S /Q C:\\Windows",
        ));
        assert_eq!(blocked.command, "This uninstaller was blocked for safety.");
        assert!(!blocked.command.contains("rmdir"));

        let bad_msix = preview(&msix("evil'; Remove-Item C:\\"));
        assert!(!bad_msix.command.contains("Remove-Item C:\\"));
    }

    #[test]
    fn preview_shows_normalized_command_for_valid_targets() {
        let exe = preview(&command(r"C:\App\uninstall.exe", "/S"));
        assert_eq!(exe.command, "\"C:\\App\\uninstall.exe\" /S");
        assert_eq!(exe.mechanism, UninstallMechanism::RegisteredCommand);

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

    // ---- argument parser unit tests ------------------------------------

    #[test]
    fn parses_quoted_and_empty_arguments() {
        assert_eq!(parse_arguments("").unwrap(), Vec::<String>::new());
        assert_eq!(parse_arguments("   ").unwrap(), Vec::<String>::new());
        assert_eq!(
            parse_arguments(r#"/dir "C:\Program Files\App" /q"#).unwrap(),
            vec![r"/dir", r"C:\Program Files\App", "/q"]
        );
        assert_eq!(parse_arguments(r#""a""b""#).unwrap(), vec![r#"a"b"#]);
        assert!(parse_arguments(r#"unterminated "quote"#).is_err());
    }

    #[test]
    fn recognizes_product_codes() {
        assert!(is_product_code("{2C4E5A1B-9F0D-4A7B-8C3E-1122334455AA}"));
        assert!(!is_product_code("2C4E5A1B-9F0D-4A7B-8C3E-1122334455AA"));
        assert!(!is_product_code("{2C4E5A1B-9F0D-4A7B-8C3E-1122334455A}"));
        assert!(!is_product_code("{ZZZZZZZZ-9F0D-4A7B-8C3E-1122334455AA}"));
    }
}
