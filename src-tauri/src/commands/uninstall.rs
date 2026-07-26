use super::run_blocking;
use crate::app_state::{execute_and_record, preview_for, AppState, UninstallPreview};
use crate::error::AppError;
use crate::platform::windows::{uninstall_history, uninstaller};
use tauri::Manager;

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
