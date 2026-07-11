// Custom title bar (decorations disabled) needs window drag/min/max/close permissions;
// see src-tauri/capabilities/default.json.
mod catalog;
mod lifecycle;
mod platform;

use catalog::cache::CatalogCache;
use catalog::scan_coordinator::{ScanCoordinator, ScanJob, Submission};
use catalog::sync::{compute_delta, SyncRequest};
use catalog::{cache, AppInfo, LaunchKind, SourceKind, UninstallTarget};
use platform::windows::{autostart, global_shortcut, launcher, uninstall_history, uninstaller};
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};
use tauri::{Emitter, Manager};

#[derive(Clone, Debug)]
struct UninstallRecord {
    app_name: String,
    publisher: Option<String>,
    source_kind: SourceKind,
    target: UninstallTarget,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UninstallPreview {
    app_name: String,
    publisher: Option<String>,
    source: SourceKind,
    mechanism: uninstaller::UninstallMechanism,
    command: String,
}

static UNINSTALL_TARGETS: OnceLock<Mutex<HashMap<String, UninstallRecord>>> = OnceLock::new();
static LAUNCH_TARGETS: OnceLock<Mutex<HashMap<String, (LaunchKind, String)>>> = OnceLock::new();
static SYNC_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
static SCAN_COORDINATOR: OnceLock<ScanCoordinator<Vec<AppInfo>>> = OnceLock::new();
static HYDRATION_QUEUE: OnceLock<Mutex<catalog::hydration::HydrationQueue>> = OnceLock::new();
static CHANGE_WATCHER: OnceLock<Mutex<Option<platform::windows::change_watcher::WatcherGuard>>> =
    OnceLock::new();

fn sync_lock() -> &'static Mutex<()> {
    SYNC_LOCK.get_or_init(|| Mutex::new(()))
}

fn scan_coordinator() -> &'static ScanCoordinator<Vec<AppInfo>> {
    SCAN_COORDINATOR.get_or_init(ScanCoordinator::default)
}

fn hydration_queue() -> &'static Mutex<catalog::hydration::HydrationQueue> {
    HYDRATION_QUEUE.get_or_init(|| Mutex::new(catalog::hydration::HydrationQueue::default()))
}

fn restart_change_watcher(app: tauri::AppHandle, settings: &catalog::scan_settings::ScanSettings) {
    let slot = CHANGE_WATCHER.get_or_init(|| Mutex::new(None));
    let previous = slot.lock().ok().and_then(|mut current| current.take());
    drop(previous);
    let paths = catalog::watcher_paths(settings);
    let callback_handle = app.clone();
    let callback = Arc::new(move || {
        let handle = callback_handle.clone();
        tauri::async_runtime::spawn(async move {
            let blocking_handle = handle.clone();
            let _ = tauri::async_runtime::spawn_blocking(move || {
                run_coordinated_scan(&blocking_handle, SyncRequest::Watch, false)
            })
            .await;
        });
    });
    let watcher = platform::windows::change_watcher::start(paths, callback);
    if let Ok(mut current) = slot.lock() {
        *current = Some(watcher);
    }
}

fn uninstall_targets() -> &'static Mutex<HashMap<String, UninstallRecord>> {
    UNINSTALL_TARGETS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn remember_uninstall_targets(apps: &[AppInfo]) {
    let targets = apps
        .iter()
        .filter_map(|app| {
            app.uninstall.clone().map(|target| {
                (
                    app.id.clone(),
                    UninstallRecord {
                        app_name: app.name.clone(),
                        publisher: app.publisher.clone(),
                        source_kind: app.source_kind,
                        target,
                    },
                )
            })
        })
        .collect();
    if let Ok(mut stored) = uninstall_targets().lock() {
        *stored = targets;
    }
}

fn launch_targets() -> &'static Mutex<HashMap<String, (LaunchKind, String)>> {
    LAUNCH_TARGETS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Record the trusted launch target (kind + path) for every catalog entry, keyed by
