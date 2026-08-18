use crate::catalog::cache::CatalogCache;
use crate::catalog::incremental::{FilesystemIndex, ScanMode, DEFAULT_MAX_DURATION};
use crate::catalog::scan_settings::ScanSettings;
use crate::catalog::source::{SourceKey, SourceSnapshot};
use crate::catalog::sources::registry::RegistryMetadata;
use crate::catalog::sync::health::SourceOutcome;
use crate::catalog::sync::scan_control::{ScanControl, StageStop};
use crate::catalog::sync::{portable, SyncRequest};
use crate::catalog::{self, ScanProgress};
use std::path::PathBuf;
use std::time::Instant;

pub(super) struct SourceScan {
    pub updates: Vec<SourceSnapshot>,
    pub outcomes: Vec<SourceOutcome>,
    pub registry_metadata: Option<Vec<RegistryMetadata>>,
    pub filesystem_index: Option<FilesystemIndex>,
}

pub(super) fn scan_all(
    previous: &CatalogCache,
    settings: &ScanSettings,
    request: SyncRequest,
    progress: &impl Fn(ScanProgress),
    is_cancelled: &(impl Fn() -> bool + Sync),
) -> SourceScan {
    let control = ScanControl::new(is_cancelled);
    progress(ScanProgress {
        stage: "Windows applications".into(),
        location: None,
        completed_roots: 0,
        total_roots: 0,
    });
    let mut updates = Vec::new();
    let mut outcomes = Vec::new();

    let registry_at = Instant::now();
    let registry = catalog::scan_registry(&control);
    let registry_replaced = registry.stop.is_none() && registry.complete;
    outcomes.push(SourceOutcome {
        key: catalog::source::REGISTRY_SOURCE,
        stop: registry.stop,
        answered: registry.complete,
        replaced: registry_replaced,
        records: registry.apps.len(),
        duration: registry_at.elapsed(),
    });

    let start_menu_at = Instant::now();
    let start_menu = catalog::scan_start_menu(&control);
    let start_menu_replaced = start_menu.stop.is_none() && start_menu.complete;
    outcomes.push(SourceOutcome {
        key: catalog::source::START_MENU_SOURCE,
        stop: start_menu.stop,
        answered: start_menu.complete,
        replaced: start_menu_replaced,
        records: start_menu.apps.len(),
        duration: start_menu_at.elapsed(),
    });

    let start_apps_at = Instant::now();
    let start_apps = catalog::start_apps::scan(&control);
    outcomes.push(SourceOutcome {
        key: catalog::source::START_APPS_SOURCE,
        stop: (start_apps.is_none() && control.is_cancelled()).then_some(StageStop::Cancelled),
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
    let mut steam_complete = true;
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
        let scan = catalog::steam::scan_library(library);
        steam_complete &= scan.complete;
        steam_apps.extend(scan.games.into_iter().map(catalog::steam_app));
        progress(ScanProgress {
            stage: "Steam libraries".into(),
            location: Some(library.to_string_lossy().into_owned()),
            completed_roots: index + 1,
            total_roots: libraries.len(),
        });
    }
    let steam_replaced = !steam_cancelled && steam_complete;
    outcomes.push(SourceOutcome {
        key: "steam",
        stop: steam_cancelled.then_some(StageStop::Cancelled),
        answered: steam_complete,
        replaced: steam_replaced,
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
        progress,
        is_cancelled,
    );
    let portable_replaced = portable.stop.is_none();
    outcomes.push(SourceOutcome {
        key: "portable",
        stop: portable.stop,
        answered: true,
        replaced: portable_replaced,
        records: portable.apps.len(),
        duration: portable_at.elapsed(),
    });

    if steam_replaced {
        updates.push(snapshot("steam", steam_apps));
    }
    if portable_replaced {
        updates.push(snapshot("portable", portable.apps));
    }
    if registry_replaced {
        updates.push(snapshot(catalog::source::REGISTRY_SOURCE, registry.apps));
    }
    if start_menu_replaced {
        updates.push(snapshot(
            catalog::source::START_MENU_SOURCE,
            start_menu.apps,
        ));
    }
    if let Some(apps) = start_apps {
        updates.push(snapshot(catalog::source::START_APPS_SOURCE, apps));
    }
    if installer_cache.stop.is_none() {
        updates.push(snapshot(
            catalog::source::INSTALLER_CACHE_SOURCE,
            installer_cache.apps,
        ));
    }

    SourceScan {
        updates,
        outcomes,
        registry_metadata: registry_replaced.then_some(registry.metadata),
        filesystem_index: portable_replaced.then_some(portable.filesystem_index),
    }
}

fn snapshot(key: &str, apps: Vec<catalog::AppInfo>) -> SourceSnapshot {
    SourceSnapshot {
        key: SourceKey(key.into()),
        fingerprint: None,
        health: None,
        apps,
    }
}
