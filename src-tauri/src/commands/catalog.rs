use super::run_blocking;
use crate::app_state::{known_catalog_ids, remember_catalog, AppState};
use crate::catalog::sync::SyncRequest;
use crate::catalog::sync::{enqueue_hydration, load_sanitized_document, run_coordinated_scan};
use crate::catalog::{self, cache, CatalogAppDto};
use crate::error::AppError;
use serde::Serialize;
use tauri::Manager;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CatalogSnapshot {
    apps: Vec<CatalogAppDto>,
    has_cache: bool,
    generation: u64,
    diagnostics: Option<cache::CatalogDiagnostics>,
}

const MAX_HYDRATION_IDS: usize = 128;
const MAX_HYDRATION_ID_LENGTH: usize = 512;

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
        remember_catalog(state.inner(), &apps);
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
        apps: apps.iter().map(CatalogAppDto::from).collect(),
        generation,
        diagnostics,
    })
}

#[tauri::command]
pub(crate) async fn refresh_apps(app: tauri::AppHandle) -> Result<Vec<CatalogAppDto>, AppError> {
    let apps = tauri::async_runtime::spawn_blocking(move || {
        run_coordinated_scan(&app, SyncRequest::Refresh, true)?.ok_or(AppError::Coalesced {
            what: "Application refresh",
        })
    })
    .await
    .map_err(|error| AppError::Interrupted {
        context: "Application scanning",
        source: error.to_string(),
    })??;
    Ok(apps.iter().map(CatalogAppDto::from).collect())
}

#[tauri::command]
pub(crate) async fn force_full_scan(app: tauri::AppHandle) -> Result<Vec<CatalogAppDto>, AppError> {
    let apps = tauri::async_runtime::spawn_blocking(move || {
        run_coordinated_scan(&app, SyncRequest::Force, true)?.ok_or(AppError::Coalesced {
            what: "Application scan",
        })
    })
    .await
    .map_err(|error| AppError::Interrupted {
        context: "Application scanning",
        source: error.to_string(),
    })??;
    Ok(apps.iter().map(CatalogAppDto::from).collect())
}

#[tauri::command]
pub(crate) async fn reset_catalog_cache(
    app: tauri::AppHandle,
) -> Result<Vec<CatalogAppDto>, AppError> {
    let apps = tauri::async_runtime::spawn_blocking(move || {
        let app_data_dir = app
            .path()
            .app_data_dir()
            .map_err(|error| AppError::AppDataDir(error.to_string()))?;
        {
            let state = app.state::<AppState>();
            state.scan_coordinator.cancel_all();
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
    })??;
    Ok(apps.iter().map(CatalogAppDto::from).collect())
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
    let ids = {
        let state = app.state::<AppState>();
        validate_hydration_ids(state.inner(), ids)?
    };
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

fn validate_hydration_ids(state: &AppState, ids: Vec<String>) -> Result<Vec<String>, AppError> {
    if ids.len() > MAX_HYDRATION_IDS
        || ids
            .iter()
            .any(|id| id.chars().count() > MAX_HYDRATION_ID_LENGTH)
    {
        return Err(AppError::InvalidHydrationRequest);
    }
    Ok(known_catalog_ids(state, ids))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_state::{cached_app, remember_catalog};
    use crate::catalog::UninstallTarget;

    #[test]
    fn hydration_ids_are_deduplicated_and_limited_to_the_trusted_catalog() {
        let state = AppState::default();
        let mut known = cached_app("Known", r"C:\Known.exe");
        known.id = "known".into();
        remember_catalog(&state, &[known]);

        assert_eq!(
            validate_hydration_ids(
                &state,
                vec![
                    "known".into(),
                    String::new(),
                    "unknown".into(),
                    "known".into(),
                ]
            )
            .unwrap(),
            vec!["known"]
        );
    }

    #[test]
    fn hydration_rejects_oversized_id_lists() {
        let state = AppState::default();
        let ids = (0..=MAX_HYDRATION_IDS)
            .map(|index| format!("app-{index}"))
            .collect();

        assert!(matches!(
            validate_hydration_ids(&state, ids),
            Err(AppError::InvalidHydrationRequest)
        ));
    }

    #[test]
    fn hydration_rejects_oversized_ids() {
        let state = AppState::default();

        assert!(matches!(
            validate_hydration_ids(&state, vec!["x".repeat(MAX_HYDRATION_ID_LENGTH + 1)]),
            Err(AppError::InvalidHydrationRequest)
        ));
    }

    #[test]
    fn hydration_length_limit_counts_unicode_characters() {
        let state = AppState::default();
        let accepted_id = "я".repeat(MAX_HYDRATION_ID_LENGTH);
        let mut known = cached_app("Known", r"C:\Known.exe");
        known.id = accepted_id.clone();
        remember_catalog(&state, &[known]);

        assert_eq!(
            validate_hydration_ids(&state, vec![accepted_id.clone()]).unwrap(),
            vec![accepted_id]
        );
        assert!(matches!(
            validate_hydration_ids(&state, vec!["я".repeat(MAX_HYDRATION_ID_LENGTH + 1)]),
            Err(AppError::InvalidHydrationRequest)
        ));
    }

    #[test]
    fn catalog_snapshot_excludes_execution_metadata_from_webview_json() {
        let mut app = cached_app("Editor", r"C:\Editor\editor.exe");
        app.id = "editor".into();
        app.uninstall = Some(UninstallTarget::Command {
            executable: "TOP_SECRET_UNINSTALL_EXECUTABLE".into(),
            arguments: "TOP_SECRET_UNINSTALL_ARGUMENTS".into(),
        });
        app.launch_arguments = Some("TOP_SECRET_LAUNCH_ARGUMENTS".into());
        app.resolved_path = Some("TOP_SECRET_RESOLVED_TARGET".into());
        app.shortcut_icon_path = Some("TOP_SECRET_SHORTCUT_ICON".into());

        let json = serde_json::to_value(CatalogSnapshot {
            apps: vec![CatalogAppDto::from(&app)],
            has_cache: true,
            generation: 1,
            diagnostics: None,
        })
        .unwrap();
        let serialized = json.to_string();

        for secret in [
            "TOP_SECRET_UNINSTALL_EXECUTABLE",
            "TOP_SECRET_UNINSTALL_ARGUMENTS",
            "TOP_SECRET_LAUNCH_ARGUMENTS",
            "TOP_SECRET_RESOLVED_TARGET",
            "TOP_SECRET_SHORTCUT_ICON",
        ] {
            assert!(!serialized.contains(secret));
        }
        let app = &json["apps"][0];
        assert!(app.get("uninstall").is_none());
        assert!(app.get("launchArguments").is_none());
        assert!(app.get("resolvedPath").is_none());
        assert!(app.get("shortcutIconPath").is_none());
    }
}