/// id. `launch_app` resolves through this map so the webview can only launch apps the
/// scanner actually found — never an arbitrary path supplied over IPC.
fn remember_launch_targets(apps: &[AppInfo]) {
    let targets = apps
        .iter()
        .map(|app| (app.id.clone(), (app.launch_kind, app.path.clone())))
        .collect();
    if let Ok(mut stored) = launch_targets().lock() {
        *stored = targets;
    }
}

fn preview_for(record: &UninstallRecord) -> UninstallPreview {
    let target = uninstaller::preview(&record.target);
    UninstallPreview {
        app_name: record.app_name.clone(),
        publisher: record.publisher.clone(),
        source: record.source_kind,
        mechanism: target.mechanism,
        command: target.command,
    }
}

fn execute_and_record(
    app_data_dir: &Path,
    record: UninstallRecord,
    executor: impl FnOnce(UninstallTarget) -> Result<(), String>,
) -> Result<(), String> {
    let preview = uninstaller::preview(&record.target);
    let result = executor(record.target);
    let history_result = if result.is_ok() {
        uninstall_history::UninstallResult::Succeeded
    } else {
        uninstall_history::UninstallResult::Failed
    };
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs());
    let _ = uninstall_history::append(
        app_data_dir,
        uninstall_history::UninstallHistoryEntry {
            id: String::new(),
            timestamp,
            app_name: record.app_name,
            publisher: record.publisher,
            mechanism: preview.mechanism,
            result: history_result,
        },
    );
    result
}

