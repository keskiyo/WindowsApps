use crate::catalog::{portable, portable_app, AppInfo};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::os::windows::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, UNIX_EPOCH};

pub const DEFAULT_MAX_DEPTH: usize = 16;
pub const DEFAULT_MAX_ENTRIES: usize = 500_000;
pub const DEFAULT_MAX_DURATION: Duration = Duration::from_secs(3 * 60);
const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
const CANCELLATION_CHECK_INTERVAL: usize = 128;

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
    pub entries_seen: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScanLimit {
    Depth,
    Entries,
    Time,
}

impl ScanLimit {
    pub fn message(self) -> &'static str {
        match self {
            Self::Depth => "maximum folder depth reached",
            Self::Entries => "maximum file count reached",
            Self::Time => "time limit reached",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ScanLimits {
    pub max_depth: usize,
    pub max_entries: usize,
    pub max_duration: Duration,
}

impl Default for ScanLimits {
    fn default() -> Self {
        Self {
            max_depth: DEFAULT_MAX_DEPTH,
            max_entries: DEFAULT_MAX_ENTRIES,
            max_duration: DEFAULT_MAX_DURATION,
        }
    }
}

pub struct IncrementalScanResult {
    pub apps: Vec<AppInfo>,
    pub index: FilesystemIndex,
    pub statistics: ScanStatistics,
    pub limit_reached: Option<ScanLimit>,
}

pub fn scan_root(
    root: &Path,
    previous: &FilesystemIndex,
    mode: ScanMode,
    excluded: &[PathBuf],
    is_cancelled: impl Fn() -> bool,
) -> IncrementalScanResult {
    scan_root_with_limits(
        root,
        previous,
        mode,
        excluded,
        is_cancelled,
        ScanLimits::default(),
    )
}

fn scan_root_with_limits(
    root: &Path,
    previous: &FilesystemIndex,
    mode: ScanMode,
    excluded: &[PathBuf],
    is_cancelled: impl Fn() -> bool,
    limits: ScanLimits,
) -> IncrementalScanResult {
    let mut result = IncrementalScanResult {
        apps: Vec::new(),
        index: FilesystemIndex::default(),
        statistics: ScanStatistics::default(),
        limit_reached: None,
    };
    let started_at = Instant::now();
    visit_directory(
        root,
        0,
        previous,
        mode,
        excluded,
        &is_cancelled,
        limits,
        started_at,
        &mut result,
    );
    result
        .apps
        .sort_by_cached_key(|app| app.path.to_lowercase());
    result
}

fn visit_directory(
    directory: &Path,
    depth: usize,
    previous: &FilesystemIndex,
    mode: ScanMode,
    excluded: &[PathBuf],
    is_cancelled: &impl Fn() -> bool,
    limits: ScanLimits,
    started_at: Instant,
    result: &mut IncrementalScanResult,
) {
    if should_stop(result, limits, started_at)
        || is_cancelled()
        || !is_scannable_directory(directory)
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
            if depth >= limits.max_depth {
                mark_limit(result, ScanLimit::Depth);
                continue;
            }
            visit_directory(
                Path::new(&child),
                depth + 1,
                previous,
                mode,
                excluded,
                is_cancelled,
                limits,
                started_at,
                result,
            );
            if should_stop(result, limits, started_at) {
                return;
            }
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
        if should_stop(result, limits, started_at) {
            break;
        }
        if result.statistics.entries_seen % CANCELLATION_CHECK_INTERVAL == 0 && is_cancelled() {
            break;
        }
        result.statistics.entries_seen += 1;
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            if is_reparse_point(&path) {
                continue;
            } else if depth >= limits.max_depth {
                mark_limit(result, ScanLimit::Depth);
            } else if portable::should_visit_directory(&path, excluded) {
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
    if !hard_limit_reached(result) {
        result.index.directories.insert(
            key,
            DirectoryRecord {
                modified_nanos,
                child_directories: children.clone(),
                apps: direct_apps,
            },
        );
    }
    for child in children {
        if should_stop(result, limits, started_at) {
            return;
        }
        visit_directory(
            Path::new(&child),
            depth + 1,
            previous,
            mode,
            excluded,
            is_cancelled,
            limits,
            started_at,
            result,
        );
    }
}

fn should_stop(
    result: &mut IncrementalScanResult,
    limits: ScanLimits,
    started_at: Instant,
) -> bool {
    if started_at.elapsed() >= limits.max_duration {
        mark_limit(result, ScanLimit::Time);
        return true;
    }
    if result.statistics.entries_seen >= limits.max_entries {
        mark_limit(result, ScanLimit::Entries);
        return true;
    }
    matches!(
        result.limit_reached,
        Some(ScanLimit::Entries | ScanLimit::Time)
    )
}

fn mark_limit(result: &mut IncrementalScanResult, limit: ScanLimit) {
    if result.limit_reached.is_none() || limit != ScanLimit::Depth {
        result.limit_reached = Some(limit);
    }
}

fn hard_limit_reached(result: &IncrementalScanResult) -> bool {
    matches!(
        result.limit_reached,
        Some(ScanLimit::Entries | ScanLimit::Time)
    )
}

fn is_reparse_point(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .is_ok_and(|metadata| has_reparse_point_attribute(metadata.file_attributes()))
}

fn is_scannable_directory(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|metadata| {
        metadata.is_dir() && !has_reparse_point_attribute(metadata.file_attributes())
    })
}

fn has_reparse_point_attribute(attributes: u32) -> bool {
    attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0
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
    use std::time::Duration;

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

    #[test]
    fn stops_at_the_configured_directory_depth() {
        let root = tempfile::tempdir().unwrap();
        let mut directory = root.path().to_path_buf();
        for index in 1..=3 {
            directory = directory.join(format!("level-{index}"));
            std::fs::create_dir_all(&directory).unwrap();
        }
        let allowed = root.path().join("level-1").join("Allowed");
        std::fs::create_dir_all(&allowed).unwrap();
        std::fs::write(allowed.join("Allowed.exe"), []).unwrap();
        let too_deep = directory.join("TooDeep");
        std::fs::create_dir_all(&too_deep).unwrap();
        std::fs::write(too_deep.join("TooDeep.exe"), []).unwrap();

        let result = scan_root_with_limits(
            root.path(),
            &FilesystemIndex::default(),
            ScanMode::Force,
            &[],
            || false,
            ScanLimits {
                max_depth: 2,
                max_entries: 100,
                max_duration: Duration::from_secs(10),
            },
        );

        assert!(result.apps.iter().any(|app| app.name == "Allowed"));
        assert!(!result.apps.iter().any(|app| app.name == "TooDeep"));
        assert_eq!(result.limit_reached, Some(ScanLimit::Depth));
    }

    #[test]
    fn stops_when_the_entry_budget_is_exhausted() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("Portable.exe"), []).unwrap();

        let result = scan_root_with_limits(
            root.path(),
            &FilesystemIndex::default(),
            ScanMode::Force,
            &[],
            || false,
            ScanLimits {
                max_depth: 16,
                max_entries: 0,
                max_duration: Duration::from_secs(10),
            },
        );

        assert!(result.apps.is_empty());
        assert_eq!(result.limit_reached, Some(ScanLimit::Entries));
    }

