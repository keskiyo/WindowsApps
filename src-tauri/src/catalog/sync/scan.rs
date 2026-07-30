//! Coordinated scanning: the single `sync_lock`-guarded synchronization pass plus the coordinator
//! plumbing (`run_coordinated_scan`/`process_scan_chain`) that serializes and chains scan jobs.

use super::document::load_sanitized_document;
use super::hydration::enqueue_hydration;
use crate::app_state::{remember_catalog, AppState};
use crate::catalog::cache;
use crate::catalog::scan_coordinator::{ScanJob, Submission};
use crate::catalog::sync::{compute_delta, SyncRequest};
use crate::catalog::{self, AppInfo};
use crate::error::AppError;
use tauri::{Emitter, Manager};

fn synchronize_catalog_once(
    app: &tauri::AppHandle,
    job: &ScanJob<Vec<AppInfo>>,
) -> Result<Vec<AppInfo>, AppError> {
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
        return Err(AppError::ScanCancelled);
    }
    let delta = compute_delta(document.generation, &previous.apps, &document.apps);
    remember_catalog(state.inner(), &document.apps);
    cache::write_document(&app_data_dir, &document)
        .map_err(|error| format!("Could not save the application cache: {error}"))?;
    // Once per scan: applications that left the catalog would otherwise keep their cached icon
    // on disk forever, since `write_icon` only ever cleans up after the app it is writing.
    let live_ids = document
        .apps
        .iter()
        .map(|app| app.id.clone())
        .collect::<Vec<_>>();
    catalog::icon_cache::retain_only(&app_data_dir, &live_ids);
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
) -> Result<Option<Vec<AppInfo>>, AppError> {
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

fn process_scan_chain(app: &tauri::AppHandle, mut job: ScanJob<Vec<AppInfo>>) {
    let state = app.state::<AppState>();
    loop {
        let result = synchronize_catalog_once(app, &job);
        let Some(next) = state.scan_coordinator.complete(job, result) else {
            break;
        };
        job = next;
    }
}
