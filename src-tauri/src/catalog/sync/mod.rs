//! Catalog synchronization core and Tauri-owned orchestration.
//!
//! The pure delta/scanning logic remains callable through this module, while the focused
//! submodules own cache-document loading, background hydration, coordinated scan execution,
//! and filesystem-watcher lifecycle.

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
use crate::catalog::source::{merge_sources, SourceKey, SourceSnapshot};
use crate::catalog::sync::scan_control::ScanControl;
use crate::catalog::{self, AppInfo, ScanProgress};
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

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
    // Each Windows source is checked before it starts and bounded while it runs. An incomplete
    // source is left out of `updates` below so `merge_sources` keeps its previous snapshot —
    // reporting a partial list would delete every application the source did not reach.
    let registry = catalog::scan_registry(&control);
    let start_menu = catalog::scan_start_menu(&control);
    // `None` means PowerShell could not run, not "no Start apps" — see `start_apps::scan`.
    let start_apps = catalog::start_apps::scan(&control);

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
    let installer_cache = catalog::installer_cache::scan_roots(&installer_roots, &installer_budget);
    progress(ScanProgress {
        stage: "Installer caches".into(),
        location: None,
        completed_roots: installer_roots.len(),
        total_roots: installer_roots.len(),
    });

    let libraries = catalog::steam::installed_libraries();
    let mut steam_apps = Vec::new();
    progress(ScanProgress {
        stage: "Steam libraries".into(),
        location: None,
        completed_roots: 0,
        total_roots: libraries.len(),
    });
    for (index, library) in libraries.iter().enumerate() {
        if is_cancelled() {
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
    let portable = portable::scan_roots(
        portable::PortableScanInput {
            previous_apps: previous_portable_apps,
            previous_index: &previous.filesystem_index,
            roots: &roots,
            excluded: &excluded,
            mode,
            max_duration: DEFAULT_MAX_DURATION,
        },
        &progress,
        &is_cancelled,
    );

    // One snapshot per scanner, so a scanner that failed can be left out of `updates` and keep
    // its previous snapshot instead of being replaced by an empty one. Merging all of Windows
    // into a single key meant one transient PowerShell failure deleted every Store application
    // from the catalog.
    let mut updates = vec![
        SourceSnapshot {
            key: SourceKey("steam".into()),
            fingerprint: None,
            apps: steam_apps,
        },
        SourceSnapshot {
            key: SourceKey("portable".into()),
            fingerprint: None,
            apps: portable.apps,
        },
    ];
    if registry.stop.is_none() {
        updates.push(SourceSnapshot {
            key: SourceKey(catalog::source::REGISTRY_SOURCE.into()),
            fingerprint: None,
            apps: registry.apps,
        });
    }
    if start_menu.stop.is_none() {
        updates.push(SourceSnapshot {
            key: SourceKey(catalog::source::START_MENU_SOURCE.into()),
            fingerprint: None,
            apps: start_menu.apps,
        });
    }
    if let Some(apps) = start_apps {
        updates.push(SourceSnapshot {
            key: SourceKey(catalog::source::START_APPS_SOURCE.into()),
            fingerprint: None,
            apps,
        });
    }
    if installer_cache.stop.is_none() {
        updates.push(SourceSnapshot {
            key: SourceKey(catalog::source::INSTALLER_CACHE_SOURCE.into()),
            fingerprint: None,
            apps: installer_cache.apps,
        });
    }
    let merged = merge_sources(previous.sources.clone(), updates);
    let mut apps = merged.apps;
    // Metadata first, deduplication second, exactly once: publisher and install location come
    // from the registry records and are what makes a shortcut and its registered product
    // recognizable as the same application.
    catalog::attach_registry_metadata(&mut apps, &registry.metadata);
    // Drop entries whose launch target no longer exists (a shortcut left by an uninstalled app)
    // before dedup, so phantoms neither merge nor reach the catalog. Filesystem-touching, so it
    // stays here on the scan path rather than in the pure sanitize/dedup passes.
    apps.retain(catalog::target_is_present);
    apps = catalog::sanitize_reported(apps);
    // Console (CLI) executables are command-line tools, not GUI applications — move them to
    // Auxiliary by their PE subsystem. Filesystem-touching, so it stays here on the scan path.
    catalog::demote_console_applications(&mut apps);
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
    };
    let app_details = catalog::details::retain_cached_details(&previous.app_details, &apps);
    CatalogCache {
        schema_version: crate::catalog::cache::CACHE_SCHEMA_VERSION,
        generation: previous.generation.saturating_add(1),
        apps,
        sources: merged.sources,
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

    // Cancellation reaches the Windows sources *before* they start, and — the part that matters —
    // an incomplete source is reported as incomplete rather than as an empty success. Before this,
    // registry and Start Menu ran to completion before the first check and an empty result would
    // have been indistinguishable from "this machine has no installed applications".
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
        // `None` is the "PowerShell did not answer" signal that keeps the stored snapshot.
        assert_eq!(start_apps, None);
    }

    // The snapshot rule that makes the above safe: a source left out of the updates keeps its
    // previous apps, so a cancelled scan cannot delete every Store or Start Menu application.
    #[test]
    fn an_incomplete_source_keeps_its_previous_snapshot() {
        let previous = vec![SourceSnapshot {
            key: SourceKey(catalog::source::START_MENU_SOURCE.into()),
            fingerprint: None,
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
        shortcut.install_location = Some(r"C:\Users\Maks\Desktop".into());
        let mut registry = app("firefox-registry", "Mozilla Firefox (x64 ru)");
        registry.path = r"C:\Program Files\Mozilla Firefox\firefox.exe".into();
        registry.publisher = Some("Mozilla".into());
        registry.install_location = Some(r"C:\Program Files\Mozilla Firefox".into());

        let apps = catalog::sanitize(vec![shortcut, registry]);

        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].name, "Firefox");
    }
}
