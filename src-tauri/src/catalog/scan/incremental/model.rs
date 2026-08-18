use crate::catalog::AppInfo;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

pub(crate) const DEFAULT_MAX_DEPTH: usize = 16;
pub(crate) const DEFAULT_MAX_ENTRIES: usize = 500_000;
pub(crate) const DEFAULT_MAX_DURATION: Duration = Duration::from_secs(3 * 60);

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

pub(super) fn should_stop(
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

pub(super) fn mark_limit(result: &mut IncrementalScanResult, limit: ScanLimit) {
    if result.limit_reached.is_none() || limit != ScanLimit::Depth {
        result.limit_reached = Some(limit);
    }
}

pub(super) fn hard_limit_reached(result: &IncrementalScanResult) -> bool {
    matches!(
        result.limit_reached,
        Some(ScanLimit::Entries | ScanLimit::Time)
    )
}
