use std::path::{Path, PathBuf};

pub(in crate::catalog) fn should_visit_directory(path: &Path, excluded: &[PathBuf]) -> bool {
    let path = path.to_string_lossy().to_lowercase();
    if excluded
        .iter()
        .any(|value| crate::catalog::path_is_within(&path, &value.to_string_lossy().to_lowercase()))
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
            | ".venv"
            | "venv"
            | "env"
            | "site-packages"
            | "chipset_software"
            | "issetupprerequisites"
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

pub(in crate::catalog) fn is_portable_candidate(path: &Path) -> bool {
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
    if crate::catalog::filters::is_uninstall_target_path(path)
        || stem.contains("redist")
        || stem.contains("redistributable")
    {
        return false;
    }
    if crate::catalog::filters::is_helper_executable_stem(&stem) {
        return false;
    }
    let name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();
    !is_helper_named(&name)
}

const HELPER_SUBSTRINGS: &[&str] = &[
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
    "subprocess",
    "sessionmonitor",
    "blizzarderror",
    "blizzardbrowser",
];

const HELPER_TOKENS: &[&str] = &[
    "helper", "service", "daemon", "watchdog", "tracing", "elevated", "proxy", "overlay", "runtime",
];

fn is_helper_named(name: &str) -> bool {
    if HELPER_SUBSTRINGS.iter().any(|marker| name.contains(marker)) {
        return true;
    }
    name.split(|character: char| !character.is_alphanumeric())
        .filter(|token| !token.is_empty())
        .any(|token| HELPER_TOKENS.contains(&token))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_installers_but_rejects_helpers_and_documentation_executables() {
        for path in [
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
        assert!(is_portable_candidate(Path::new(r"C:\Apps\setup-app.exe")));
        assert!(is_portable_candidate(Path::new(
            r"C:\Apps\App-Installer.exe"
        )));
        assert!(is_portable_candidate(Path::new(
            r"C:\Downloads\tsetup-x64.7.3.4.exe"
        )));
        assert!(is_portable_candidate(Path::new(r"C:\Apps\rufus-4.11p.exe")));
        assert!(is_portable_candidate(Path::new(r"C:\Apps\Notepad.exe")));
    }

    #[test]
    fn real_applications_that_merely_contain_a_helper_word_are_kept() {
        for path in [
            r"C:\Apps\ServiceDesk.exe",
            r"C:\Apps\ProxySwitcher.exe",
            r"C:\Apps\RuntimeEditor.exe",
            r"C:\Apps\OverlayStudio.exe",
            r"C:\Apps\DaemonTools.exe",
        ] {
            assert!(is_portable_candidate(Path::new(path)), "{path}");
        }
    }

    #[test]
    fn genuine_helpers_are_still_rejected() {
        for path in [
            r"C:\Apps\notification_helper.exe",
            r"C:\Apps\battlenet.overlay.runtime.exe",
            r"C:\Apps\CefSharp.BrowserSubprocess.exe",
            r"C:\Apps\update-service.exe",
            r"C:\Apps\WerFault.exe",
        ] {
            assert!(!is_portable_candidate(Path::new(path)), "{path}");
        }
    }

    #[test]
    fn an_exclusion_does_not_swallow_similarly_named_folders() {
        let excluded = vec![PathBuf::from(r"D:\Games")];

        assert!(!should_visit_directory(Path::new(r"D:\Games"), &excluded));
        assert!(!should_visit_directory(
            Path::new(r"D:\Games\Steam"),
            &excluded
        ));
        assert!(should_visit_directory(
            Path::new(r"D:\GamesBackup"),
            &excluded
        ));
        assert!(should_visit_directory(Path::new(r"D:\Games2"), &excluded));
    }

    #[test]
    fn an_exclusion_matches_regardless_of_trailing_separator_or_case() {
        for excluded in [
            vec![PathBuf::from(r"D:\Games\")],
            vec![PathBuf::from(r"d:\GAMES")],
        ] {
            assert!(!should_visit_directory(
                Path::new(r"D:\Games\Steam"),
                &excluded
            ));
            assert!(should_visit_directory(
                Path::new(r"D:\GamesBackup"),
                &excluded
            ));
        }
    }

    #[test]
    fn skips_hidden_runtime_directories() {
        for path in [
            Path::new(r"C:\Users\Example\.cache"),
            Path::new(r"C:\Users\Example\.codex"),
            Path::new(r"C:\Apps\node_modules"),
        ] {
            assert!(!should_visit_directory(path, &[]), "{}", path.display());
        }
        assert!(should_visit_directory(
            Path::new(r"C:\Users\Example\.local"),
            &[]
        ));
    }

    #[test]
    fn skips_python_virtual_environment_directories() {
        for path in [
            Path::new(r"D:\Github sites\Project\backend\.venv"),
            Path::new(r"D:\Github sites\Project\backend\venv"),
            Path::new(r"D:\Projects\App\env"),
            Path::new(r"C:\Python\Lib\site-packages"),
        ] {
            assert!(!should_visit_directory(path, &[]), "{}", path.display());
        }
        assert!(should_visit_directory(
            Path::new(r"D:\Apps\Environments"),
            &[]
        ));
        assert!(should_visit_directory(Path::new(r"D:\Apps\Venvender"), &[]));
    }

    #[test]
    fn skips_driver_installer_staging_directories() {
        assert!(!should_visit_directory(
            Path::new(r"C:\AMD\Chipset_Software"),
            &[]
        ));
        assert!(!should_visit_directory(
            Path::new(r"D:\Drivers\AMD\chipset_software"),
            &[]
        ));
        assert!(should_visit_directory(Path::new(r"C:\AMD"), &[]));
        assert!(should_visit_directory(Path::new(r"C:\AMD\Adrenalin"), &[]));
        assert!(!should_visit_directory(
            Path::new(r"E:\Apps\KOMPAS-3D V19\ISSetupPrerequisites"),
            &[]
        ));
    }
}
