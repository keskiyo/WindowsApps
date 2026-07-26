//! Filesystem-change watcher lifecycle. The watcher guard lives in `AppState`; a change fires a
//! non-interactive coordinated scan so the catalog stays fresh without user action.

use super::scan::run_coordinated_scan;
use crate::app_state::AppState;
use crate::catalog;
use crate::catalog::sync::SyncRequest;
use crate::platform::windows::change_watcher;
use std::sync::Arc;
use tauri::Manager;

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
    let watcher = change_watcher::start(paths, callback);
    if let Ok(mut current) = state.change_watcher.lock() {
        *current = watcher;
    };
}
