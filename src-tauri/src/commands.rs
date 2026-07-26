//! Tauri IPC command surface. Every command resolves real launch/uninstall targets
//! and filesystem paths server-side; the webview only ever passes catalog ids.

use crate::app_state::{
    execute_and_record, preview_for, remember_launch_targets, remember_uninstall_targets, AppState,
    UninstallPreview,
};
use crate::catalog::sync::SyncRequest;
use crate::catalog::{self, cache, AppInfo};
use crate::catalog_sync::{
    enqueue_hydration, load_sanitized_document, restart_change_watcher, run_coordinated_scan,
};
use crate::error::AppError;
use crate::platform::windows::{
    autostart, exec_target, global_shortcut, install_registry, launcher, uninstall_history,
    uninstaller,
};
use serde::Serialize;
use std::path::Path;
use tauri::{Emitter, Manager};

async fn run_blocking<T, F>(context: &'static str, work: F) -> Result<T, AppError>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|error| AppError::Interrupted {
            context,
            source: error.to_string(),
        })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CatalogSnapshot {
    apps: Vec<AppInfo>,
    has_cache: bool,
    generation: u64,
    diagnostics: Option<cache::CatalogDiagnostics>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SystemSettings {
    version: &'static str,
    autostart_enabled: bool,
    shortcut: global_shortcut::Status,
    scan_settings: catalog::scan_settings::ScanSettings,
    fixed_drives: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StaleCopy {
    installed_version: String,
    install_location: String,
}

#[derive(Clone, Serialize)]
pub(crate) struct LaunchStatusPayload {
    id: String,
    state: &'static str,
}

fn normalize_scan_settings(
    settings: catalog::scan_settings::ScanSettings,
) -> Result<catalog::scan_settings::ScanSettings, AppError> {
    let normalize = |values: Vec<String>| -> Result<Vec<String>, AppError> {
        let mut normalized = Vec::<String>::new();
        for value in values {
            let value = value.trim().trim_matches('"').to_string();
            if value.is_empty() {
                continue;
            }
            if !Path::new(&value).is_absolute() {
                return Err(AppError::ScanPathNotAbsolute(value));
            }
            if !normalized
                .iter()
                .any(|existing| existing.eq_ignore_ascii_case(&value))
            {
                normalized.push(value);
            }
        }
        Ok(normalized)
    };
    Ok(catalog::scan_settings::ScanSettings {
        auto_scan_fixed_drives: settings.auto_scan_fixed_drives,
        included_paths: normalize(settings.included_paths)?,
        excluded_paths: normalize(settings.excluded_paths)?,
    })
}

#[tauri::command]
pub(crate) async fn get_system_settings(app: tauri::AppHandle) -> Result<SystemSettings, AppError> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| AppError::AppDataDir(error.to_string()))?;
    let (autostart_enabled, scan_settings, fixed_drives) =
        run_blocking("System settings read", move || {
            let autostart_enabled = autostart::is_enabled()?;
            let scan_settings = catalog::scan_settings::read(&app_data_dir);
            let fixed_drives = crate::platform::windows::drives::fixed_drive_roots();
            Ok::<_, String>((autostart_enabled, scan_settings, fixed_drives))
        })
        .await??;
    let shortcut = app
        .state::<AppState>()
        .shortcut_status
        .lock()
        .map(|status| status.clone())
        .unwrap_or_default();
    Ok(SystemSettings {
        version: env!("CARGO_PKG_VERSION"),
        autostart_enabled,
        shortcut,
        scan_settings,
        fixed_drives: fixed_drives
            .into_iter()
            .map(|path| path.to_string_lossy().into_owned())
            .collect(),
    })
}

#[tauri::command]
pub(crate) async fn set_scan_settings(
    app: tauri::AppHandle,
    settings: catalog::scan_settings::ScanSettings,
) -> Result<catalog::scan_settings::ScanSettings, AppError> {
    let settings = normalize_scan_settings(settings)?;
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| AppError::AppDataDir(error.to_string()))?;
    let watcher_settings = settings.clone();
    run_blocking("Scan settings update", move || {
        catalog::scan_settings::write(&app_data_dir, &watcher_settings)
            .map_err(|error| AppError::SaveScanSettings(error.to_string()))?;
        restart_change_watcher(app, &watcher_settings);
        Ok::<_, AppError>(())
    })
    .await??;
    Ok(settings)
}

