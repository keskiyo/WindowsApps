mod apps_scanner;
mod app_lifecycle;
mod autostart;
mod cache;
mod executable_metadata;
mod icon_extractor;
mod launcher;
mod global_shortcut;
mod shortcut;
mod uninstaller;

use apps_scanner::{AppInfo, LaunchKind, UninstallTarget};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};
use serde::Serialize;
use tauri::{Emitter, Manager};

static UNINSTALL_TARGETS: OnceLock<Mutex<HashMap<String, UninstallTarget>>> = OnceLock::new();

fn uninstall_targets() -> &'static Mutex<HashMap<String, UninstallTarget>> {
    UNINSTALL_TARGETS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn remember_uninstall_targets(apps: &[AppInfo]) {
    let targets = apps.iter().filter_map(|app| app.uninstall.clone().map(|target| (app.id.clone(), target))).collect();
    if let Ok(mut stored) = uninstall_targets().lock() { *stored = targets; }
}

fn load_sanitized_cache(app_data_dir: &Path) -> Option<Vec<AppInfo>> {
    let original = cache::read_cache(app_data_dir)?;
    let mut apps = apps_scanner::sanitize(original.clone());
    for app in &mut apps {
        app.can_uninstall = app.uninstall.is_some();
    }
    if apps != original {
        let _ = cache::write_cache(app_data_dir, &apps);
    }
    Some(apps)
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
}

#[tauri::command]
async fn get_system_settings() -> Result<SystemSettings, String> {
    let autostart_enabled = tauri::async_runtime::spawn_blocking(autostart::is_enabled)
        .await.map_err(|error| format!("Startup check was interrupted: {error}"))??;
    Ok(SystemSettings { version: env!("CARGO_PKG_VERSION"), autostart_enabled, shortcut: global_shortcut::status() })
}

#[tauri::command]
async fn set_autostart(enabled: bool) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || autostart::set_enabled(enabled))
        .await.map_err(|error| format!("Startup update was interrupted: {error}"))?
}

#[tauri::command]
async fn open_telegram() -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(|| launcher::shell_execute("https://t.me/keskiyo"))
        .await.map_err(|error| format!("Telegram launch was interrupted: {error}"))?
}

#[tauri::command]
async fn get_apps(app: tauri::AppHandle) -> Result<CatalogSnapshot, String> {
    let app_data_dir = app.path().app_data_dir().map_err(|error| format!("Could not open the application data folder: {error}"))?;
    let cached = load_sanitized_cache(&app_data_dir);
    let has_cache = cached.is_some();
    let apps = cached.unwrap_or_default();
    remember_uninstall_targets(&apps);
    Ok(CatalogSnapshot { has_cache, apps })
}

#[tauri::command]
async fn refresh_apps(app: tauri::AppHandle) -> Result<Vec<AppInfo>, String> {
    let app_data_dir = app.path().app_data_dir().map_err(|error| format!("Could not open the application data folder: {error}"))?;
    let cached = load_sanitized_cache(&app_data_dir).unwrap_or_default();
    let mut apps = tauri::async_runtime::spawn_blocking(apps_scanner::discover_apps)
        .await.map_err(|error| format!("Application scanning was interrupted: {error}"))?;
    apps_scanner::reuse_cached_icons(&mut apps, &cached);
    remember_uninstall_targets(&apps);
    cache::write_cache(&app_data_dir, &apps)
        .map_err(|error| format!("Could not save the application cache: {error}"))?;

    let response = apps.clone();
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        let hydrated = tauri::async_runtime::spawn_blocking(move || {
            let mut apps = apps;
            apps_scanner::hydrate_missing_icons(&mut apps);
            apps
        }).await;
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
        .await.map_err(|error| format!("Application launch was interrupted: {error}"))?
}

#[tauri::command]
async fn uninstall_app(id: String) -> Result<(), String> {
    let target = uninstall_targets().lock().map_err(|_| "Uninstall data is temporarily unavailable".to_string())?.get(&id).cloned();
    tauri::async_runtime::spawn_blocking(move || uninstaller::execute(target))
        .await.map_err(|error| format!("Uninstall launch was interrupted: {error}"))??;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
	let lifecycle = Arc::new(app_lifecycle::LifecycleState::default());
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
			if let Err(error) = app_lifecycle::setup_tray(app.handle(), Arc::clone(&tray_lifecycle)) {
				log::error!("Could not create the system tray: {error}");
			}
            if let Ok(app_data_dir) = app.path().app_data_dir() {
                if let Some(apps) = load_sanitized_cache(&app_data_dir) {
                    remember_uninstall_targets(&apps);
                }
            }
            if cfg!(debug_assertions) {
                app.handle().plugin(tauri_plugin_log::Builder::default().level(log::LevelFilter::Info).build())?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_apps, refresh_apps, launch_app, uninstall_app, get_system_settings, set_autostart, open_telegram])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;
    use apps_scanner::{AppCategory, SourceKind};

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
                cached_app("Visual Studio Installer", "Microsoft.VisualStudio.Installer"),
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
}
