use crate::catalog::{portable, portable_app, AppInfo};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::os::windows::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, UNIX_EPOCH};

pub(crate) const DEFAULT_MAX_DEPTH: usize = 16;
pub(crate) const DEFAULT_MAX_ENTRIES: usize = 500_000;
pub(crate) const DEFAULT_MAX_DURATION: Duration = Duration::from_secs(3 * 60);
const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
const CANCELLATION_CHECK_INTERVAL: usize = 128;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ScanMode {
    Incremental,
    Force,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FilesystemIndex {
    pub directories: BTreeMap<String, DirectoryRecord>,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FileFingerprint {
    pub size: u64,
    pub modified_nanos: u128,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DirectoryRecord {
    pub modified_nanos: u128,
    pub child_directories: Vec<String>,
    pub apps: Vec<AppInfo>,
    #[serde(default)]
    pub executables: BTreeMap<String, FileFingerprint>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct ScanStatistics {
    pub directories_enumerated: usize,
    pub executables_inspected: usize,
    pub entries_seen: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ScanLimit {
    Depth,
    Entries,
    Time,
}

impl ScanLimit {
    pub(crate) fn message(self) -> &'static str {
        match self {
            Self::Depth => "maximum folder depth reached",
            Self::Entries => "maximum file count reached",
            Self::Time => "time limit reached",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct ScanLimits {
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

pub(crate) struct IncrementalScanResult {
    pub apps: Vec<AppInfo>,
    pub index: FilesystemIndex,
    pub statistics: ScanStatistics,
    pub limit_reached: Option<ScanLimit>,
}

#[cfg(test)]
pub(crate) fn scan_root(
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
        true,
    )
}

pub(crate) fn scan_root_with_duration(
    root: &Path,
    previous: &FilesystemIndex,
    mode: ScanMode,
    excluded: &[PathBuf],
    is_cancelled: impl Fn() -> bool,
    max_duration: Duration,
    verify_fingerprints: bool,
) -> IncrementalScanResult {
    scan_root_with_limits(
        root,
        previous,
        mode,
        excluded,
        is_cancelled,
        ScanLimits {
            max_duration,
            ..ScanLimits::default()
        },
        verify_fingerprints,
    )
}

fn scan_root_with_limits(
    root: &Path,
    previous: &FilesystemIndex,
    mode: ScanMode,
    excluded: &[PathBuf],
    is_cancelled: impl Fn() -> bool,
    limits: ScanLimits,
    verify_fingerprints: bool,
) -> IncrementalScanResult {
    let mut result = IncrementalScanResult {
        apps: Vec::new(),
        index: FilesystemIndex::default(),
        statistics: ScanStatistics::default(),
        limit_reached: None,
    };
    let facts = crate::catalog::machine::MachineFacts::current();
    let context = VisitContext {
        previous,
        mode,
        excluded,
        is_cancelled: &is_cancelled,
        limits,
        started_at: Instant::now(),
        facts: &facts,
        verify_fingerprints,
    };
    visit_directory(root, 0, &context, &mut result);
    result
        .apps
        .sort_by_cached_key(|app| app.path.to_lowercase());
    result
}

struct VisitContext<'a, F: Fn() -> bool> {
    previous: &'a FilesystemIndex,
    mode: ScanMode,
    excluded: &'a [PathBuf],
    is_cancelled: &'a F,
    limits: ScanLimits,
    started_at: Instant,
    facts: &'a crate::catalog::machine::MachineFacts,
    verify_fingerprints: bool,
}

fn visit_directory<F: Fn() -> bool>(
    directory: &Path,
    depth: usize,
    context: &VisitContext<'_, F>,
    result: &mut IncrementalScanResult,
) {
    if should_stop(result, context.limits, context.started_at)
        || (context.is_cancelled)()
        || !is_scannable_directory(directory)
        || !portable::should_visit_directory(directory, context.excluded)
    {
        return;
    }
    let key = normalized_path(directory);
    let modified_nanos = directory_modified_nanos(directory);
    let cached = context.previous.directories.get(&key);
    let unchanged = context.mode == ScanMode::Incremental
        && cached.is_some_and(|record| {
            record.modified_nanos == modified_nanos
                && cached_children_are_valid(directory, &record.child_directories)
        });

    if unchanged {
        let mut record = cached.expect("checked above").clone();
        if context.verify_fingerprints {
            let (apps, executables) =
                verify_cached_executables(&record, context.facts, &mut result.statistics);
            record.apps = apps;
            record.executables = executables;
        }
        result.apps.extend(record.apps.iter().cloned());
        result.index.directories.insert(key, record.clone());
        for child in record.child_directories {
            if depth >= context.limits.max_depth {
                mark_limit(result, ScanLimit::Depth);
                continue;
            }
            visit_directory(Path::new(&child), depth + 1, context, result);
            if should_stop(result, context.limits, context.started_at) {
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
        if should_stop(result, context.limits, context.started_at) {
            break;
        }
        if result
            .statistics
            .entries_seen
            .is_multiple_of(CANCELLATION_CHECK_INTERVAL)
            && (context.is_cancelled)()
        {
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
            } else if depth >= context.limits.max_depth {
                mark_limit(result, ScanLimit::Depth);
            } else if portable::should_visit_directory(&path, context.excluded) {
                children.push(path.to_string_lossy().into_owned());
            }
        } else if file_type.is_file() && portable::is_portable_candidate(&path) {
            result.statistics.executables_inspected += 1;
            if let Some(app) = portable_app(path, context.facts) {
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
                executables: fingerprints_of(&direct_apps),
                apps: direct_apps,
            },
        );
    }
    for child in children {
        if should_stop(result, context.limits, context.started_at) {
            return;
        }
        visit_directory(Path::new(&child), depth + 1, context, result);
    }
}

fn cached_children_are_valid(parent: &Path, children: &[String]) -> bool {
    let parent = normalized_path(parent);
    children.iter().all(|child| {
        let child = Path::new(child);
        is_scannable_directory(child)
            && child
                .parent()
                .is_some_and(|value| normalized_path(value) == parent)
    })
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

fn fingerprint_key(path: &str) -> String {
    path.to_lowercase()
}

fn read_fingerprint(path: &Path) -> Result<Option<FileFingerprint>, ()> {
    match fs::metadata(path) {
        Ok(metadata) => {
            let modified_nanos = metadata
                .modified()
                .ok()
                .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
                .map_or(0, |duration| duration.as_nanos());
            Ok(Some(FileFingerprint {
                size: metadata.len(),
                modified_nanos,
            }))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(_) => Err(()),
    }
}

fn fingerprints_of(apps: &[AppInfo]) -> BTreeMap<String, FileFingerprint> {
    apps.iter()
        .filter_map(|app| {
            let fingerprint = read_fingerprint(Path::new(&app.path)).ok().flatten()?;
            Some((fingerprint_key(&app.path), fingerprint))
        })
        .collect()
}

fn verify_cached_executables(
    record: &DirectoryRecord,
    facts: &crate::catalog::machine::MachineFacts,
    statistics: &mut ScanStatistics,
) -> (Vec<AppInfo>, BTreeMap<String, FileFingerprint>) {
    let mut apps = Vec::with_capacity(record.apps.len());
    let mut fingerprints = BTreeMap::new();
    for app in &record.apps {
        let key = fingerprint_key(&app.path);
        let path = PathBuf::from(&app.path);
        let Ok(current) = read_fingerprint(&path) else {
            apps.push(app.clone());
            if let Some(stored) = record.executables.get(&key) {
                fingerprints.insert(key, *stored);
            }
            continue;
        };
        let Some(current) = current else {
            continue;
        };
        match record.executables.get(&key) {
            Some(stored) if *stored == current => {
                apps.push(app.clone());
                fingerprints.insert(key, current);
            }
            None => {
                apps.push(app.clone());
                fingerprints.insert(key, current);
            }
            Some(_) => {
                statistics.executables_inspected += 1;
                if let Some(refreshed) = portable_app(path, facts) {
                    apps.push(refreshed);
                }
                fingerprints.insert(key, current);
            }
        }
    }
    (apps, fingerprints)
}

fn directory_modified_nanos(path: &Path) -> u128 {
    fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |duration| duration.as_nanos())
}

#[cfg(test)]
mod fingerprint_tests {
    use super::*;

    fn indexed_editor() -> (tempfile::TempDir, PathBuf, FilesystemIndex) {
        let root = tempfile::tempdir().unwrap();
        let editor = root.path().join("Editor");
        std::fs::create_dir_all(&editor).unwrap();
        let executable = editor.join("Editor.exe");
        std::fs::write(&executable, b"first build").unwrap();

        let scanned = scan_root(
            root.path(),
            &FilesystemIndex::default(),
            ScanMode::Force,
            &[],
            || false,
        );

        assert_eq!(scanned.apps.len(), 1, "the fixture must produce one card");
        (root, executable, scanned.index)
    }

    fn pin_directory_timestamps(index: &mut FilesystemIndex) {
        for (path, record) in &mut index.directories {
            record.modified_nanos = directory_modified_nanos(Path::new(path));
        }
    }

    fn rescan(root: &Path, index: &FilesystemIndex) -> IncrementalScanResult {
        scan_root(root, index, ScanMode::Incremental, &[], || false)
    }

    #[test]
    fn an_executable_rewritten_in_place_is_re_read() {
        let (root, executable, mut index) = indexed_editor();
        std::fs::write(&executable, b"second build, a different size").unwrap();
        pin_directory_timestamps(&mut index);

        let scanned = rescan(root.path(), &index);

        assert_eq!(scanned.apps.len(), 1);
        assert_eq!(
            scanned.statistics.executables_inspected, 1,
            "the changed file must be read again"
        );
        assert_eq!(
            scanned.statistics.directories_enumerated, 0,
            "and the unchanged tree must still not be walked"
        );
    }

    #[test]
    fn an_unchanged_executable_is_not_read_again() {
        let (root, _executable, mut index) = indexed_editor();
        pin_directory_timestamps(&mut index);

        let scanned = rescan(root.path(), &index);

        assert_eq!(scanned.apps.len(), 1);
        assert_eq!(scanned.statistics.executables_inspected, 0);
        assert_eq!(scanned.statistics.directories_enumerated, 0);
    }

    #[test]
    fn a_deleted_executable_drops_only_its_own_record() {
        let (root, executable, _) = indexed_editor();
        let viewer = root.path().join("Viewer");
        std::fs::create_dir_all(&viewer).unwrap();
        std::fs::write(viewer.join("Viewer.exe"), b"another build").unwrap();

        let mut index = scan_root(
            root.path(),
            &FilesystemIndex::default(),
            ScanMode::Force,
            &[],
            || false,
        )
        .index;
        std::fs::remove_file(&executable).unwrap();
        pin_directory_timestamps(&mut index);

        let scanned = rescan(root.path(), &index);

        let remaining = scanned
            .apps
            .iter()
            .map(|app| app.path.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            remaining,
            vec![viewer.join("Viewer.exe").to_string_lossy().as_ref()]
        );
    }

    #[test]
    fn an_index_without_fingerprints_migrates_without_losing_anything() {
        let (root, _executable, mut index) = indexed_editor();
        pin_directory_timestamps(&mut index);
        for record in index.directories.values_mut() {
            record.executables.clear();
        }

        let scanned = rescan(root.path(), &index);

        assert_eq!(scanned.apps.len(), 1);
        assert_eq!(scanned.statistics.executables_inspected, 0);
        assert!(
            scanned
                .index
                .directories
                .values()
                .any(|record| !record.executables.is_empty()),
            "the fingerprints are on file for next time"
        );
    }

    #[test]
    fn an_executable_that_cannot_be_checked_keeps_its_record() {
        let (root, _executable, mut index) = indexed_editor();
        pin_directory_timestamps(&mut index);
        for record in index.directories.values_mut() {
            for app in &mut record.apps {
                app.path = format!("{}\0", app.path);
            }
        }

        let scanned = rescan(root.path(), &index);

        assert_eq!(
            scanned.apps.len(),
            1,
            "an unverifiable executable keeps its card"
        );
    }

    #[test]
    fn an_unchanged_tree_costs_no_enumeration_and_no_metadata_reads() {
        let root = tempfile::tempdir().unwrap();
        for index in 0..50 {
            let folder = root.path().join(format!("Tool{index}"));
            std::fs::create_dir_all(&folder).unwrap();
            std::fs::write(folder.join(format!("Tool{index}.exe")), b"build").unwrap();
        }
        let mut index = scan_root(
            root.path(),
            &FilesystemIndex::default(),
            ScanMode::Force,
            &[],
            || false,
        )
        .index;
        pin_directory_timestamps(&mut index);

        let scanned = rescan(root.path(), &index);

        assert_eq!(scanned.apps.len(), 50);
        assert_eq!(scanned.statistics.directories_enumerated, 0);
        assert_eq!(scanned.statistics.executables_inspected, 0);
        assert_eq!(scanned.statistics.entries_seen, 0);
    }

    #[test]
    fn a_force_scan_ignores_the_index_entirely() {
        let (root, executable, mut index) = indexed_editor();
        std::fs::remove_file(&executable).unwrap();
        pin_directory_timestamps(&mut index);

        let scanned = scan_root(root.path(), &index, ScanMode::Force, &[], || false);

        assert!(scanned.apps.is_empty());
        assert!(scanned.statistics.directories_enumerated > 0);
    }
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
    fn recovers_an_invalid_cached_child_path() {
        let root = tempfile::tempdir().unwrap();
        let downloads = root
            .path()
            .join("\u{417}\u{430}\u{433}\u{440}\u{443}\u{437}\u{43a}\u{438}");
        std::fs::create_dir_all(&downloads).unwrap();
        let app_directory = downloads.join("Tool");
        std::fs::create_dir_all(&app_directory).unwrap();
        let installer = app_directory.join("Tool.exe");
        std::fs::write(&installer, []).unwrap();
        assert!(portable::is_portable_candidate(&installer));
        assert!(
            portable_app(installer, &crate::catalog::machine::MachineFacts::current()).is_some()
        );
        let mut first = scan_root(
            root.path(),
            &FilesystemIndex::default(),
            ScanMode::Force,
            &[],
            || false,
        );
        assert!(first.apps.iter().any(|app| app.path.ends_with("Tool.exe")));
        first
            .index
            .directories
            .get_mut(&normalized_path(root.path()))
            .unwrap()
            .child_directories = vec![root
            .path()
            .join("missing-cache-child")
            .to_string_lossy()
            .into_owned()];

        let second = scan_root(
            root.path(),
            &first.index,
            ScanMode::Incremental,
            &[],
            || false,
        );

        assert!(second.apps.iter().any(|app| app.path.ends_with("Tool.exe")));
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
            true,
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
            true,
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
            true,
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
            true,
        );

        assert_eq!(result.limit_reached, Some(ScanLimit::Time));
    }

    #[test]
    fn recognizes_windows_reparse_point_attribute() {
        assert!(has_reparse_point_attribute(0x400));
        assert!(!has_reparse_point_attribute(0x20));
    }
}