#[tauri::command]
pub(crate) fn cancel_scan(state: tauri::State<'_, AppState>) {
    state.scan_coordinator.cancel_all();
}

#[tauri::command]
pub(crate) async fn set_autostart(enabled: bool) -> Result<(), AppError> {
    tauri::async_runtime::spawn_blocking(move || autostart::set_enabled(enabled))
        .await
        .map_err(|error| AppError::Interrupted {
            context: "Startup update",
            source: error.to_string(),
        })?
        .map_err(AppError::from)
}

#[tauri::command]
pub(crate) async fn open_telegram() -> Result<(), AppError> {
    tauri::async_runtime::spawn_blocking(|| launcher::shell_execute("https://t.me/keskiyo"))
        .await
        .map_err(|error| AppError::Interrupted {
            context: "Telegram launch",
            source: error.to_string(),
        })?
        .map_err(AppError::from)
}

/// Reports when this process is an outdated leftover copy: the uninstall registry says a
/// newer version is installed in a different directory (e.g. an update landed elsewhere).
#[tauri::command]
pub(crate) fn stale_copy_status(app: tauri::AppHandle) -> Option<StaleCopy> {
    let product = app.config().product_name.clone()?;
    install_registry::stale_copy_info(&product).map(|info| StaleCopy {
        installed_version: info.installed_version,
        install_location: info.install_location,
    })
}

/// Launches the registered (newer) installed copy and exits this outdated one. The target
/// path comes from the registry, not from the webview.
#[tauri::command]
pub(crate) fn open_installed_copy(app: tauri::AppHandle) -> Result<(), AppError> {
    let product = app
        .config()
        .product_name
        .clone()
        .ok_or(AppError::ProductNameMissing)?;
    let info = install_registry::stale_copy_info(&product).ok_or(AppError::NoNewerCopy)?;
    let binary = std::env::current_exe()
        .ok()
        .and_then(|path| path.file_name().map(|name| name.to_os_string()))
        .unwrap_or_else(|| "app.exe".into());
    let target = std::path::Path::new(&info.install_location).join(binary);
    // `InstallLocation` comes from HKCU, which any process running as the user can write.
    // Validate it exactly like an uninstaller target before handing it to the shell, so a
    // poisoned key cannot turn this button into "run an arbitrary executable".
    let target = exec_target::validate_executable_path(&target.to_string_lossy())
        .map_err(|_| AppError::LaunchUnavailable)?;
    launcher::shell_execute(&target.to_string_lossy())?;
    // Give the invoke response a moment to reach the webview before exiting.
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(500));
        app.exit(0);
    });
    Ok(())
}

#[tauri::command]
pub(crate) async fn open_github() -> Result<(), AppError> {
    tauri::async_runtime::spawn_blocking(|| {
        launcher::shell_execute("https://github.com/keskiyo/WindowsApps")
    })
    .await
    .map_err(|error| AppError::Interrupted {
        context: "GitHub launch",
        source: error.to_string(),
    })?
    .map_err(AppError::from)
}

#[tauri::command]
pub(crate) async fn open_release(version: String) -> Result<(), AppError> {
    if version.is_empty()
        || !version
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '.' | '-'))
    {
        return Err(AppError::InvalidReleaseVersion);
    }
    tauri::async_runtime::spawn_blocking(move || {
        launcher::shell_execute(&format!(
            "https://github.com/keskiyo/WindowsApps/releases/tag/v{version}"
        ))
    })
    .await
    .map_err(|error| AppError::Interrupted {
        context: "Release notes launch",
        source: error.to_string(),
    })?
    .map_err(AppError::from)
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

/// Best-effort readiness: blocks until the launched GUI process finishes its startup and is
/// waiting for input (or the timeout/no-message-queue case returns). Always resolves to
/// "ready" so the UI clears its launching state as soon as any signal arrives; genuine
/// launch failures surface earlier via the `launch_app` error path.
fn wait_for_launch_ready(handle: &launcher::OwnedProcessHandle) -> &'static str {
    launcher::wait_for_input_idle(handle, 12000);
    "ready"
}

