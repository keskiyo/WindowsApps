mod catalog;
mod lifecycle;
mod platform;

use catalog::cache::CatalogCache;
use catalog::sync::{compute_delta, SyncRequest};
use catalog::{cache, AppInfo, LaunchKind, SourceKind, UninstallTarget};
use platform::windows::{autostart, global_shortcut, launcher, uninstall_history, uninstaller};
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex, OnceLock,
};
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
static SCAN_CANCELLED: AtomicBool = AtomicBool::new(false);
static SYNC_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
static CHANGE_WATCHER: OnceLock<Mutex<Option<platform::windows::change_watcher::WatcherGuard>>> =
    OnceLock::new();

fn sync_lock() -> &'static Mutex<()> {
    SYNC_LOCK.get_or_init(|| Mutex::new(()))
}

fn restart_change_watcher(app: tauri::AppHandle, settings: &catalog::scan_settings::ScanSettings) {
    let paths = catalog::watcher_paths(settings);
    let callback_handle = app.clone();
    let callback = Arc::new(move || {
        let handle = callback_handle.clone();
        tauri::async_runtime::spawn(async move {
            let blocking_handle = handle.clone();
            let _ = tauri::async_runtime::spawn_blocking(move || {
                synchronize_catalog(&blocking_handle, SyncRequest::Watch)
            })
            .await;
        });
    });
    let watcher = platform::windows::change_watcher::start(paths, callback);
    if let Ok(mut current) = CHANGE_WATCHER.get_or_init(|| Mutex::new(None)).lock() {
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

fn spawn_hydration(
    app: tauri::AppHandle,
    app_data_dir: std::path::PathBuf,
    apps: Vec<AppInfo>,
    generation: u64,
) {
    tauri::async_runtime::spawn(async move {
        let hydration_dir = app_data_dir.clone();
        let hydrated = tauri::async_runtime::spawn_blocking(move || {
            catalog::hydration::hydrate(&hydration_dir, &apps, generation)
        })
        .await;
        let Ok(patches) = hydrated else {
            return;
        };
        for batch in catalog::hydration::batch_patches(patches.clone(), 32) {
            let current = cache::read_document(&app_data_dir)
                .is_some_and(|document| document.generation == generation);
            if !current {
                return;
            }
            let _ = app.emit("catalog://patches", batch);
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
        for patch in patches {
            let Some(target) = document.apps.iter_mut().find(|app| app.id == patch.id) else {
                continue;
            };
            target.description = patch.description;
            target.version = patch.version;
            target.publisher = patch.publisher;
            target.install_location = patch.install_location;
            target.can_uninstall = patch.can_uninstall.unwrap_or(target.can_uninstall);
        }
        let _ = cache::write_document(&app_data_dir, &document);
    });
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

fn prepare_cached_apps(
    original: Vec<AppInfo>,
    enrich_uninstall: impl FnOnce(&mut [AppInfo]),
) -> Vec<AppInfo> {
    let mut apps = catalog::sanitize(original);
    enrich_uninstall(&mut apps);
    for app in &mut apps {
        app.can_uninstall = app.uninstall.is_some();
    }
    apps
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
    SCAN_CANCELLED.store(true, Ordering::Relaxed);
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
    spawn_hydration(app.clone(), app_data_dir, apps.clone(), generation);
    Ok(CatalogSnapshot {
        has_cache,
        apps,
        generation,
    })
}

fn synchronize_catalog(
    app: &tauri::AppHandle,
    request: SyncRequest,
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
    SCAN_CANCELLED.store(false, Ordering::Relaxed);
    let document = catalog::sync::synchronize(
        &previous,
        &settings,
        request,
        |progress| {
            let _ = app.emit("scan://progress", progress);
        },
        || SCAN_CANCELLED.load(Ordering::Relaxed),
    );
    if SCAN_CANCELLED.load(Ordering::Relaxed) {
        return Err("Application scan cancelled".into());
    }
    let delta = compute_delta(document.generation, &previous.apps, &document.apps);
    remember_uninstall_targets(&document.apps);
    cache::write_document(&app_data_dir, &document)
        .map_err(|error| format!("Could not save the application cache: {error}"))?;
    if delta.summary.added + delta.summary.removed + delta.summary.updated > 0 {
        let _ = app.emit("catalog://delta", &delta);
        let _ = app.emit("catalog://changed", &delta.summary);
    }
    let _ = app.emit("apps://updated", &document.apps);
    spawn_hydration(
        app.clone(),
        app_data_dir,
        document.apps.clone(),
        document.generation,
    );
    Ok(document.apps)
}

#[tauri::command]
async fn refresh_apps(app: tauri::AppHandle) -> Result<Vec<AppInfo>, String> {
    tauri::async_runtime::spawn_blocking(move || synchronize_catalog(&app, SyncRequest::Refresh))
        .await
        .map_err(|error| format!("Application scanning was interrupted: {error}"))?
}

#[tauri::command]
async fn force_full_scan(app: tauri::AppHandle) -> Result<Vec<AppInfo>, String> {
    tauri::async_runtime::spawn_blocking(move || synchronize_catalog(&app, SyncRequest::Force))
        .await
        .map_err(|error| format!("Application scanning was interrupted: {error}"))?
}

#[tauri::command]
async fn reset_catalog_cache(app: tauri::AppHandle) -> Result<Vec<AppInfo>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let app_data_dir = app
            .path()
            .app_data_dir()
            .map_err(|error| format!("Could not open the application data folder: {error}"))?;
        cache::reset(&app_data_dir)
            .map_err(|error| format!("Could not reset catalog cache: {error}"))?;
        catalog::icon_cache::clear(&app_data_dir)
            .map_err(|error| format!("Could not reset icon cache: {error}"))?;
        synchronize_catalog(&app, SyncRequest::Force)
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
    let generation = document.generation;
    let apps = catalog::hydration::prioritize_apps(document.apps, &ids);
    spawn_hydration(app, app_data_dir, apps, generation);
    Ok(())
}

#[tauri::command]
fn start_background_sync(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        let handle = app.clone();
        let _ = tauri::async_runtime::spawn_blocking(move || {
            synchronize_catalog(&handle, SyncRequest::Startup)
        })
        .await;
    });
}

#[tauri::command]
async fn launch_app(launch_kind: LaunchKind, path: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || launcher::launch(launch_kind, &path))
        .await
        .map_err(|error| format!("Application launch was interrupted: {error}"))?
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
    tauri::Builder::default()
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
            open_telegram
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
    fn loads_and_persists_a_sanitized_cache() {
        let dir = tempfile::tempdir().unwrap();
        cache::write_cache(
            dir.path(),
            &[
                cached_app(
                    "Visual Studio Installer",
                    "Microsoft.VisualStudio.Installer",
                ),
                cached_app("Visual Studio Code", r"C:\Code.exe"),
            ],
        )
        .unwrap();
        let apps = load_sanitized_cache(dir.path()).unwrap();
        assert_eq!(apps.len(), 1);
        assert_eq!(cache::read_cache(dir.path()).unwrap().len(), 1);
    }

    #[test]
    fn disables_stale_uninstall_flag_without_a_cached_target() {
        let dir = tempfile::tempdir().unwrap();
        let mut app = cached_app("Editor", r"C:\Editor.exe");
        app.can_uninstall = true;
        cache::write_cache(dir.path(), &[app]).unwrap();
        let apps = load_sanitized_cache(dir.path()).unwrap();
        assert!(!apps[0].can_uninstall);
    }

    #[test]
    fn enriches_cached_uninstall_data_before_recomputing_availability() {
        let mut app = cached_app("Editor", r"C:\Editor.exe");
        app.can_uninstall = false;
        let apps = prepare_cached_apps(vec![app], |apps| {
            apps[0].uninstall = Some(UninstallTarget::Command {
                executable: r"C:\Editor\uninstall.exe".into(),
                arguments: String::new(),
            });
        });
        assert!(apps[0].can_uninstall);
        assert!(apps[0].uninstall.is_some());
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
