use super::fingerprint::{fingerprints_of, verify_cached_executables};
use super::model::{
    hard_limit_reached, mark_limit, should_stop, DirectoryRecord, FilesystemIndex,
    IncrementalScanResult, ScanLimit, ScanLimits, ScanMode,
};
use crate::catalog::machine::MachineFacts;
use crate::catalog::{portable, portable_app};
use std::fs;
use std::os::windows::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::time::{Instant, UNIX_EPOCH};

const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
const CANCELLATION_CHECK_INTERVAL: usize = 128;

pub(super) struct VisitContext<'a, F: Fn() -> bool> {
    pub(super) previous: &'a FilesystemIndex,
    pub(super) mode: ScanMode,
    pub(super) excluded: &'a [PathBuf],
    pub(super) is_cancelled: &'a F,
    pub(super) limits: ScanLimits,
    pub(super) started_at: Instant,
    pub(super) facts: &'a MachineFacts,
    pub(super) verify_fingerprints: bool,
}

pub(super) fn visit_directory<F: Fn() -> bool>(
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

pub(super) fn normalized_path(path: &Path) -> String {
    path.to_string_lossy().to_lowercase()
}

pub(super) fn directory_modified_nanos(path: &Path) -> u128 {
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
    fn recognizes_windows_reparse_point_attribute() {
        assert!(has_reparse_point_attribute(
            FILE_ATTRIBUTE_REPARSE_POINT | 0x10
        ));
        assert!(!has_reparse_point_attribute(0x10));
    }
}
