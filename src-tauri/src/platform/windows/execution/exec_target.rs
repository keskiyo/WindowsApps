use std::path::{Path, PathBuf};

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RejectedTarget {
    Empty,
    NetworkOrDevice,
    NotExecutable,
    NotAbsolute,
    Interpreter,
}

pub(crate) fn validate_executable_path(value: &str) -> Result<PathBuf, RejectedTarget> {
    let expanded = expand_env(value.trim());
    let candidate = expanded.trim().trim_matches('"').trim();
    if candidate.is_empty() {
        return Err(RejectedTarget::Empty);
    }
    if candidate.starts_with(r"\\") || candidate.starts_with("//") {
        return Err(RejectedTarget::NetworkOrDevice);
    }
    let path = Path::new(candidate);
    let extension_is_exe = path
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case("exe"));
    if !extension_is_exe {
        return Err(RejectedTarget::NotExecutable);
    }
    if !path.is_absolute() {
        return Err(RejectedTarget::NotAbsolute);
    }
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if INTERPRETER_EXECUTABLES.contains(&file_name.as_str()) {
        return Err(RejectedTarget::Interpreter);
    }
    Ok(path.to_path_buf())
}

pub(crate) fn system_msiexec() -> PathBuf {
    system_root().join("System32").join("msiexec.exe")
}

fn system_root() -> PathBuf {
    std::env::var("SystemRoot")
        .or_else(|_| std::env::var("windir"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(r"C:\Windows"))
}

pub(crate) fn expand_env(value: &str) -> String {
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

    #[test]
    fn accepts_an_absolute_local_executable() {
        assert_eq!(
            validate_executable_path(r"C:\Program Files\App\app.exe").unwrap(),
            PathBuf::from(r"C:\Program Files\App\app.exe")
        );
        assert_eq!(
            validate_executable_path(r#""C:\Program Files\App\app.EXE""#).unwrap(),
            PathBuf::from(r"C:\Program Files\App\app.EXE")
        );
    }

    #[test]
    fn rejects_empty_relative_unc_and_device_targets() {
        assert_eq!(validate_executable_path(""), Err(RejectedTarget::Empty));
        assert_eq!(validate_executable_path("   "), Err(RejectedTarget::Empty));
        assert_eq!(
            validate_executable_path(r"app\uninstall.exe"),
            Err(RejectedTarget::NotAbsolute)
        );
        assert_eq!(
            validate_executable_path(r"\\server\share\app.exe"),
            Err(RejectedTarget::NetworkOrDevice)
        );
        assert_eq!(
            validate_executable_path(r"\\?\C:\app.exe"),
            Err(RejectedTarget::NetworkOrDevice)
        );
        assert_eq!(
            validate_executable_path(r"\\.\pipe\app.exe"),
            Err(RejectedTarget::NetworkOrDevice)
        );
        assert_eq!(
            validate_executable_path("//server/share/app.exe"),
            Err(RejectedTarget::NetworkOrDevice)
        );
    }

    #[test]
    fn rejects_non_executables_and_interpreters() {
        for value in [
            r"C:\App\install.bat",
            r"C:\App\run.cmd",
            r"C:\App\script.ps1",
            r"C:\App\notes.url",
            r"C:\App\link.lnk",
        ] {
            assert_eq!(
                validate_executable_path(value),
                Err(RejectedTarget::NotExecutable),
                "{value}"
            );
        }
        for value in [
            r"C:\Windows\System32\cmd.exe",
            r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe",
            r"C:\Windows\System32\rundll32.exe",
        ] {
            assert_eq!(
                validate_executable_path(value),
                Err(RejectedTarget::Interpreter),
                "{value}"
            );
        }
    }

    #[test]
    fn expands_known_variables_and_keeps_unknown_ones_verbatim() {
        std::env::set_var("WINAPPS_TEST_EXPAND_ROOT", r"C:\Base");
        assert_eq!(
            expand_env(r"%WINAPPS_TEST_EXPAND_ROOT%\app.exe"),
            r"C:\Base\app.exe"
        );
        assert_eq!(
            expand_env(r"%WINAPPS_TEST_MISSING_VAR%\app.exe"),
            r"%WINAPPS_TEST_MISSING_VAR%\app.exe"
        );
        assert_eq!(expand_env(r"C:\plain\app.exe"), r"C:\plain\app.exe");
    }

    #[test]
    fn expanded_variables_make_a_registry_path_usable() {
        std::env::set_var("WINAPPS_TEST_VALIDATE_ROOT", r"C:\Base");
        assert_eq!(
            validate_executable_path(r"%WINAPPS_TEST_VALIDATE_ROOT%\App\app.exe").unwrap(),
            PathBuf::from(r"C:\Base\App\app.exe")
        );
    }

    #[test]
    fn the_installer_host_resolves_under_the_windows_directory() {
        assert!(system_msiexec().starts_with(system_root()));
        assert!(system_msiexec().is_absolute());
    }
}
