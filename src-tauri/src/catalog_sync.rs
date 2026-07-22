//! Catalog synchronization glue: cache sanitization, background icon/metadata
//! hydration, scan coalescing, and the filesystem-change watcher. All of these run
//! against the shared [`AppState`] resolved from a Tauri `AppHandle`.

use crate::app_state::{remember_launch_targets, remember_uninstall_targets, AppState};
use crate::catalog::cache::{self, CatalogCache};
use crate::catalog::scan_coordinator::{ScanJob, Submission};
use crate::catalog::sync::{compute_delta, SyncRequest};
use crate::catalog::{self, AppInfo};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tauri::{Emitter, Manager};

pub(crate) fn restart_change_watcher(
    app: tauri::AppHandle,
    settings: &catalog::scan_settings::ScanSettings,
) {
    let state = app.state::<AppState>();
    let previous = state
        .change_watcher
        .lock()
        .ok()
        .and_then(|mut current| current.take());
    drop(previous);
    let paths = catalog::watcher_paths(settings);
    let callback_handle = app.clone();
    let callback = Arc::new(move || {
        let handle = callback_handle.clone();
        tauri::async_runtime::spawn(async move {
            let _ = tauri::async_runtime::spawn_blocking(move || {
                run_coordinated_scan(&handle, SyncRequest::Watch, false)
            })
            .await;
        });
    });
    let watcher = crate::platform::windows::change_watcher::start(paths, callback);
    if let Ok(mut current) = state.change_watcher.lock() {
        *current = Some(watcher);
    };
}

pub(crate) fn enqueue_hydration(
    app: tauri::AppHandle,
    app_data_dir: std::path::PathBuf,
    generation: u64,
    ids: Vec<String>,
    priority: bool,
) {
    let should_start = {
        let state = app.state::<AppState>();
        state
            .hydration_queue
            .lock()
            .is_ok_and(|mut queue| queue.enqueue(generation, ids, priority))
    };
    if !should_start {
        return;
    }
    tauri::async_runtime::spawn(async move {
        let hydration_dir = app_data_dir.clone();
        let worker_app = app.clone();
        let hydrated = tauri::async_runtime::spawn_blocking(move || {
            let state = worker_app.state::<AppState>();
            let Some(document) = cache::read_document(&hydration_dir) else {
                return Vec::new();
            };
            if document.generation != generation {
                return Vec::new();
            }
            let apps = document
                .apps
                .into_iter()
                .map(|app| (app.id.clone(), app))
                .collect::<HashMap<_, _>>();
            // Patches are emitted in batches rather than one event per icon: the frontend
            // rebuilds its whole app list per `catalog://patches` event, so ~N events for N
            // apps caused O(N^2) work and main-thread jank (cards flickering in one-by-one,
            // delayed hover animations). Batching collapses ~N events into ~N/BATCH.
            const BATCH: usize = 24;
            let mut patches = Vec::new();
            let mut batch: Vec<catalog::hydration::AppHydrationPatch> = Vec::new();
            loop {
                let id = {
                    let Ok(mut queue) = state.hydration_queue.lock() else {
                        break;
                    };
                    let id = queue.pop(generation);
                    if id.is_none() {
                        queue.finish(generation);
                    }
                    id
                };
                let Some(id) = id else {
                    break;
                };
                if let Some(app_info) = apps.get(&id) {
                    let patch =
                        catalog::hydration::hydrate_one(&hydration_dir, app_info, generation);
                    batch.push(patch.clone());
                    patches.push(patch);
                    if batch.len() >= BATCH {
                        let _ = worker_app.emit("catalog://patches", &batch);
                        batch.clear();
                    }
                }
                if let Ok(mut queue) = state.hydration_queue.lock() {
                    queue.complete(generation, &id);
                }
            }
            if !batch.is_empty() {
                let _ = worker_app.emit("catalog://patches", &batch);
            }
            patches
        })
        .await;
        let Ok(patches) = hydrated else {
            return;
        };
        if patches.is_empty() {
            return;
        }
        let post_state = app.state::<AppState>();
        let Ok(_guard) = post_state.sync_lock.lock() else {
            return;
        };
        let Some(mut document) = cache::read_document(&app_data_dir) else {
            return;
        };
        if document.generation != generation {
            return;
        }
        apply_hydration_patches_to_document(&mut document, patches);
        let _ = cache::write_document(&app_data_dir, &document);
    });
}

