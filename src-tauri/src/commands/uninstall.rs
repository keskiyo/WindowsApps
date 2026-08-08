use super::{is_valid_catalog_id, run_blocking};
use crate::app_state::{execute_and_record, preview_for, AppState, UninstallPreview};
use crate::error::AppError;
use crate::platform::windows::{uninstall_history, uninstaller};
use tauri::Manager;

fn resolve_uninstall_record(
    state: &AppState,
    id: &str,
) -> Result<crate::app_state::UninstallRecord, AppError> {
    if !is_valid_catalog_id(id) {
        return Err(AppError::UninstallUnavailable);
    }
    let stored = state
        .uninstall_targets
        .lock()
        .map_err(|_| AppError::UninstallDataUnavailable)?;
    stored
        .get(id)
        .cloned()
        .ok_or(AppError::UninstallUnavailable)
}

#[tauri::command]
pub(crate) async fn get_uninstall_preview(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<UninstallPreview, AppError> {
    Ok(preview_for(&resolve_uninstall_record(&state, &id)?))
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
    let record = resolve_uninstall_record(&app.state::<AppState>(), &id)?;
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
    use crate::app_state::{cached_app, remember_catalog};
    use crate::catalog::UninstallTarget;
    use crate::commands::MAX_CATALOG_ID_LENGTH;

    fn removable(id: &str) -> crate::catalog::AppInfo {
        let mut app = cached_app("Editor", r"C:\Editor.exe");
        app.id = id.into();
        app.can_uninstall = true;
        app.uninstall = Some(UninstallTarget::Msix {
            package_full_name: "Contoso.Editor_1.0.0_x64__abc".into(),
        });
        app
    }

    #[test]
    fn resolves_a_known_catalog_id_to_its_stored_record() {
        let state = AppState::default();
        remember_catalog(&state, &[removable("editor")]);

        assert!(resolve_uninstall_record(&state, "editor").is_ok());
    }

    #[test]
    fn rejects_an_oversized_id_before_the_lookup() {
        let state = AppState::default();
        let app = removable(&"x".repeat(MAX_CATALOG_ID_LENGTH + 1));
        remember_catalog(&state, std::slice::from_ref(&app));

        assert!(matches!(
            resolve_uninstall_record(&state, &app.id),
            Err(AppError::UninstallUnavailable)
        ));
    }

    #[test]
    fn rejects_a_blank_id() {
        let state = AppState::default();

        assert!(matches!(
            resolve_uninstall_record(&state, ""),
            Err(AppError::UninstallUnavailable)
        ));
    }

    #[test]
    fn rejects_an_unknown_id() {
        let state = AppState::default();

        assert!(matches!(
            resolve_uninstall_record(&state, "missing"),
            Err(AppError::UninstallUnavailable)
        ));
    }
}