fn enqueue_hydration(
    app: tauri::AppHandle,
    app_data_dir: std::path::PathBuf,
    generation: u64,
    ids: Vec<String>,
    priority: bool,
) {
    let should_start = hydration_queue()
        .lock()
        .is_ok_and(|mut queue| queue.enqueue(generation, ids, priority));
    if !should_start {
        return;
    }
    tauri::async_runtime::spawn(async move {
        let hydration_dir = app_data_dir.clone();
        let worker_app = app.clone();
        let hydrated = tauri::async_runtime::spawn_blocking(move || {
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
                    let Ok(mut queue) = hydration_queue().lock() else {
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
                if let Ok(mut queue) = hydration_queue().lock() {
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
        let Ok(_guard) = sync_lock().lock() else {
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

fn apply_hydration_patches_to_document(
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
        target.install_location = patch.install_location.clone();
        target.can_uninstall = patch.can_uninstall.unwrap_or(target.can_uninstall);
        if patch.icon_base64.is_some() {
            target.icon_base64 = patch.icon_base64.clone();
        }
    }
}

fn load_sanitized_document(app_data_dir: &Path) -> Option<CatalogCache> {
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

fn load_sanitized_cache(app_data_dir: &Path) -> Option<Vec<AppInfo>> {
    load_sanitized_document(app_data_dir).map(|document| document.apps)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CatalogSnapshot {
    apps: Vec<AppInfo>,
    has_cache: bool,
    generation: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SystemSettings {
    version: &'static str,
    autostart_enabled: bool,
    shortcut: global_shortcut::Status,
    scan_settings: catalog::scan_settings::ScanSettings,
    fixed_drives: Vec<String>,
}

fn normalize_scan_settings(
    settings: catalog::scan_settings::ScanSettings,
) -> Result<catalog::scan_settings::ScanSettings, String> {
    let normalize = |values: Vec<String>| -> Result<Vec<String>, String> {
        let mut normalized = Vec::<String>::new();
        for value in values {
            let value = value.trim().trim_matches('"').to_string();
            if value.is_empty() {
                continue;
            }
            if !Path::new(&value).is_absolute() {
                return Err(format!("Scan path must be an absolute path: {value}"));
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
async fn get_system_settings(app: tauri::AppHandle) -> Result<SystemSettings, String> {
    let autostart_enabled = tauri::async_runtime::spawn_blocking(autostart::is_enabled)
        .await
        .map_err(|error| format!("Startup check was interrupted: {error}"))??;
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Could not open the application data folder: {error}"))?;
    let fixed_drives = platform::windows::drives::fixed_drive_roots();
    Ok(SystemSettings {
        version: env!("CARGO_PKG_VERSION"),
        autostart_enabled,
        shortcut: global_shortcut::status(),
        scan_settings: catalog::scan_settings::read(&app_data_dir),
        fixed_drives: fixed_drives
            .into_iter()
            .map(|path| path.to_string_lossy().into_owned())
            .collect(),
    })
}

#[tauri::command]
async fn set_scan_settings(
    app: tauri::AppHandle,
    settings: catalog::scan_settings::ScanSettings,
) -> Result<catalog::scan_settings::ScanSettings, String> {
    let settings = normalize_scan_settings(settings)?;
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Could not open the application data folder: {error}"))?;
    catalog::scan_settings::write(&app_data_dir, &settings)
        .map_err(|error| format!("Could not save scan settings: {error}"))?;
    restart_change_watcher(app, &settings);
    Ok(settings)
}

#[tauri::command]
fn cancel_scan() {
    scan_coordinator().cancel_all();
}

#[tauri::command]
async fn set_autostart(enabled: bool) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || autostart::set_enabled(enabled))
        .await
        .map_err(|error| format!("Startup update was interrupted: {error}"))?
}

#[tauri::command]
async fn open_telegram() -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(|| launcher::shell_execute("https://t.me/keskiyo"))
        .await
        .map_err(|error| format!("Telegram launch was interrupted: {error}"))?
}

#[tauri::command]
async fn open_github() -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(|| {
        launcher::shell_execute("https://github.com/keskiyo/WindowsApps")
    })
    .await
    .map_err(|error| format!("GitHub launch was interrupted: {error}"))?
}

#[tauri::command]
async fn get_apps(app: tauri::AppHandle) -> Result<CatalogSnapshot, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Could not open the application data folder: {error}"))?;
    let cached = load_sanitized_document(&app_data_dir);
    let has_cache = cached.is_some();
    let document = cached.unwrap_or_default();
    let generation = document.generation;
    let apps = document.apps;
    remember_uninstall_targets(&apps);
    remember_launch_targets(&apps);
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
    })
}

fn synchronize_catalog_once(
    app: &tauri::AppHandle,
    job: &ScanJob<Vec<AppInfo>>,
) -> Result<Vec<AppInfo>, String> {
    let _guard = sync_lock()
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
    remember_uninstall_targets(&document.apps);
    remember_launch_targets(&document.apps);
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

fn run_coordinated_scan(
    app: &tauri::AppHandle,
    request: SyncRequest,
    wants_result: bool,
) -> Result<Option<Vec<AppInfo>>, String> {
    match scan_coordinator().submit(request, wants_result) {
        Submission::Start { job, receiver } => {
            if let Some(receiver) = receiver {
                let result = synchronize_catalog_once(app, &job);
                if let Some(next) = scan_coordinator().complete(job, result) {
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
    loop {
        let result = synchronize_catalog_once(app, &job);
        let Some(next) = scan_coordinator().complete(job, result) else {
            break;
        };
        job = next;
    }
}

#[tauri::command]
async fn refresh_apps(app: tauri::AppHandle) -> Result<Vec<AppInfo>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        run_coordinated_scan(&app, SyncRequest::Refresh, true)?
            .ok_or_else(|| "Application refresh was coalesced unexpectedly".to_string())
    })
    .await
    .map_err(|error| format!("Application scanning was interrupted: {error}"))?
}

#[tauri::command]
async fn force_full_scan(app: tauri::AppHandle) -> Result<Vec<AppInfo>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        run_coordinated_scan(&app, SyncRequest::Force, true)?
            .ok_or_else(|| "Application scan was coalesced unexpectedly".to_string())
    })
    .await
    .map_err(|error| format!("Application scanning was interrupted: {error}"))?
}

#[tauri::command]
async fn reset_catalog_cache(app: tauri::AppHandle) -> Result<Vec<AppInfo>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        scan_coordinator().cancel_all();
        let app_data_dir = app
            .path()
            .app_data_dir()
            .map_err(|error| format!("Could not open the application data folder: {error}"))?;
        cache::reset(&app_data_dir)
            .map_err(|error| format!("Could not reset catalog cache: {error}"))?;
        catalog::icon_cache::clear(&app_data_dir)
            .map_err(|error| format!("Could not reset icon cache: {error}"))?;
        run_coordinated_scan(&app, SyncRequest::Force, true)?
            .ok_or_else(|| "Catalog reset scan was coalesced unexpectedly".to_string())
    })
    .await
    .map_err(|error| format!("Catalog reset was interrupted: {error}"))?
}

#[tauri::command]
async fn hydrate_visible_icons(app: tauri::AppHandle, ids: Vec<String>) -> Result<(), String> {
    if ids.is_empty() {
        return Ok(());
    }
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Could not open the application data folder: {error}"))?;
    let Some(document) = cache::read_document(&app_data_dir) else {
        return Ok(());
    };
    enqueue_hydration(app, app_data_dir, document.generation, ids, true);
    Ok(())
}

#[tauri::command]
fn start_background_sync(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        let handle = app.clone();
        let _ = tauri::async_runtime::spawn_blocking(move || {
            run_coordinated_scan(&handle, SyncRequest::Startup, false)
        })
        .await;
    });
}

#[derive(Clone, Serialize)]
struct LaunchStatusPayload {
    id: String,
    state: &'static str,
}

/// Best-effort readiness: blocks until the launched GUI process finishes its startup and is
/// waiting for input (or the timeout/no-message-queue case returns). Always resolves to
/// "ready" so the UI clears its launching state as soon as any signal arrives; genuine
/// launch failures surface earlier via the `launch_app` error path.
fn wait_for_launch_ready(raw_handle: isize) -> &'static str {
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::System::Threading::WaitForInputIdle;
    let handle = HANDLE(raw_handle as *mut core::ffi::c_void);
    unsafe {
        WaitForInputIdle(handle, 12000);
        let _ = CloseHandle(handle);
    }
    "ready"
}

