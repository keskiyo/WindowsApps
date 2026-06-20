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
    let path = entry.path().to_string_lossy().to_lowercase();
    if excluded
        .iter()
        .any(|value| path.starts_with(&value.to_string_lossy().to_lowercase()))
    {
        return false;
    }
    if !entry.file_type().is_dir() {
        return true;
    }
    let name = entry.file_name().to_string_lossy().to_lowercase();
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

fn is_portable_candidate(path: &Path) -> bool {
    if !path
        .extension()
        .is_some_and(|value| value.eq_ignore_ascii_case("exe"))
    {
        return false;
    }
    let name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();
    ![
        "setup.exe",
        "installer",
        "uninstall",
        "unins000",
        "updater",
        "update.exe",
        "repair.exe",
        "bootstrap",
        "crashpad",
        "crashreport",
        "crash_handler",
        "vc_redist",
        "vcredist",
        "dxsetup",
        "eac_launcher",
        "easyanticheat",
        "workshoputility",
        "workshop_utility",
        "workshop utility",
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
    fn stops_when_scan_is_cancelled() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("Portable.exe"), []).unwrap();

        assert!(discover_executables(&[root.path().to_path_buf()], &[], || true).is_empty());
    }
}