pub(crate) fn apply_hydration_patches_to_document(
    document: &mut CatalogCache,
    patches: Vec<catalog::hydration::AppHydrationPatch>,
) {
    let patches = patches
        .into_iter()
        .map(|patch| (patch.id.clone(), patch))
        .collect::<HashMap<_, _>>();
    for target in &mut document.apps {
        let Some(patch) = patches.get(&target.id) else {
            continue;
        };
        target.description = patch.description.clone();
        target.version = patch.version.clone();
        target.publisher = patch.publisher.clone();
        target.product_name = patch.product_name.clone();
        target.original_filename = patch.original_filename.clone();
        target.install_location = patch.install_location.clone();
        target.can_uninstall = patch.can_uninstall.unwrap_or(target.can_uninstall);
        if patch.icon_base64.is_some() {
            target.icon_base64 = patch.icon_base64.clone();
        }
    }
}

pub(crate) fn load_sanitized_document(app_data_dir: &Path) -> Option<CatalogCache> {
    let mut document = cache::read_document(app_data_dir)?;
    let original = document.apps.clone();
    document.apps = catalog::sanitize(document.apps);
    for app in &mut document.apps {
        app.can_uninstall = app.uninstall.is_some();
        app.icon_base64 = None;
    }
    if document.apps != original {
        let _ = cache::write_document(app_data_dir, &document);
    }
    Some(document)
}

pub(crate) fn load_sanitized_cache(app_data_dir: &Path) -> Option<Vec<AppInfo>> {
    load_sanitized_document(app_data_dir).map(|document| document.apps)
}

pub(crate) fn synchronize_catalog_once(
    app: &tauri::AppHandle,
    job: &ScanJob<Vec<AppInfo>>,
) -> Result<Vec<AppInfo>, String> {
    let state = app.state::<AppState>();
    let _guard = state
        .sync_lock
        .lock()
        .map_err(|_| "Application synchronization is temporarily unavailable".to_string())?;
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Could not open the application data folder: {error}"))?;
    let previous = load_sanitized_document(&app_data_dir).unwrap_or_default();
    let settings = catalog::scan_settings::read(&app_data_dir);
    let document = catalog::sync::synchronize(
        &previous,
        &settings,
        job.request,
        |progress| {
            let _ = app.emit("scan://progress", progress);
        },
        || job.cancelled.load(std::sync::atomic::Ordering::Relaxed),
    );
    if job.cancelled.load(std::sync::atomic::Ordering::Relaxed) {
        return Err("Application scan cancelled".into());
    }
    let delta = compute_delta(document.generation, &previous.apps, &document.apps);
    remember_uninstall_targets(state.inner(), &document.apps);
    remember_launch_targets(state.inner(), &document.apps);
    cache::write_document(&app_data_dir, &document)
        .map_err(|error| format!("Could not save the application cache: {error}"))?;
    if delta.summary.added + delta.summary.removed + delta.summary.updated > 0 {
        let _ = app.emit("catalog://delta", &delta);
        let _ = app.emit("catalog://changed", &delta.summary);
    }
    // Background (filesystem-watch) syncs must not replace the whole catalog on the
    // frontend — that wipes loaded icons and re-renders the entire grid (jank). They
    // ship only the incremental delta + patches. Interactive Refresh/Force, which show a
    // loading state, still send the full list.
    if job.request.is_interactive() {
        let _ = app.emit("apps://updated", &document.apps);
    }
    // Hydrate every app on first/interactive sync (icons may be on-disk cached), but only
    // the changed apps on a watch sync — avoids re-hydrating the whole catalog repeatedly.
    let hydration_ids = if job.request == SyncRequest::Watch {
        delta.upserted.iter().map(|app| app.id.clone()).collect()
    } else {
        document.apps.iter().map(|app| app.id.clone()).collect()
    };
    enqueue_hydration(
        app.clone(),
        app_data_dir,
        document.generation,
        hydration_ids,
        false,
    );
    Ok(document.apps)
}

