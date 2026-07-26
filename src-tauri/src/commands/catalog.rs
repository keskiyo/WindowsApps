use super::run_blocking;
use crate::app_state::{remember_launch_targets, remember_uninstall_targets, AppState};
use crate::catalog::sync::SyncRequest;
use crate::catalog::{self, cache, AppInfo};
use crate::catalog_sync::{enqueue_hydration, load_sanitized_document, run_coordinated_scan};
use crate::error::AppError;
use serde::Serialize;
use tauri::Manager;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CatalogSnapshot {
    apps: Vec<AppInfo>,
    has_cache: bool,
    generation: u64,
    diagnostics: Option<cache::CatalogDiagnostics>,
}

#[tauri::command]
pub(crate) async fn get_apps(app: tauri::AppHandle) -> Result<CatalogSnapshot, AppError> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| AppError::AppDataDir(error.to_string()))?;
    let cache_dir = app_data_dir.clone();
    let cached = run_blocking("Catalog cache read", move || {
        load_sanitized_document(&cache_dir)
    })
    .await?;
    let has_cache = cached.is_some();
    let document = cached.unwrap_or_default();
    let generation = document.generation;
    let diagnostics = document.diagnostics.clone();
    let apps = document.apps;
    {
        let state = app.state::<AppState>();
        remember_uninstall_targets(state.inner(), &apps);
        remember_launch_targets(state.inner(), &apps);
    }
    enqueue_hydration(
        app.clone(),
        app_data_dir,
        generation,
        apps.iter().map(|app| app.id.clone()).collect(),
        false,
    );
    Ok(CatalogSnapshot {
        has_cache,
        apps,
        generation,
        diagnostics,
    })
}

#[tauri::command]
pub(crate) async fn refresh_apps(app: tauri::AppHandle) -> Result<Vec<AppInfo>, AppError> {
    tauri::async_runtime::spawn_blocking(move || {
        run_coordinated_scan(&app, SyncRequest::Refresh, true)?.ok_or(AppError::Coalesced {
            what: "Application refresh",
        })
    })
    .await
    .map_err(|error| AppError::Interrupted {
        context: "Application scanning",
        source: error.to_string(),
    })?
}

#[tauri::command]
pub(crate) async fn force_full_scan(app: tauri::AppHandle) -> Result<Vec<AppInfo>, AppError> {
    tauri::async_runtime::spawn_blocking(move || {
        run_coordinated_scan(&app, SyncRequest::Force, true)?.ok_or(AppError::Coalesced {
            what: "Application scan",
        })
    })
    .await
    .map_err(|error| AppError::Interrupted {
        context: "Application scanning",
        source: error.to_string(),
    })?
}

#[tauri::command]
pub(crate) async fn reset_catalog_cache(app: tauri::AppHandle) -> Result<Vec<AppInfo>, AppError> {
    tauri::async_runtime::spawn_blocking(move || {
        let app_data_dir = app
            .path()
            .app_data_dir()
            .map_err(|error| AppError::AppDataDir(error.to_string()))?;
        {
            let state = app.state::<AppState>();
            state.scan_coordinator.cancel_all();
            // `cancel_all` only raises the flag. Acquiring the cache lock waits for any scan
            // still finishing to release it, so the files are deleted with no scan mid-write.
            // The guard is dropped here — the fresh scan below takes the same lock.
            let _guard = state.sync_lock.lock().map_err(|_| {
                AppError::ResetCatalogCache("catalog synchronization is unavailable".into())
            })?;
            cache::reset(&app_data_dir)
                .map_err(|error| AppError::ResetCatalogCache(error.to_string()))?;
            catalog::icon_cache::clear(&app_data_dir)
                .map_err(|error| AppError::ResetIconCache(error.to_string()))?;
        }
        run_coordinated_scan(&app, SyncRequest::Force, true)?.ok_or(AppError::Coalesced {
            what: "Catalog reset scan",
        })
    })
    .await
    .map_err(|error| AppError::Interrupted {
        context: "Catalog reset",
        source: error.to_string(),
    })?
}

#[tauri::command]
pub(crate) async fn clear_icon_cache(app: tauri::AppHandle) -> Result<(), AppError> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| AppError::AppDataDir(error.to_string()))?;
    run_blocking("Icon cache clear", move || {
        catalog::icon_cache::clear(&app_data_dir)
    })
    .await?
    .map_err(|error| AppError::ClearIconCache(error.to_string()))
}

#[tauri::command]
pub(crate) async fn hydrate_visible_icons(
    app: tauri::AppHandle,
    ids: Vec<String>,
) -> Result<(), AppError> {
    if ids.is_empty() {
        return Ok(());
    }
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| AppError::AppDataDir(error.to_string()))?;
    let cache_dir = app_data_dir.clone();
    let Some(document) = run_blocking("Catalog cache read", move || {
        cache::read_document(&cache_dir)
    })
    .await?
    else {
        return Ok(());
    };
    enqueue_hydration(app, app_data_dir, document.generation, ids, true);
    Ok(())
}

#[tauri::command]
pub(crate) fn start_background_sync(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        let handle = app.clone();
        let _ = tauri::async_runtime::spawn_blocking(move || {
            run_coordinated_scan(&handle, SyncRequest::Startup, false)
        })
        .await;
    });
}