    #[test]
    fn does_not_cache_a_partially_enumerated_directory() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("First.exe"), []).unwrap();
        std::fs::write(root.path().join("Second.exe"), []).unwrap();

        let result = scan_root_with_limits(
            root.path(),
            &FilesystemIndex::default(),
            ScanMode::Force,
            &[],
            || false,
            ScanLimits {
                max_depth: 16,
                max_entries: 1,
                max_duration: Duration::from_secs(10),
            },
        );

        assert_eq!(result.limit_reached, Some(ScanLimit::Entries));
        assert!(!result
            .index
            .directories
            .contains_key(&normalized_path(root.path())));
    }

    #[test]
    fn stops_when_the_time_budget_is_exhausted() {
        let root = tempfile::tempdir().unwrap();

        let result = scan_root_with_limits(
            root.path(),
            &FilesystemIndex::default(),
            ScanMode::Force,
            &[],
            || false,
            ScanLimits {
                max_depth: 16,
                max_entries: 100,
                max_duration: Duration::ZERO,
            },
        );

        assert_eq!(result.limit_reached, Some(ScanLimit::Time));
    }

    #[test]
    fn recognizes_windows_reparse_point_attribute() {
        assert!(has_reparse_point_attribute(0x400));
        assert!(!has_reparse_point_attribute(0x20));
    }
}