pub(crate) fn run_coordinated_scan(
    app: &tauri::AppHandle,
    request: SyncRequest,
    wants_result: bool,
) -> Result<Option<Vec<AppInfo>>, String> {
    let state = app.state::<AppState>();
    let coordinator = &state.scan_coordinator;
    match coordinator.submit(request, wants_result) {
        Submission::Start { job, receiver } => {
            if let Some(receiver) = receiver {
                let result = synchronize_catalog_once(app, &job);
                if let Some(next) = coordinator.complete(job, result) {
                    let handle = app.clone();
                    tauri::async_runtime::spawn_blocking(move || {
                        process_scan_chain(&handle, next);
                    });
                }
                receiver
                    .recv()
                    .map_err(|_| "Application scan result was interrupted".to_string())?
                    .map(Some)
            } else {
                process_scan_chain(app, job);
                Ok(None)
            }
        }
        Submission::Wait(receiver) => receiver
            .recv()
            .map_err(|_| "Application scan result was interrupted".to_string())?
            .map(Some),
        Submission::Coalesced => Ok(None),
    }
}

pub(crate) fn process_scan_chain(app: &tauri::AppHandle, mut job: ScanJob<Vec<AppInfo>>) {
    let state = app.state::<AppState>();
    loop {
        let result = synchronize_catalog_once(app, &job);
        let Some(next) = state.scan_coordinator.complete(job, result) else {
            break;
        };
        job = next;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_state::cached_app;

    #[test]
    fn loads_and_persists_a_sanitized_cache() {
        let dir = tempfile::tempdir().unwrap();
        cache::write_document(
            dir.path(),
            &CatalogCache {
                apps: vec![
                    cached_app(
                        "Visual Studio Installer",
                        "Microsoft.VisualStudio.Installer",
                    ),
                    cached_app("Visual Studio Code", r"C:\Code.exe"),
                ],
                ..CatalogCache::default()
            },
        )
        .unwrap();
        let apps = load_sanitized_cache(dir.path()).unwrap();
        assert_eq!(apps.len(), 1);
        assert_eq!(cache::read_document(dir.path()).unwrap().apps.len(), 1);
    }

    #[test]
    fn disables_stale_uninstall_flag_without_a_cached_target() {
        let dir = tempfile::tempdir().unwrap();
        let mut app = cached_app("Editor", r"C:\Editor.exe");
        app.can_uninstall = true;
        cache::write_document(
            dir.path(),
            &CatalogCache {
                apps: vec![app],
                ..CatalogCache::default()
            },
        )
        .unwrap();
        let apps = load_sanitized_cache(dir.path()).unwrap();
        assert!(!apps[0].can_uninstall);
    }

    #[test]
    fn hydration_patches_persist_icons_without_erasing_existing_icons() {
        let mut first = cached_app("Code", r"C:\Code.exe");
        first.id = "code".into();
        let mut second = cached_app("Claude", r"C:\Claude.exe");
        second.id = "claude".into();
        second.icon_base64 = Some("data:image/png;base64,old".into());
        let mut document = CatalogCache {
            apps: vec![first, second],
            ..CatalogCache::default()
        };

        apply_hydration_patches_to_document(
            &mut document,
            vec![
                catalog::hydration::AppHydrationPatch {
                    id: "code".into(),
                    generation: 1,
                    icon_base64: Some("data:image/png;base64,new".into()),
                    description: None,
                    version: None,
                    publisher: None,
                    product_name: None,
                    original_filename: None,
                    install_location: None,
                    can_uninstall: None,
                },
                catalog::hydration::AppHydrationPatch {
                    id: "claude".into(),
                    generation: 1,
                    icon_base64: None,
                    description: None,
                    version: None,
                    publisher: None,
                    product_name: None,
                    original_filename: None,
                    install_location: None,
                    can_uninstall: None,
                },
            ],
        );

        assert_eq!(
            document.apps[0].icon_base64.as_deref(),
            Some("data:image/png;base64,new")
        );
        assert_eq!(
            document.apps[1].icon_base64.as_deref(),
            Some("data:image/png;base64,old")
        );
    }
}
