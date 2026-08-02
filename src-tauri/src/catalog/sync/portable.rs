use crate::catalog::incremental::{scan_root_with_duration, FilesystemIndex, ScanLimit, ScanMode};
use crate::catalog::{AppInfo, ScanProgress};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub(super) struct PortableScanResult {
    pub(super) apps: Vec<AppInfo>,
    pub(super) filesystem_index: FilesystemIndex,
}

pub(super) struct PortableScanInput<'a> {
    pub(super) previous_apps: &'a [AppInfo],
    pub(super) previous_index: &'a FilesystemIndex,
    pub(super) roots: &'a [PathBuf],
    pub(super) excluded: &'a [PathBuf],
    pub(super) mode: ScanMode,
    pub(super) max_duration: Duration,
}

pub(super) fn scan_roots(
    input: PortableScanInput<'_>,
    progress: &impl Fn(ScanProgress),
    is_cancelled: &impl Fn() -> bool,
) -> PortableScanResult {
    let started_at = Instant::now();
    let mut apps = BTreeMap::new();
    let mut filesystem_index = FilesystemIndex::default();
    progress(ScanProgress {
        stage: "Portable applications".into(),
        location: None,
        completed_roots: 0,
        total_roots: input.roots.len(),
    });
    for (index, root) in input.roots.iter().enumerate() {
        if is_cancelled() {
            break;
        }
        let remaining = input.max_duration.saturating_sub(started_at.elapsed());
        let scanned = scan_root_with_duration(
            root,
            input.previous_index,
            input.mode,
            input.excluded,
            is_cancelled,
            remaining,
        );
        let complete = !matches!(
            scanned.limit_reached,
            Some(ScanLimit::Entries | ScanLimit::Time)
        );
        for app in merge_root_apps(input.previous_apps, scanned.apps, root, complete) {
            apps.insert(app.id.clone(), app);
        }
        filesystem_index.directories.extend(
            merge_root_index(input.previous_index, scanned.index, root, complete).directories,
        );
        progress(ScanProgress {
            stage: scanned.limit_reached.map_or_else(
                || "Portable applications".into(),
                |limit| format!("Portable applications · {}", limit.message()),
            ),
            location: Some(root.to_string_lossy().into_owned()),
            completed_roots: index + 1,
            total_roots: input.roots.len(),
        });
    }
    PortableScanResult {
        apps: apps.into_values().collect(),
        filesystem_index,
    }
}

fn merge_root_apps(
    previous: &[AppInfo],
    scanned: Vec<AppInfo>,
    root: &Path,
    complete: bool,
) -> Vec<AppInfo> {
    let mut apps = BTreeMap::new();
    if !complete {
        for app in previous
            .iter()
            .filter(|app| path_is_within_root(&app.path, root))
        {
            apps.insert(app.id.clone(), app.clone());
        }
    }
    for app in scanned {
        apps.insert(app.id.clone(), app);
    }
    apps.into_values().collect()
}

fn merge_root_index(
    previous: &FilesystemIndex,
    scanned: FilesystemIndex,
    root: &Path,
    complete: bool,
) -> FilesystemIndex {
    let mut directories = BTreeMap::new();
    if !complete {
        for (path, record) in previous
            .directories
            .iter()
            .filter(|(path, _)| path_is_within_root(path, root))
        {
            directories.insert(path.clone(), record.clone());
        }
    }
    directories.extend(scanned.directories);
    FilesystemIndex { directories }
}

fn path_is_within_root(path: &str, root: &Path) -> bool {
    let path = path.replace('/', r"\").to_lowercase();
    let root = root.to_string_lossy().replace('/', r"\").to_lowercase();
    crate::catalog::path_is_within(&path, &root)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::incremental::{DirectoryRecord, FilesystemIndex, ScanMode};
    use crate::catalog::{AppCategory, AppInfo, LaunchKind, SourceKind, VisibilityClass};
    use std::collections::BTreeMap;
    use std::path::Path;
    use std::time::Duration;

    fn app(id: &str, path: &str) -> AppInfo {
        AppInfo {
            id: id.into(),
            name: id.into(),
            path: path.into(),
            icon_base64: None,
            artifact_kind: Default::default(),
            category: AppCategory::Other,
            launch_kind: LaunchKind::Executable,
            source_kind: SourceKind::Portable,
            description: None,
            version: None,
            publisher: None,
            product_name: None,
            original_filename: None,
            install_location: Path::new(path)
                .parent()
                .map(|value| value.to_string_lossy().into_owned()),
            can_uninstall: false,
            uninstall: None,
            resolved_path: None,
            shortcut_icon_path: None,
            launch_arguments: None,
            canonical_identity: None,
            preference_identity: None,
            visibility_class: VisibilityClass::Primary,
            visibility_score: 20,
            visibility_reasons: Vec::new(),
        }
    }

    #[test]
    fn incomplete_root_keeps_previous_apps_and_overlays_new_results() {
        let root = Path::new(r"D:\Apps");
        let previous = vec![
            app("same", r"D:\Apps\Old.exe"),
            app("kept", r"D:\Apps\Kept.exe"),
        ];
        let scanned = vec![app("same", r"D:\Apps\New.exe")];

        let merged = merge_root_apps(&previous, scanned, root, false);

        assert_eq!(
            merged.iter().map(|app| app.id.as_str()).collect::<Vec<_>>(),
            vec!["kept", "same"]
        );
        assert_eq!(
            merged.iter().find(|app| app.id == "same").unwrap().path,
            r"D:\Apps\New.exe"
        );
    }

    #[test]
    fn completed_root_replaces_stale_apps() {
        let root = Path::new(r"D:\Apps");
        let previous = vec![app("stale", r"D:\Apps\Stale.exe")];

        assert!(merge_root_apps(&previous, Vec::new(), root, true).is_empty());
    }

    #[test]
    fn removed_scan_root_does_not_keep_previous_apps() {
        let previous = vec![app("removed-root", r"E:\Portable\Tool.exe")];

        assert!(merge_root_apps(&previous, Vec::new(), Path::new(r"D:\Apps"), false,).is_empty());
    }

    #[test]
    fn incomplete_root_keeps_previous_index_records() {
        let root = Path::new(r"D:\Apps");
        let previous = FilesystemIndex {
            directories: BTreeMap::from([(
                r"d:\apps\kept".into(),
                DirectoryRecord {
                    modified_nanos: 1,
                    child_directories: Vec::new(),
                    apps: vec![app("kept", r"D:\Apps\Kept\kept.exe")],
                },
            )]),
        };

        let merged = merge_root_index(&previous, FilesystemIndex::default(), root, false);

        assert!(merged.directories.contains_key(r"d:\apps\kept"));
    }

    #[test]
    fn zero_total_budget_preserves_every_unvisited_root() {
        let first = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        let previous = vec![
            app("first", &first.path().join("First.exe").to_string_lossy()),
            app(
                "second",
                &second.path().join("Second.exe").to_string_lossy(),
            ),
        ];

        let previous_index = FilesystemIndex::default();
        let roots = [first.path().to_path_buf(), second.path().to_path_buf()];
        let scanned = scan_roots(
            PortableScanInput {
                previous_apps: &previous,
                previous_index: &previous_index,
                roots: &roots,
                excluded: &[],
                mode: ScanMode::Incremental,
                max_duration: Duration::ZERO,
            },
            &|_| {},
            &|| false,
        );

        assert_eq!(
            scanned
                .apps
                .iter()
                .map(|app| app.id.as_str())
                .collect::<Vec<_>>(),
            vec!["first", "second"]
        );
    }
}
