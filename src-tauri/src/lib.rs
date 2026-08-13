#![deny(unreachable_pub)]

mod app_state;
mod catalog;
mod commands;
mod error;
mod lifecycle;
mod platform;

use std::sync::Arc;
use tauri::Manager;

use app_state::{remember_catalog, AppState};
use catalog::sync::{load_sanitized_cache, restart_change_watcher};
use platform::windows::{autostart, global_shortcut, install_registry};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let lifecycle = Arc::new(lifecycle::LifecycleState::default());
    let starts_hidden_from_autostart = lifecycle::starts_hidden_from_autostart(std::env::args_os());
    let close_lifecycle = Arc::clone(&lifecycle);
    let tray_lifecycle = Arc::clone(&lifecycle);
    let mut builder = tauri::Builder::default();
    #[cfg(desktop)]
    {
        builder = builder
            .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
                if lifecycle::should_show_on_second_instance(args) {
                    lifecycle::show_main_window(app);
                }
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
            let registered = global_shortcut::register(app.handle().clone());
            {
                let state = app.state::<AppState>();
                if let Ok(mut status) = state.shortcut_status.lock() {
                    *status = registered.status;
                }
                if let Some(guard) = registered.guard {
                    if let Ok(mut current) = state.global_shortcut.lock() {
                        *current = Some(guard);
                    }
                }
            }
            let tray_ready = match lifecycle::setup_tray(app.handle(), Arc::clone(&tray_lifecycle))
            {
                Ok(()) => true,
                Err(error) => {
                    log::error!("Could not create the system tray: {error}");
                    false
                }
            };
            if lifecycle::should_hide_on_autostart(starts_hidden_from_autostart, tray_ready) {
                lifecycle::hide_main_window(app.handle());
            }
            if let Ok(app_data_dir) = app.path().app_data_dir() {
                if let Some(apps) = load_sanitized_cache(&app_data_dir) {
                    let state = app.state::<AppState>();
                    remember_catalog(state.inner(), &apps);
                }
                let settings = catalog::scan_settings::read(&app_data_dir);
                restart_change_watcher(app.handle().clone(), &settings);
            }
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
            commands::catalog::get_apps,
            commands::catalog::refresh_apps,
            commands::catalog::force_full_scan,
            commands::catalog::reset_catalog_cache,
            commands::catalog::clear_icon_cache,
            commands::catalog::hydrate_visible_icons,
            commands::catalog::start_background_sync,
            commands::settings::cancel_scan,
            commands::launch::launch_app,
            commands::close::close_apps,
            commands::details::get_app_details,
            commands::details::open_app_folder,
            commands::uninstall::get_uninstall_preview,
            commands::uninstall::uninstall_app,
            commands::uninstall::get_uninstall_history,
            commands::uninstall::clear_uninstall_history,
            commands::settings::get_system_settings,
            commands::settings::set_autostart,
            commands::settings::set_scan_settings,
            commands::settings::save_preferences_backup,
            commands::links::open_telegram,
            commands::links::open_github,
            commands::links::open_apps_settings,
            commands::links::open_release,
            commands::links::stale_copy_status,
            commands::links::open_installed_copy
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
