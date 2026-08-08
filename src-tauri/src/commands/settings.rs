use super::run_blocking;
use crate::app_state::AppState;
use crate::catalog;
use crate::catalog::sync::restart_change_watcher;
use crate::error::AppError;
use crate::platform::windows::{autostart, drives, global_shortcut};
use serde::Serialize;
use std::path::Path;
use tauri::Manager;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SystemSettings {
    version: &'static str,
    autostart_enabled: bool,
    shortcut: global_shortcut::Status,
    scan_settings: catalog::scan_settings::ScanSettings,
    fixed_drives: Vec<String>,
}

fn normalize_scan_settings(
    settings: catalog::scan_settings::ScanSettings,
    stored: &catalog::scan_settings::ScanSettings,
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
        catalog_target_availability_v1: stored.catalog_target_availability_v1,
        catalog_portable_fingerprint_v1: stored.catalog_portable_fingerprint_v1,
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
            let fixed_drives = drives::fixed_drive_roots();
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
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| AppError::AppDataDir(error.to_string()))?;
    let settings = normalize_scan_settings(settings, &catalog::scan_settings::read(&app_data_dir))?;
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

#[cfg(test)]
mod tests {
    use super::*;

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
            catalog_target_availability_v1: true,
            catalog_portable_fingerprint_v1: true,
        };
        let stored = catalog::scan_settings::ScanSettings::default();
        let normalized = normalize_scan_settings(settings, &stored).unwrap();
        assert_eq!(
            normalized.included_paths,
            vec![r"D:\Portable", r"F:\Stick\Tools"]
        );
        assert_eq!(normalized.excluded_paths, vec![r"\\Server\Share"]);

        let invalid = catalog::scan_settings::ScanSettings {
            auto_scan_fixed_drives: true,
            included_paths: vec![r"relative\path".into()],
            excluded_paths: Vec::new(),
            catalog_target_availability_v1: true,
            catalog_portable_fingerprint_v1: true,
        };
        assert!(normalize_scan_settings(invalid, &stored).is_err());
    }

    #[test]
    fn the_availability_rollback_flag_comes_from_disk_not_from_the_window() {
        let rolled_back = catalog::scan_settings::ScanSettings {
            catalog_target_availability_v1: false,
            catalog_portable_fingerprint_v1: false,
            ..catalog::scan_settings::ScanSettings::default()
        };
        let from_window = catalog::scan_settings::ScanSettings {
            catalog_target_availability_v1: true,
            catalog_portable_fingerprint_v1: true,
            ..catalog::scan_settings::ScanSettings::default()
        };

        let normalized = normalize_scan_settings(from_window, &rolled_back).unwrap();

        assert!(!normalized.catalog_target_availability_v1);
    }
}
