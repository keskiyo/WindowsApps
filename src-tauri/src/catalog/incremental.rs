use crate::catalog::{portable, portable_app, AppInfo};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScanMode {
    Incremental,
    Force,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FilesystemIndex {
    pub directories: BTreeMap<String, DirectoryRecord>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryRecord {
    pub modified_nanos: u128,
    pub child_directories: Vec<String>,
    pub apps: Vec<AppInfo>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ScanStatistics {
    pub directories_enumerated: usize,
    pub executables_inspected: usize,
}

pub struct IncrementalScanResult {
    pub apps: Vec<AppInfo>,
    pub index: FilesystemIndex,
    pub statistics: ScanStatistics,
}

pub fn scan_root(
    root: &Path,
    previous: &FilesystemIndex,
    mode: ScanMode,
    excluded: &[PathBuf],
    is_cancelled: impl Fn() -> bool,
) -> IncrementalScanResult {
    let mut result = IncrementalScanResult {
        apps: Vec::new(),
        index: FilesystemIndex::default(),
        statistics: ScanStatistics::default(),
    };
    visit_directory(root, previous, mode, excluded, &is_cancelled, &mut result);
    result
        .apps
        .sort_by_cached_key(|app| app.path.to_lowercase());
    result
}

fn visit_directory(
    directory: &Path,
    previous: &FilesystemIndex,
    mode: ScanMode,
    excluded: &[PathBuf],
    is_cancelled: &impl Fn() -> bool,
    result: &mut IncrementalScanResult,
) {
    if is_cancelled()
        || !directory.is_dir()
        || !portable::should_visit_directory(directory, excluded)
    {
        return;
    }
    let key = normalized_path(directory);
    let modified_nanos = directory_modified_nanos(directory);
    let cached = previous.directories.get(&key);
    let unchanged = mode == ScanMode::Incremental
        && cached.is_some_and(|record| record.modified_nanos == modified_nanos);

    if unchanged {
        let record = cached.expect("checked above").clone();
        result.apps.extend(record.apps.iter().cloned());
        result.index.directories.insert(key, record.clone());
        for child in record.child_directories {
            visit_directory(
                Path::new(&child),
                previous,
                mode,
                excluded,
                is_cancelled,
                result,
            );
        }
        return;
    }

    let Ok(entries) = fs::read_dir(directory) else {
        if let Some(record) = cached.cloned() {
            result.apps.extend(record.apps.iter().cloned());
            result.index.directories.insert(key, record);
        }
        return;
    };
    result.statistics.directories_enumerated += 1;
    let mut children = Vec::new();
    let mut direct_apps = Vec::new();
    for entry in entries.filter_map(Result::ok) {
        if is_cancelled() {
            break;
        }
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_dir() {
            if portable::should_visit_directory(&path, excluded) {
                children.push(path.to_string_lossy().into_owned());
            }
        } else if file_type.is_file() && portable::is_portable_candidate(&path) {
            result.statistics.executables_inspected += 1;
            if let Some(app) = portable_app(path) {
                direct_apps.push(app);
            }
        }
    }
    children.sort_by_key(|path| path.to_lowercase());
    direct_apps.sort_by_cached_key(|app| app.path.to_lowercase());
    result.apps.extend(direct_apps.iter().cloned());
    result.index.directories.insert(
        key,
        DirectoryRecord {
            modified_nanos,
            child_directories: children.clone(),
            apps: direct_apps,
        },
    );
    for child in children {
        visit_directory(
            Path::new(&child),
            previous,
            mode,
            excluded,
            is_cancelled,
            result,
        );
    }
}

fn normalized_path(path: &Path) -> String {
    path.to_string_lossy().to_lowercase()
}

fn directory_modified_nanos(path: &Path) -> u128 {
    fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |duration| duration.as_nanos())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unchanged_directories_reuse_cached_apps_without_rechecking_executables() {
        let root = tempfile::tempdir().unwrap();
        let editor = root.path().join("Editor");
        std::fs::create_dir_all(&editor).unwrap();
        std::fs::write(editor.join("Editor.exe"), []).unwrap();

        let first = scan_root(
            root.path(),
            &FilesystemIndex::default(),
            ScanMode::Force,
            &[],
            || false,
        );
        let second = scan_root(
            root.path(),
            &first.index,
            ScanMode::Incremental,
            &[],
            || false,
        );

        assert_eq!(first.apps.len(), 1);
        assert_eq!(second.apps, first.apps);
        assert_eq!(second.statistics.executables_inspected, 0);
    }

    #[test]
    fn changed_nested_directory_adds_and_removes_apps() {
        let root = tempfile::tempdir().unwrap();
        let nested = root.path().join("Apps").join("Tool");
        std::fs::create_dir_all(&nested).unwrap();
        let original = nested.join("Tool.exe");
        std::fs::write(&original, []).unwrap();
        let first = scan_root(
            root.path(),
            &FilesystemIndex::default(),
            ScanMode::Force,
            &[],
            || false,
        );
        std::fs::remove_file(original).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        let replacement_dir = root.path().join("Apps").join("Replacement");
        std::fs::create_dir_all(&replacement_dir).unwrap();
        std::fs::write(replacement_dir.join("Replacement.exe"), []).unwrap();

        let second = scan_root(
            root.path(),
            &first.index,
            ScanMode::Incremental,
            &[],
            || false,
        );

        assert_eq!(second.apps.len(), 1);
        assert_eq!(second.apps[0].name, "Replacement");
    }
}