#[tauri::command]
async fn launch_app(app: tauri::AppHandle, id: String) -> Result<(), String> {
    let (launch_kind, path) = launch_targets()
        .lock()
        .map_err(|_| "Launch data is temporarily unavailable".to_string())?
        .get(&id)
        .cloned()
        .ok_or_else(|| "Application is not available for launch".to_string())?;
    let handle = tauri::async_runtime::spawn_blocking(move || launcher::launch(launch_kind, &path))
        .await
        .map_err(|error| format!("Application launch was interrupted: {error}"))??;
    if let Some(raw_handle) = handle {
        let emitter = app.clone();
        let launch_id = id.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let state = wait_for_launch_ready(raw_handle);
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
async fn get_uninstall_preview(id: String) -> Result<UninstallPreview, String> {
    let record = uninstall_targets()
        .lock()
        .map_err(|_| "Uninstall data is temporarily unavailable".to_string())?
        .get(&id)
        .cloned()
        .ok_or_else(|| "Uninstall is unavailable for this application".to_string())?;
    Ok(preview_for(&record))
}

#[tauri::command]
async fn get_uninstall_history(
    app: tauri::AppHandle,
) -> Result<Vec<uninstall_history::UninstallHistoryEntry>, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Could not open the application data folder: {error}"))?;
    Ok(uninstall_history::read(&app_data_dir))
}

#[tauri::command]
async fn clear_uninstall_history(app: tauri::AppHandle) -> Result<(), String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Could not open the application data folder: {error}"))?;
    uninstall_history::clear(&app_data_dir)
        .map_err(|error| format!("Could not clear uninstall history: {error}"))
}

