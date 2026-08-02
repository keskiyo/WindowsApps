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
            // Python virtual environments and package trees hold project-local interpreters
            // (`.venv\Scripts\python.exe`) and console-script shims, not user-facing apps.
            // Without this each project's `.venv` surfaced its own "python" card.
            | ".venv"
            | "venv"
            | "env"
            | "site-packages"
            // Driver installer staging trees (e.g. AMD chipset). Extraction folders hold service
            // binaries duplicated across `Binaries\` and per-OS `Packages\...\W11x64`/`WTx64`
            // subtrees — each surfaced its own driver-service "app". These are not installed
            // applications; the real services run from system directories.
            | "chipset_software"
            // InstallShield bundles redistributable prerequisites (Access Database Engine, VC++
            // runtimes, …) under this folder inside a product tree — installer payload, not apps.
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

/// Specific, unambiguous fragments — matched anywhere in the name, because they do not occur
/// inside real application names (`CefSharp.BrowserSubprocess`, `BlizzardError`, `dxsetup`).
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

/// Generic English words — matched only as a whole token. As a substring they swallowed real
/// applications: `ServiceDesk` matched "service", `ProxySwitcher` matched "proxy",
/// `RuntimeEditor` matched "runtime". A component that really is one of these carries the word
/// as its own token (`notification_helper`, `battlenet.overlay.runtime`).
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

    // The helper markers used to match as substrings, so real applications whose name merely
    // contains one of the generic words were silently dropped from discovery.
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

    // Genuine helpers/components carry the word as its own token and stay rejected — the
    // existing behaviour these markers were added for.
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

    // Excluding a folder must not also exclude its name-alike neighbours: with a raw prefix
    // comparison, excluding `D:\Games` silently removed `D:\GamesBackup` from scanning too.
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

    // Each Python project ships its own `.venv\Scripts\python.exe`; before this every project
    // surfaced a separate "python" card. The environment folder itself is skipped so the
    // interpreter and console-script shims inside are never discovered.
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
        // A real application whose name merely starts with those letters must still be scanned.
        assert!(should_visit_directory(
            Path::new(r"D:\Apps\Environments"),
            &[]
        ));
        assert!(should_visit_directory(Path::new(r"D:\Apps\Venvender"), &[]));
    }

    // AMD chipset drivers extract to `C:\AMD\Chipset_Software\` and duplicate every service
    // binary across `Binaries\` and per-OS `Packages\...\W11x64`/`WTx64` folders, so each one
    // surfaced as its own "AMD ... Service" card. The staging tree is not applications.
    #[test]
    fn skips_driver_installer_staging_directories() {
        // The walk stops at the staging root, so its duplicated service subtrees are never
        // entered — the leaf-name check does the pruning during recursion.
        assert!(!should_visit_directory(
            Path::new(r"C:\AMD\Chipset_Software"),
            &[]
        ));
        assert!(!should_visit_directory(
            Path::new(r"D:\Drivers\AMD\chipset_software"),
            &[]
        ));
        // The AMD root itself and unrelated AMD apps are still scanned.
        assert!(should_visit_directory(Path::new(r"C:\AMD"), &[]));
        assert!(should_visit_directory(Path::new(r"C:\AMD\Adrenalin"), &[]));
        // InstallShield prerequisite payload (Access Database Engine, VC++ runtimes) is skipped.
        assert!(!should_visit_directory(
            Path::new(r"E:\Apps\KOMPAS-3D V19\ISSetupPrerequisites"),
            &[]
        ));
    }
}
