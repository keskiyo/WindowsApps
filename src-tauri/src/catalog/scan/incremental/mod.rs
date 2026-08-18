mod fingerprint;
mod model;
mod walk;

pub(crate) use model::{
    FilesystemIndex, IncrementalScanResult, ScanLimit, ScanLimits, ScanMode, ScanStatistics,
    DEFAULT_MAX_DURATION,
};

#[cfg(test)]
pub(crate) use model::DirectoryRecord;

use crate::catalog::machine::MachineFacts;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use walk::{visit_directory, VisitContext};

#[cfg(test)]
use crate::catalog::{portable, portable_app};
#[cfg(test)]
use walk::{directory_modified_nanos, normalized_path};

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
    let facts = MachineFacts::current();
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
}
