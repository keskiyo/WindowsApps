mod document;
mod hydration;
mod portable;
mod scan;
pub(crate) mod scan_control;
mod watcher;

pub(crate) use document::{load_sanitized_cache, load_sanitized_document};
pub(crate) use hydration::enqueue_hydration;
pub(crate) use scan::run_coordinated_scan;
pub(crate) use watcher::restart_change_watcher;

use crate::catalog::cache::{CatalogCache, CatalogDiagnostics};
use crate::catalog::incremental::{ScanMode, DEFAULT_MAX_DURATION};
use crate::catalog::scan_settings::ScanSettings;
use crate::catalog::source::{
    apply_health, merge_sources, SourceErrorKind, SourceHealth, SourceHealthState, SourceKey,
    SourceSnapshot,
};
use crate::catalog::sync::scan_control::ScanControl;
use crate::catalog::{self, AppInfo, ScanProgress};
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum SyncRequest {
    Watch,
    Startup,
    Refresh,
    Force,
}

impl SyncRequest {
    pub(crate) fn is_interactive(self) -> bool {
        matches!(self, Self::Refresh | Self::Force)
    }

    fn label(self) -> &'static str {
        match self {
            Self::Watch => "watch",
            Self::Startup => "startup",
            Self::Refresh => "refresh",
            Self::Force => "force",
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CatalogChangeSummary {
    pub added: usize,
    pub removed: usize,
    pub updated: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CatalogDelta {
    pub generation: u64,
    pub upserted: Vec<AppInfo>,
    pub removed_ids: Vec<String>,
    pub summary: CatalogChangeSummary,
}

pub(crate) fn compute_delta(
    generation: u64,
    previous: &[AppInfo],
    current: &[AppInfo],
) -> CatalogDelta {
    let old = previous
        .iter()
        .map(|app| (app.id.as_str(), app))
        .collect::<HashMap<_, _>>();
    let new = current
        .iter()
        .map(|app| (app.id.as_str(), app))
        .collect::<HashMap<_, _>>();
    let mut upserted = current
        .iter()
        .filter(|app| old.get(app.id.as_str()).is_none_or(|old| *old != *app))
        .cloned()
        .collect::<Vec<_>>();
    let mut removed_ids = previous
        .iter()
        .filter(|app| !new.contains_key(app.id.as_str()))
        .map(|app| app.id.clone())
        .collect::<Vec<_>>();
    upserted.sort_by_cached_key(|app| app.id.clone());
    removed_ids.sort();
    let added = upserted
        .iter()
        .filter(|app| !old.contains_key(app.id.as_str()))
        .count();
    let updated = upserted.len().saturating_sub(added);
    CatalogDelta {
        generation,
        upserted,
        summary: CatalogChangeSummary {
            added,
            removed: removed_ids.len(),
            updated,
        },
        removed_ids,
    }
}

struct SourceOutcome {
    key: &'static str,
    stop: Option<scan_control::StageStop>,
    answered: bool,
    replaced: bool,
    records: usize,
    duration: Duration,
}

fn seconds_since_epoch() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map_or(0, |duration| duration.as_secs())
}

fn source_health(
    previous: &[SourceSnapshot],
    outcome: SourceOutcome,
    attempted_at: u64,
) -> SourceHealth {
    let key = SourceKey(outcome.key.to_string());
    let completed = outcome.answered && outcome.stop.is_none();
    let cancelled = outcome.stop == Some(scan_control::StageStop::Cancelled);
    let last_success_at = catalog::source::previous_success(previous, &key);
    let failures = catalog::source::previous_failures(previous, &key);
    let served = previous
        .iter()
        .find(|snapshot| snapshot.key == key)
        .map_or(0, |snapshot| snapshot.apps.len());

    let state = if completed {
        SourceHealthState::Fresh
    } else if last_success_at.is_none() {
        SourceHealthState::FailedWithoutSnapshot
    } else if outcome.stop.is_some() {
        SourceHealthState::Incomplete
    } else {
        SourceHealthState::Stale
    };

    SourceHealth {
        state,
        last_attempt_at: Some(attempted_at),
        last_success_at: if completed {
            Some(attempted_at)
        } else {
            last_success_at
        },
        consecutive_failures: match (completed, cancelled) {
            (true, _) => 0,
            (false, true) => failures,
            (false, false) => failures.saturating_add(1),
        },
        last_duration_ms: u64::try_from(outcome.duration.as_millis()).ok(),
        last_error: if completed {
            None
        } else {
            outcome
                .stop
                .map(SourceErrorKind::from)
                .or(Some(SourceErrorKind::ProviderFailed))
        },
        record_count: if outcome.replaced {
            outcome.records
        } else {
            served
        },
        key,
    }
}

pub(crate) fn synchronize(
    previous: &CatalogCache,
    settings: &ScanSettings,
    request: SyncRequest,
    progress: impl Fn(ScanProgress),
    is_cancelled: impl Fn() -> bool + Sync,
) -> CatalogCache {
    let started_at = Instant::now();
    let control = ScanControl::new(&is_cancelled);
    progress(ScanProgress {
        stage: "Windows applications".into(),
        location: None,
        completed_roots: 0,
        total_roots: 0,
    });
    let attempted_at = seconds_since_epoch();
    let mut outcomes = Vec::new();

    let registry_at = Instant::now();
    let registry = catalog::scan_registry(&control);
    outcomes.push(SourceOutcome {
        key: catalog::source::REGISTRY_SOURCE,
        stop: registry.stop,
        answered: true,
        replaced: registry.stop.is_none(),
        records: registry.apps.len(),
        duration: registry_at.elapsed(),
    });

    let start_menu_at = Instant::now();
    let start_menu = catalog::scan_start_menu(&control);
    outcomes.push(SourceOutcome {
        key: catalog::source::START_MENU_SOURCE,
        stop: start_menu.stop,
        answered: true,
        replaced: start_menu.stop.is_none(),
        records: start_menu.apps.len(),
        duration: start_menu_at.elapsed(),
    });

    let start_apps_at = Instant::now();
    let start_apps = catalog::start_apps::scan(&control);
    outcomes.push(SourceOutcome {
        key: catalog::source::START_APPS_SOURCE,
        stop: (start_apps.is_none() && control.is_cancelled())
            .then_some(scan_control::StageStop::Cancelled),
        answered: start_apps.is_some(),
        replaced: start_apps.is_some(),
        records: start_apps.as_ref().map_or(0, Vec::len),
        duration: start_apps_at.elapsed(),
    });

    let installer_roots = catalog::installer_cache::roots();
    progress(ScanProgress {
        stage: "Installer caches".into(),
        location: None,
        completed_roots: 0,
        total_roots: installer_roots.len(),
    });
    let installer_budget = control.stage_with(
        catalog::installer_cache::MAX_DURATION,
        catalog::installer_cache::MAX_ENTRIES,
        catalog::installer_cache::MAX_DEPTH,
    );
    let installer_at = Instant::now();
    let installer_cache = catalog::installer_cache::scan_roots(&installer_roots, &installer_budget);
    outcomes.push(SourceOutcome {
        key: catalog::source::INSTALLER_CACHE_SOURCE,
        stop: installer_cache.stop,
        answered: true,
        replaced: installer_cache.stop.is_none(),
        records: installer_cache.apps.len(),
        duration: installer_at.elapsed(),
    });
    progress(ScanProgress {
        stage: "Installer caches".into(),
        location: None,
        completed_roots: installer_roots.len(),
        total_roots: installer_roots.len(),
    });

    let steam_at = Instant::now();
    let libraries = catalog::steam::installed_libraries();
    let mut steam_apps = Vec::new();
    let mut steam_cancelled = false;
    progress(ScanProgress {
        stage: "Steam libraries".into(),
        location: None,
        completed_roots: 0,
        total_roots: libraries.len(),
    });
    for (index, library) in libraries.iter().enumerate() {
        if is_cancelled() {
            steam_cancelled = true;
            break;
        }
        steam_apps.extend(
            catalog::steam::scan_library(library)
                .into_iter()
                .map(catalog::steam_app),
        );
        progress(ScanProgress {
            stage: "Steam libraries".into(),
            location: Some(library.to_string_lossy().into_owned()),
            completed_roots: index + 1,
            total_roots: libraries.len(),
        });
    }
    outcomes.push(SourceOutcome {
        key: "steam",
        stop: steam_cancelled.then_some(scan_control::StageStop::Cancelled),
        answered: true,
        replaced: true,
        records: steam_apps.len(),
        duration: steam_at.elapsed(),
    });

    let mut roots = if settings.auto_scan_fixed_drives {
        crate::platform::windows::drives::fixed_drive_roots()
    } else {
        Vec::new()
    };
    roots.extend(
        settings
            .included_paths
            .iter()
            .map(PathBuf::from)
            .filter(|path| path.is_dir()),
    );
    roots.sort_by_cached_key(|path| path.to_string_lossy().to_lowercase());
    roots.dedup_by(|left, right| {
        left.to_string_lossy()
            .eq_ignore_ascii_case(&right.to_string_lossy())
    });
    let mut excluded = catalog::default_portable_exclusions();
    excluded.extend(settings.excluded_paths.iter().map(PathBuf::from));
    excluded.extend(libraries);
    let mode = if request == SyncRequest::Force {
        ScanMode::Force
    } else {
        ScanMode::Incremental
    };
    let previous_portable_apps = previous
        .sources
        .iter()
        .find(|snapshot| snapshot.key.0 == "portable")
        .map(|snapshot| snapshot.apps.as_slice())
        .unwrap_or_default();
    let portable_at = Instant::now();
    let portable = portable::scan_roots(
        portable::PortableScanInput {
            previous_apps: previous_portable_apps,
            previous_index: &previous.filesystem_index,
            roots: &roots,
            excluded: &excluded,
            mode,
            max_duration: DEFAULT_MAX_DURATION,
            verify_fingerprints: settings.catalog_portable_fingerprint_v1,
        },
        &progress,
        &is_cancelled,
    );
    outcomes.push(SourceOutcome {
        key: "portable",
        stop: if portable.complete {
            None
        } else if is_cancelled() {
            Some(scan_control::StageStop::Cancelled)
        } else {
            Some(scan_control::StageStop::EntryLimit)
        },
        answered: true,
        replaced: true,
        records: portable.apps.len(),
        duration: portable_at.elapsed(),
    });

    let mut updates = vec![
        SourceSnapshot {
            key: SourceKey("steam".into()),
            fingerprint: None,
            health: None,
            apps: steam_apps,
        },
        SourceSnapshot {
            key: SourceKey("portable".into()),
            fingerprint: None,
            health: None,
            apps: portable.apps,
        },
    ];
    if registry.stop.is_none() {
        updates.push(SourceSnapshot {
            key: SourceKey(catalog::source::REGISTRY_SOURCE.into()),
            fingerprint: None,
            health: None,
            apps: registry.apps,
        });
    }
    if start_menu.stop.is_none() {
        updates.push(SourceSnapshot {
            key: SourceKey(catalog::source::START_MENU_SOURCE.into()),
            fingerprint: None,
            health: None,
            apps: start_menu.apps,
        });
    }
    if let Some(apps) = start_apps {
        updates.push(SourceSnapshot {
            key: SourceKey(catalog::source::START_APPS_SOURCE.into()),
            fingerprint: None,
            health: None,
            apps,
        });
    }
    if installer_cache.stop.is_none() {
        updates.push(SourceSnapshot {
            key: SourceKey(catalog::source::INSTALLER_CACHE_SOURCE.into()),
            fingerprint: None,
            health: None,
            apps: installer_cache.apps,
        });
    }
    let health = outcomes
        .into_iter()
        .map(|outcome| source_health(&previous.sources, outcome, attempted_at))
        .collect::<Vec<_>>();
    let merged = merge_sources(previous.sources.clone(), updates);
    let mut sources = merged.sources;
    apply_health(&mut sources, health.clone());
    let mut apps = merged.apps;
    catalog::attach_registry_metadata(&mut apps, &registry.metadata);
    let target_availability = catalog::retain_present_targets(&mut apps, settings);
    apps = catalog::sanitize_reported(apps);
    catalog::demote_console_applications(&mut apps);
    catalog::attach_category_reasons(&mut apps);
    catalog::attach_close_risk(&mut apps);
    for app in &mut apps {
        app.icon_base64 = None;
    }
    let completed_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map_or(0, |duration| duration.as_secs());
    let delta = compute_delta(previous.generation.saturating_add(1), &previous.apps, &apps);
    let mut source_counts = std::collections::BTreeMap::new();
    let mut visibility_counts = std::collections::BTreeMap::new();
    for app in &apps {
        *source_counts
            .entry(format!("{:?}", app.source_kind).to_lowercase())
            .or_insert(0) += 1;
        *visibility_counts
            .entry(format!("{:?}", app.visibility_class).to_lowercase())
            .or_insert(0) += 1;
    }
    let diagnostics = CatalogDiagnostics {
        completed_at,
        duration_ms: started_at
            .elapsed()
            .as_millis()
            .try_into()
            .unwrap_or(u64::MAX),
        mode: request.label().into(),
        total_apps: apps.len(),
        source_counts,
        visibility_counts,
        added: delta.summary.added,
        removed: delta.summary.removed,
        updated: delta.summary.updated,
        sources: health,
        target_availability,
    };
    let app_details = catalog::details::retain_cached_details(&previous.app_details, &apps);
    CatalogCache {
        schema_version: crate::catalog::cache::CACHE_SCHEMA_VERSION,
        generation: previous.generation.saturating_add(1),
        apps,
        sources,
        filesystem_index: portable.filesystem_index,
        last_successful_sync: Some(completed_at),
        diagnostics: Some(diagnostics),
        app_details,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::sync::scan_control::StageStop;
    use crate::catalog::StartMenuScan;
    use crate::catalog::{AppCategory, AppInfo, LaunchKind, SourceKind};

    const NOW: u64 = 1_700_000_000;

    fn outcome(stop: Option<StageStop>, answered: bool, records: usize) -> SourceOutcome {
        SourceOutcome {
            key: "start-apps",
            stop,
            answered,
            replaced: answered && stop.is_none(),
            records,
            duration: Duration::from_millis(12),
        }
    }

    fn served(apps: usize, health: Option<SourceHealth>) -> Vec<SourceSnapshot> {
        vec![SourceSnapshot {
            key: SourceKey("start-apps".into()),
            fingerprint: None,
            apps: (0..apps)
                .map(|index| app(&format!("app-{index}"), "Served"))
                .collect(),
            health,
        }]
    }

    fn healthy(last_success_at: u64, failures: u32) -> SourceHealth {
        SourceHealth {
            key: SourceKey("start-apps".into()),
            state: SourceHealthState::Fresh,
            last_attempt_at: Some(last_success_at),
            last_success_at: Some(last_success_at),
            consecutive_failures: failures,
            last_duration_ms: Some(5),
            last_error: None,
            record_count: 3,
        }
    }

    #[test]
    fn a_completed_attempt_is_fresh_and_clears_the_failure_streak() {
        let previous = served(3, Some(healthy(NOW - 500, 4)));

        let health = source_health(&previous, outcome(None, true, 7), NOW);

        assert_eq!(health.state, SourceHealthState::Fresh);
        assert_eq!(health.last_success_at, Some(NOW));
        assert_eq!(health.consecutive_failures, 0);
        assert_eq!(health.last_error, None);
        assert_eq!(health.record_count, 7);
    }

    #[test]
    fn an_empty_successful_result_is_not_a_failure() {
        let previous = served(3, Some(healthy(NOW - 500, 0)));

        let health = source_health(&previous, outcome(None, true, 0), NOW);

        assert_eq!(health.state, SourceHealthState::Fresh);
        assert_eq!(health.record_count, 0);
        assert_eq!(health.last_error, None);
    }

    #[test]
    fn a_provider_that_did_not_answer_keeps_serving_and_reports_stale() {
        let previous = served(3, Some(healthy(NOW - 500, 1)));

        let health = source_health(&previous, outcome(None, false, 0), NOW);

        assert_eq!(health.state, SourceHealthState::Stale);
        assert_eq!(health.last_success_at, Some(NOW - 500));
        assert_eq!(health.consecutive_failures, 2);
        assert_eq!(health.last_error, Some(SourceErrorKind::ProviderFailed));
        assert_eq!(health.record_count, 3, "it still serves what it had");
    }

    #[test]
    fn a_stage_that_stopped_early_reports_incomplete_with_its_reason() {
        let previous = served(3, Some(healthy(NOW - 500, 0)));

        let health = source_health(&previous, outcome(Some(StageStop::TimedOut), true, 1), NOW);

        assert_eq!(health.state, SourceHealthState::Incomplete);
        assert_eq!(health.last_error, Some(SourceErrorKind::TimedOut));
        assert_eq!(health.record_count, 3);
    }

    #[test]
    fn cancellation_does_not_count_as_a_failure() {
        let previous = served(3, Some(healthy(NOW - 500, 2)));

        let health = source_health(&previous, outcome(Some(StageStop::Cancelled), true, 0), NOW);

        assert_eq!(health.consecutive_failures, 2);
        assert_eq!(health.last_error, Some(SourceErrorKind::Cancelled));
    }

    #[test]
    fn a_source_that_never_succeeded_is_distinguishable_from_a_stale_one() {
        let health = source_health(&[], outcome(None, false, 0), NOW);

        assert_eq!(health.state, SourceHealthState::FailedWithoutSnapshot);
        assert_eq!(health.last_success_at, None);
        assert_eq!(health.consecutive_failures, 1);
    }

    fn app(id: &str, name: &str) -> AppInfo {
        AppInfo {
            id: id.into(),
            name: name.into(),
            path: format!(r"C:\{name}.exe"),
            icon_base64: None,
            artifact_kind: Default::default(),
            category: AppCategory::Other,
            launch_kind: LaunchKind::Executable,
            source_kind: SourceKind::Registry,
            description: None,
            version: None,
            publisher: None,
            product_name: None,
            original_filename: None,
            install_location: None,
            can_uninstall: false,
            uninstall: None,
            resolved_path: None,
            shortcut_icon_path: None,
            launch_arguments: None,
            canonical_identity: None,
            preference_identity: None,
            visibility_class: Default::default(),
            visibility_score: 0,
            visibility_reasons: Vec::new(),
            target_availability: None,
            category_reasons: Vec::new(),
            close_risk: None,
        }
    }

    #[test]
    fn computes_stable_id_delta_and_summary() {
        let old = vec![app("removed", "Old"), app("same", "Editor")];
        let mut changed = app("same", "Editor");
        changed.version = Some("2".into());
        let new = vec![changed, app("added", "New")];

        let delta = compute_delta(4, &old, &new);

        assert_eq!(delta.generation, 4);
        assert_eq!(delta.removed_ids, vec!["removed"]);
        assert_eq!(delta.upserted.len(), 2);
        assert_eq!(delta.summary.added, 1);
        assert_eq!(delta.summary.removed, 1);
        assert_eq!(delta.summary.updated, 1);
    }

    #[test]
    fn a_cancelled_scan_reports_windows_sources_as_incomplete_not_empty() {
        let cancelled = || true;
        let control = ScanControl::new(&cancelled);

        let registry = catalog::scan_registry(&control);
        let start_menu = catalog::scan_start_menu(&control);
        let start_apps = catalog::start_apps::scan(&control);

        assert!(registry.apps.is_empty());
        assert_eq!(registry.stop, Some(StageStop::Cancelled));
        assert!(start_menu.apps.is_empty());
        assert_eq!(start_menu.stop, Some(StageStop::Cancelled));
        assert_eq!(start_apps, None);
    }

    #[test]
    fn an_incomplete_source_keeps_its_previous_snapshot() {
        let previous = vec![SourceSnapshot {
            key: SourceKey(catalog::source::START_MENU_SOURCE.into()),
            fingerprint: None,
            health: None,
            apps: vec![app("shortcut", "Editor")],
        }];
        let incomplete = StartMenuScan {
            apps: Vec::new(),
            stop: Some(StageStop::TimedOut),
        };

        let mut updates = Vec::new();
        if incomplete.stop.is_none() {
            updates.push(SourceSnapshot {
                key: SourceKey(catalog::source::START_MENU_SOURCE.into()),
                fingerprint: None,
                health: None,
                apps: incomplete.apps,
            });
        }
        let merged = merge_sources(previous, updates);

        assert_eq!(merged.apps.len(), 1);
        assert_eq!(merged.apps[0].name, "Editor");
    }

    #[test]
    fn post_metadata_sanitize_collapses_shortcut_registry_duplicates() {
        let mut shortcut = app("firefox-shortcut", "Firefox");
        shortcut.path = r"C:\Menu\Firefox.lnk".into();
        shortcut.launch_kind = LaunchKind::Shortcut;
        shortcut.source_kind = SourceKind::StartMenu;
        shortcut.resolved_path = Some(r"C:\Program Files\Mozilla Firefox\firefox.exe".into());
        shortcut.publisher = Some("Mozilla Foundation".into());
        shortcut.install_location = Some(r"C:\Users\Example\Desktop".into());
        let mut registry = app("firefox-registry", "Mozilla Firefox (x64 ru)");
        registry.path = r"C:\Program Files\Mozilla Firefox\firefox.exe".into();
        registry.publisher = Some("Mozilla".into());
        registry.install_location = Some(r"C:\Program Files\Mozilla Firefox".into());

        let apps = catalog::sanitize(vec![shortcut, registry]);

        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].name, "Firefox");
    }
}
