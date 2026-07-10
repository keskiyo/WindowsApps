use std::path::{Path, PathBuf};

pub(super) fn should_visit_directory(path: &Path, excluded: &[PathBuf]) -> bool {
    let path = path.to_string_lossy().to_lowercase();
    if excluded
        .iter()
        .any(|value| path.starts_with(&value.to_string_lossy().to_lowercase()))
    {
        return false;
    }
    let name = Path::new(&path)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();
    !matches!(
        name.as_str(),
        "$recycle.bin"
            | "system volume information"
            | "windows"
            | "windowsapps"
            | "winsxs"
            | "program files"
            | "program files (x86)"
            | "programdata"
            | "recovery"
            | "perflogs"
            | "documents and settings"
            | "node_modules"
            | ".cache"
            | ".codex"
            | ".git"
            | ".svn"
            | "target"
            | "cache"
            | "caches"
            | "temp"
            | "tmp"
    )
}

pub(super) fn is_portable_candidate(path: &Path) -> bool {
    if !path
        .extension()
        .is_some_and(|value| value.eq_ignore_ascii_case("exe"))
    {
        return false;
    }
    let stem = path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();
    if super::is_installer_file_name(&stem) || super::is_helper_executable_stem(&stem) {
        return false;
    }
    let name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();
    ![
        "update.exe",
        "repair.exe",
        "bootstrap",
        "crashpad",
        "crashreport",
        "crash_handler",
        "crashhandler",
        "werfault",
        "dxsetup",
        "eac_launcher",
        "easyanticheat",
        "workshoputility",
        "workshop_utility",
        "workshop utility",
        "readme",
        "manual",
        "helper",
        "subprocess",
        "service",
        "daemon",
        "watchdog",
        "tracing",
        "elevated",
        "proxy",
        "overlay",
        "runtime",
        "sessionmonitor",
        "blizzarderror",
        "blizzardbrowser",
    ]
    .iter()
    .any(|marker| name.contains(marker))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_installer_helper_and_documentation_executables() {
        for path in [
            r"C:\Apps\setup-app.exe",
            r"C:\Apps\App-Installer.exe",
            r"C:\Apps\vcredist_x64.exe",
            r"C:\Apps\unins000.exe",
            r"C:\Apps\readme.exe",
            r"C:\Apps\CefSharp.BrowserSubprocess.exe",
            r"C:\Apps\notification_helper.exe",
            r"C:\Apps\BlizzardError.exe",
            r"C:\Apps\battlenet.overlay.runtime.exe",
            r"C:\Apps\git-lfs.exe",
            r"C:\Apps\git-credential-manager.exe",
            r"C:\Apps\gettext.exe",
            r"C:\Apps\printf_gettext.exe",
            r"C:\Apps\printf_ngettext.exe",
        ] {
            assert!(!is_portable_candidate(Path::new(path)), "{path}");
        }
        assert!(is_portable_candidate(Path::new(r"C:\Apps\rufus-4.11p.exe")));
        assert!(is_portable_candidate(Path::new(r"C:\Apps\Notepad.exe")));
    }

    #[test]
    fn skips_hidden_runtime_directories() {
        for path in [
            Path::new(r"C:\Users\Maks\.cache"),
            Path::new(r"C:\Users\Maks\.codex"),
            Path::new(r"C:\Apps\node_modules"),
        ] {
            assert!(!should_visit_directory(path, &[]), "{}", path.display());
        }
        assert!(should_visit_directory(
            Path::new(r"C:\Users\Maks\.local"),
            &[]
        ));
    }
}
