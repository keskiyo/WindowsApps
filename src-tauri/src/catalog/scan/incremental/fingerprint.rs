use super::model::{DirectoryRecord, FileFingerprint, ScanStatistics};
use crate::catalog::machine::MachineFacts;
use crate::catalog::{portable_app, AppInfo};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

pub(super) fn fingerprint_key(path: &str) -> String {
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

pub(super) fn fingerprints_of(apps: &[AppInfo]) -> BTreeMap<String, FileFingerprint> {
    apps.iter()
        .filter_map(|app| {
            let fingerprint = read_fingerprint(Path::new(&app.path)).ok().flatten()?;
            Some((fingerprint_key(&app.path), fingerprint))
        })
        .collect()
}

pub(super) fn verify_cached_executables(
    record: &DirectoryRecord,
    facts: &MachineFacts,
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
