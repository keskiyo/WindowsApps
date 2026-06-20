mod catalog;
mod lifecycle;
mod platform;

use catalog::{cache, AppInfo, LaunchKind, UninstallTarget};
use platform::windows::{autostart, global_shortcut, launcher, uninstaller};
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex, OnceLock,
};
use tauri::{Emitter, Manager};

static UNINSTALL_TARGETS: OnceLock<Mutex<HashMap<String, UninstallTarget>>> = OnceLock::new();
static SCAN_CANCELLED: AtomicBool = AtomicBool::new(false);

fn uninstall_targets() -> &'static Mutex<HashMap<String, UninstallTarget>> {
    UNINSTALL_TARGETS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn remember_uninstall_targets(apps: &[AppInfo]) {
    let targets = apps
        .iter()
        .filter_map(|app| app.uninstall.clone().map(|target| (app.id.clone(), target)))
        .collect();
    if let Ok(mut stored) = uninstall_targets().lock() {
        *stored = targets;
    }
}

fn load_sanitized_cache(app_data_dir: &Path) -> Option<Vec<AppInfo>> {
    let original = cache::read_cache(app_data_dir)?;
    let apps = prepare_cached_apps(
        original.clone(),
        catalog::enrich_registered_uninstall_metadata,
    );
    if apps != original {
        let _ = cache::write_cache(app_data_dir, &apps);
    }
    Some(apps)
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
    fixed_drives: &[std::path::PathBuf],
) -> Result<catalog::scan_settings::ScanSettings, String> {
    let normalize = |values: Vec<String>| -> Result<Vec<String>, String> {
        let roots = fixed_drives
            .iter()
            .map(|path| path.to_string_lossy().to_lowercase())
            .collect::<Vec<_>>();
        let mut normalized = Vec::<String>::new();
        for value in values {
            let value = value.trim().trim_matches('"').to_string();
            if value.is_empty() {
                continue;
            }
            let lower = value.to_lowercase();
            if !roots.iter().any(|root| lower.starts_with(root)) {
                return Err(format!("Scan path is not on a fixed local drive: {value}"));
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
    let fixed_drives = platform::windows::drives::fixed_drive_roots();
    let settings = normalize_scan_settings(settings, &fixed_drives)?;
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Could not open the application data folder: {error}"))?;
    catalog::scan_settings::write(&app_data_dir, &settings)
        .map_err(|error| format!("Could not save scan settings: {error}"))?;
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
    let cached = load_sanitized_cache(&app_data_dir);
    let has_cache = cached.is_some();
    let apps = cached.unwrap_or_default();
    remember_uninstall_targets(&apps);
    Ok(CatalogSnapshot { has_cache, apps })
}

#[tauri::command]
async fn refresh_apps(app: tauri::AppHandle) -> Result<Vec<AppInfo>, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Could not open the application data folder: {error}"))?;
    let cached = load_sanitized_cache(&app_data_dir).unwrap_or_default();
    let settings = catalog::scan_settings::read(&app_data_dir);
    SCAN_CANCELLED.store(false, Ordering::Relaxed);
    let progress_handle = app.clone();
    let mut apps = tauri::async_runtime::spawn_blocking(move || {
        catalog::discover_apps_with(
            &settings,
            |progress| {
                let _ = progress_handle.emit("scan://progress", progress);
            },
            || SCAN_CANCELLED.load(Ordering::Relaxed),
        )
    })
    .await
    .map_err(|error| format!("Application scanning was interrupted: {error}"))?;
    if SCAN_CANCELLED.load(Ordering::Relaxed) {
        return Err("Application scan cancelled".into());
    }
    catalog::reuse_cached_icons(&mut apps, &cached);
    remember_uninstall_targets(&apps);
    cache::write_cache(&app_data_dir, &apps)
        .map_err(|error| format!("Could not save the application cache: {error}"))?;

    let response = apps.clone();
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        let hydrated = tauri::async_runtime::spawn_blocking(move || {
            let mut apps = apps;
            catalog::hydrate_missing_icons(&mut apps);
            apps
        })
        .await;
        if let Ok(apps) = hydrated {
            remember_uninstall_targets(&apps);
            let _ = cache::write_cache(&app_data_dir, &apps);
            let _ = handle.emit("apps://updated", apps);
        }
    });
    Ok(response)
}

#[tauri::command]
async fn launch_app(launch_kind: LaunchKind, path: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || launcher::launch(launch_kind, &path))
        .await
        .map_err(|error| format!("Application launch was interrupted: {error}"))?
}

#[tauri::command]
async fn uninstall_app(id: String) -> Result<(), String> {
    let target = uninstall_targets()
        .lock()
        .map_err(|_| "Uninstall data is temporarily unavailable".to_string())?
        .get(&id)
        .cloned();
    tauri::async_runtime::spawn_blocking(move || uninstaller::execute(target))
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
            cancel_scan,
            launch_app,
            uninstall_app,
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
    fn normalizes_scan_paths_and_rejects_non_fixed_locations() {
        let settings = catalog::scan_settings::ScanSettings {
            auto_scan_fixed_drives: true,
            included_paths: vec![r"D:\Portable".into(), r"d:\portable".into()],
            excluded_paths: vec![r"E:\Archives".into()],
        };
        let normalized = normalize_scan_settings(
            settings,
            &[
                std::path::PathBuf::from(r"D:\"),
                std::path::PathBuf::from(r"E:\"),
            ],
        )
        .unwrap();
        assert_eq!(normalized.included_paths, vec![r"D:\Portable"]);
        assert_eq!(normalized.excluded_paths, vec![r"E:\Archives"]);

        let invalid = catalog::scan_settings::ScanSettings {
            auto_scan_fixed_drives: true,
            included_paths: vec![r"Z:\Network".into()],
            excluded_paths: Vec::new(),
        };
        assert!(normalize_scan_settings(invalid, &[std::path::PathBuf::from(r"C:\")]).is_err());
    }
}
