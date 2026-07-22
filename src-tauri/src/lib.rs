// Custom title bar (decorations disabled) needs window drag/min/max/close permissions;
// see src-tauri/capabilities/default.json.
mod app_state;
mod catalog;
mod catalog_sync;
mod commands;
mod error;
mod lifecycle;
mod platform;

use std::sync::Arc;
use tauri::Manager;

use app_state::{remember_launch_targets, remember_uninstall_targets, AppState};
use catalog_sync::{load_sanitized_cache, restart_change_watcher};
use platform::windows::{autostart, global_shortcut, install_registry};

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
        .manage(AppState::default())
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
                    let state = app.state::<AppState>();
                    remember_uninstall_targets(state.inner(), &apps);
                    remember_launch_targets(state.inner(), &apps);
                }
                let settings = catalog::scan_settings::read(&app_data_dir);
                restart_change_watcher(app.handle().clone(), &settings);
            }
            // Installed copies self-heal their registry footprint on every start: the NSIS
            // install-location key follows the directory the app actually runs from (so
            // updates land in the user's chosen folder), and an enabled autostart entry is
            // rewritten if the executable moved.
            if let Some(install_dir) = install_registry::installed_copy_dir() {
                let config = app.config();
                let publisher = config.bundle.publisher.clone().unwrap_or_default();
                let product = config.product_name.clone().unwrap_or_default();
                install_registry::sync_install_dir(&publisher, &product, &install_dir);
                if autostart::is_enabled().unwrap_or(false) {
                    let _ = autostart::set_enabled(true);
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
            commands::get_apps,
            commands::refresh_apps,
            commands::force_full_scan,
            commands::reset_catalog_cache,
            commands::clear_icon_cache,
            commands::hydrate_visible_icons,
            commands::start_background_sync,
            commands::cancel_scan,
            commands::launch_app,
            commands::get_uninstall_preview,
            commands::uninstall_app,
            commands::get_uninstall_history,
            commands::clear_uninstall_history,
            commands::get_system_settings,
            commands::set_autostart,
            commands::set_scan_settings,
            commands::open_telegram,
            commands::open_github,
            commands::open_release,
            commands::stale_copy_status,
            commands::open_installed_copy
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