#[tauri::command]
async fn uninstall_app(app: tauri::AppHandle, id: String) -> Result<(), String> {
    let record = uninstall_targets()
        .lock()
        .map_err(|_| "Uninstall data is temporarily unavailable".to_string())?
        .get(&id)
        .cloned()
        .ok_or_else(|| "Uninstall is unavailable for this application".to_string())?;
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Could not open the application data folder: {error}"))?;
    tauri::async_runtime::spawn_blocking(move || {
        execute_and_record(&app_data_dir, record, |target| {
            uninstaller::execute(Some(target))
        })
    })
    .await
    .map_err(|error| format!("Uninstall launch was interrupted: {error}"))??;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let lifecycle = Arc::new(lifecycle::LifecycleState::default());
    let close_lifecycle = Arc::clone(&lifecycle);
    let tray_lifecycle = Arc::clone(&lifecycle);
    let mut builder = tauri::Builder::default();
    #[cfg(desktop)]
    {
        builder = builder
            .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
                lifecycle::show_main_window(app);
            }))
            .plugin(tauri_plugin_updater::Builder::new().build())
            .plugin(tauri_plugin_process::init());
    }
    builder
        .plugin(tauri_plugin_dialog::init())
        .on_window_event(move |window, event| {
            if window.label() == "main" {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    if close_lifecycle.should_hide_on_close() {
                        api.prevent_close();
                        let _ = window.hide();
                    }
                }
            }
        })
        .setup(move |app| {
            global_shortcut::register(app.handle().clone());
            if let Err(error) = lifecycle::setup_tray(app.handle(), Arc::clone(&tray_lifecycle)) {
                log::error!("Could not create the system tray: {error}");
            }
            if let Ok(app_data_dir) = app.path().app_data_dir() {
                if let Some(apps) = load_sanitized_cache(&app_data_dir) {
                    remember_uninstall_targets(&apps);
                    remember_launch_targets(&apps);
                }
                let settings = catalog::scan_settings::read(&app_data_dir);
                restart_change_watcher(app.handle().clone(), &settings);
            }
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_apps,
            refresh_apps,
            force_full_scan,
            reset_catalog_cache,
            hydrate_visible_icons,
            start_background_sync,
            cancel_scan,
            launch_app,
            get_uninstall_preview,
            uninstall_app,
            get_uninstall_history,
            clear_uninstall_history,
            get_system_settings,
            set_autostart,
            set_scan_settings,
            open_telegram,
            open_github
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;
    use catalog::{AppCategory, SourceKind};

    fn cached_app(name: &str, path: &str) -> AppInfo {
        AppInfo {
            id: path.into(),
            name: name.into(),
            path: path.into(),
            icon_base64: None,
            category: AppCategory::Other,
            launch_kind: LaunchKind::Executable,
            source_kind: SourceKind::Registry,
            description: None,
            version: None,
            publisher: None,
            install_location: None,
            can_uninstall: false,
            uninstall: None,
            resolved_path: None,
            shortcut_icon_path: None,
        }
    }

    #[test]
    fn launch_targets_only_resolve_known_catalog_ids() {
        let mut app = cached_app("Visual Studio Code", r"C:\Code.exe");
        app.id = "code".into();
        app.launch_kind = LaunchKind::Executable;
        remember_launch_targets(&[app]);
        let stored = launch_targets().lock().unwrap();
        assert_eq!(
            stored.get("code").cloned(),
            Some((LaunchKind::Executable, r"C:\Code.exe".to_string()))
        );
        assert!(stored.get("unknown-id").is_none());
    }

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

    fn uninstall_record(name: &str) -> UninstallRecord {
        UninstallRecord {
            app_name: name.into(),
            publisher: Some("Publisher".into()),
            source_kind: SourceKind::Registry,
            target: UninstallTarget::Command {
                executable: "uninstall.exe".into(),
                arguments: "/quiet".into(),
            },
        }
    }

    #[test]
    fn records_successful_uninstall_without_command_details() {
        let dir = tempfile::tempdir().unwrap();
        execute_and_record(dir.path(), uninstall_record("Editor"), |_| Ok(())).unwrap();

        let history = uninstall_history::read(dir.path());
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].app_name, "Editor");
        assert_eq!(
            history[0].result,
            uninstall_history::UninstallResult::Succeeded
        );
        let serialized = serde_json::to_value(&history[0]).unwrap();
        assert!(serialized.get("command").is_none());
        assert!(serialized.get("path").is_none());
        assert!(serialized.get("error").is_none());
    }

    #[test]
    fn records_failed_uninstall_and_returns_original_error() {
        let dir = tempfile::tempdir().unwrap();
        let error = execute_and_record(dir.path(), uninstall_record("Editor"), |_| {
            Err("boom".into())
        })
        .unwrap_err();

        let history = uninstall_history::read(dir.path());
        assert_eq!(error, "boom");
        assert_eq!(
            history[0].result,
            uninstall_history::UninstallResult::Failed
        );
    }
}
