use std::path::{Path, PathBuf};
use walkdir::{DirEntry, WalkDir};

pub(super) fn discover_executables(
    roots: &[PathBuf],
    excluded: &[PathBuf],
    is_cancelled: impl Fn() -> bool,
) -> Vec<PathBuf> {
    let mut executables = Vec::new();
    for root in roots {
        if is_cancelled() {
            break;
        }
        let entries = WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|entry| should_visit(entry, excluded));
        for entry in entries.filter_map(Result::ok) {
            if is_cancelled() {
                return executables;
            }
            let path = entry.path();
            if entry.file_type().is_file() && is_portable_candidate(path) {
                executables.push(path.to_path_buf());
            }
        }
    }
    executables.sort_by_cached_key(|path| path.to_string_lossy().to_lowercase());
    executables
}

fn should_visit(entry: &DirEntry, excluded: &[PathBuf]) -> bool {
    if entry.depth() == 0 {
        return true;
    }
    if !entry.file_type().is_dir() {
        return true;
    }
    should_visit_directory(entry.path(), excluded)
}

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
    if super::is_installer_file_name(&stem) {
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
        // background helper / service processes that ship next to a real app
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
    fn finds_portable_apps_and_filters_maintenance_and_excluded_directories() {
        let root = tempfile::tempdir().unwrap();
        let apps = root.path().join("random apps");
        let excluded = root.path().join("ignored");
        let windows = root.path().join("Windows");
        std::fs::create_dir_all(&apps).unwrap();
        std::fs::create_dir_all(&excluded).unwrap();
        std::fs::create_dir_all(&windows).unwrap();
        let rufus = apps.join("rufus-4.11p.exe");
        std::fs::write(&rufus, []).unwrap();
        std::fs::write(apps.join("setup.exe"), []).unwrap();
        std::fs::write(excluded.join("HiddenTool.exe"), []).unwrap();
        std::fs::write(windows.join("SystemTool.exe"), []).unwrap();

        let found = discover_executables(&[root.path().to_path_buf()], &[excluded], || false);

        assert_eq!(found, vec![rufus]);
    }

    #[test]
    fn rejects_installer_and_documentation_executables() {
        assert!(!is_portable_candidate(Path::new(r"C:\Apps\setup-app.exe")));
        assert!(!is_portable_candidate(Path::new(
            r"C:\Apps\App-Installer.exe"
        )));
        assert!(!is_portable_candidate(Path::new(r"C:\Apps\Setup_x64.exe")));
        assert!(!is_portable_candidate(Path::new(
            r"C:\Apps\vcredist_x64.exe"
        )));
        assert!(!is_portable_candidate(Path::new(r"C:\Apps\unins000.exe")));
        assert!(!is_portable_candidate(Path::new(r"C:\Apps\AppSetup.exe")));
        assert!(!is_portable_candidate(Path::new(r"C:\Apps\readme.exe")));
        assert!(!is_portable_candidate(Path::new(
            r"C:\Apps\vcredist2005_x64.exe"
        )));
        assert!(!is_portable_candidate(Path::new(
            r"C:\Apps\CefSharp.BrowserSubprocess.exe"
        )));
        assert!(!is_portable_candidate(Path::new(
            r"C:\Apps\notification_helper.exe"
        )));
        assert!(!is_portable_candidate(Path::new(
            r"C:\chrome-win\elevated_tracing_service.exe"
        )));
        assert!(!is_portable_candidate(Path::new(
            r"C:\Ассистент\ast_service.exe"
        )));
        assert!(!is_portable_candidate(Path::new(
            r"C:\Apps\BlizzardError.exe"
        )));
        assert!(!is_portable_candidate(Path::new(
            r"C:\Apps\battlenet.overlay.runtime.exe"
        )));
        assert!(!is_portable_candidate(Path::new(
            r"C:\Apps\BlizzardBrowser.exe"
        )));
        assert!(!is_portable_candidate(Path::new(
            r"C:\Apps\GameSessionMonitor.exe"
        )));
        assert!(!is_portable_candidate(Path::new(r"C:\Apps\7z2501-x64.exe")));
        assert!(is_portable_candidate(Path::new(r"C:\Apps\rufus-4.11p.exe")));
        assert!(is_portable_candidate(Path::new(r"C:\Apps\Notepad.exe")));
    }

    #[test]
    fn stops_when_scan_is_cancelled() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("Portable.exe"), []).unwrap();

        assert!(discover_executables(&[root.path().to_path_buf()], &[], || true).is_empty());
    }
}