#[tauri::command]
pub(crate) async fn launch_app(app: tauri::AppHandle, id: String) -> Result<(), AppError> {
    let (launch_kind, path, launch_waits) = {
        let state = app.state::<AppState>();
        let stored = state
            .launch_targets
            .lock()
            .map_err(|_| AppError::LaunchDataUnavailable)?;
        let (kind, path) = stored
            .get(&id)
            .cloned()
            .ok_or(AppError::LaunchUnavailable)?;
        (kind, path, state.launch_waits.clone())
    };
    let handle = tauri::async_runtime::spawn_blocking(move || launcher::launch(launch_kind, &path))
        .await
        .map_err(|error| AppError::Interrupted {
            context: "Application launch",
            source: error.to_string(),
        })??;
    if let Some(handle) = handle {
        let emitter = app.clone();
        let launch_id = id.clone();
        // Held for the whole wait; when no slot is free the process handle is still closed and
        // the UI falls back to its ceiling timer.
        let Some(permit) = launch_waits.acquire() else {
            return Ok(());
        };
        tauri::async_runtime::spawn_blocking(move || {
            let _permit = permit;
            let state = wait_for_launch_ready(&handle);
            let _ = emitter.emit(
                "launch://status",
                LaunchStatusPayload {
                    id: launch_id,
                    state,
                },
            );
        });
    }
    Ok(())
}

#[tauri::command]
pub(crate) async fn get_uninstall_preview(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<UninstallPreview, AppError> {
    let record = {
        let stored = state
            .uninstall_targets
            .lock()
            .map_err(|_| AppError::UninstallDataUnavailable)?;
        stored
            .get(&id)
            .cloned()
            .ok_or(AppError::UninstallUnavailable)?
    };
    Ok(preview_for(&record))
}

#[tauri::command]
pub(crate) async fn get_uninstall_history(
    app: tauri::AppHandle,
) -> Result<Vec<uninstall_history::UninstallHistoryEntry>, AppError> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| AppError::AppDataDir(error.to_string()))?;
    run_blocking("Uninstall history read", move || {
        uninstall_history::read(&app_data_dir)
    })
    .await
}

#[tauri::command]
pub(crate) async fn clear_uninstall_history(app: tauri::AppHandle) -> Result<(), AppError> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| AppError::AppDataDir(error.to_string()))?;
    run_blocking("Uninstall history clear", move || {
        uninstall_history::clear(&app_data_dir)
    })
    .await?
    .map_err(|error| AppError::ClearUninstallHistory(error.to_string()))
}

#[tauri::command]
pub(crate) async fn uninstall_app(app: tauri::AppHandle, id: String) -> Result<(), AppError> {
    let record = {
        let state = app.state::<AppState>();
        let stored = state
            .uninstall_targets
            .lock()
            .map_err(|_| AppError::UninstallDataUnavailable)?;
        stored
            .get(&id)
            .cloned()
            .ok_or(AppError::UninstallUnavailable)?
    };
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| AppError::AppDataDir(error.to_string()))?;
    tauri::async_runtime::spawn_blocking(move || {
        execute_and_record(&app_data_dir, record, |target| {
            uninstaller::execute(Some(target))
        })
    })
    .await
    .map_err(|error| AppError::Interrupted {
        context: "Uninstall launch",
        source: error.to_string(),
    })??;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocking_adapter_runs_work_off_the_calling_thread() {
        let calling_thread = std::thread::current().id();
        let worker_thread = tauri::async_runtime::block_on(run_blocking("Test operation", || {
            std::thread::current().id()
        }))
        .unwrap();

        assert_ne!(calling_thread, worker_thread);
    }

    #[test]
    fn normalizes_scan_paths_and_accepts_any_absolute_location() {
        let settings = catalog::scan_settings::ScanSettings {
            auto_scan_fixed_drives: true,
            included_paths: vec![
                r"D:\Portable".into(),
                r"d:\portable".into(),
                r"F:\Stick\Tools".into(),
            ],
            excluded_paths: vec![r"\\Server\Share".into()],
        };
        let normalized = normalize_scan_settings(settings).unwrap();
        assert_eq!(
            normalized.included_paths,
            vec![r"D:\Portable", r"F:\Stick\Tools"]
        );
        assert_eq!(normalized.excluded_paths, vec![r"\\Server\Share"]);

        let invalid = catalog::scan_settings::ScanSettings {
            auto_scan_fixed_drives: true,
            included_paths: vec![r"relative\path".into()],
            excluded_paths: Vec::new(),
        };
        assert!(normalize_scan_settings(invalid).is_err());
    }
}
