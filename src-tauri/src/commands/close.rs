use super::is_valid_catalog_id;
use crate::app_state::AppState;
use crate::error::AppError;
use crate::platform::windows::closer;
use serde::Serialize;
use std::collections::HashSet;
use std::path::PathBuf;
use tauri::Manager;

const MAX_CLOSE_BATCH: usize = 32;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CloseAppsResponse {
    closed: usize,
    not_running: usize,
    unavailable: usize,
    blocked: usize,
}

#[derive(Debug, Default, PartialEq, Eq)]
struct CloseRequest {
    targets: Vec<closer::CloseTarget>,
    unavailable: usize,
    blocked: usize,
}

fn resolve_close_targets(state: &AppState, ids: Vec<String>) -> Result<CloseRequest, AppError> {
    let stored = state
        .close_targets
        .lock()
        .map_err(|_| AppError::CloseDataUnavailable)?;
    let mut seen = HashSet::with_capacity(ids.len());
    let mut request = CloseRequest::default();
    for id in ids {
        if !seen.insert(id.clone()) {
            continue;
        }
        if !is_valid_catalog_id(&id) {
            request.unavailable += 1;
            continue;
        }
        match stored.get(&id) {
            Some(target) if target.blocked => request.blocked += 1,
            Some(target) if request.targets.len() < MAX_CLOSE_BATCH => {
                request.targets.push(closer::CloseTarget::new(
                    PathBuf::from(&target.path),
                    target.install_root.as_deref().map(PathBuf::from),
                ))
            }
            _ => request.unavailable += 1,
        }
    }
    Ok(request)
}

#[tauri::command]
pub(crate) async fn close_apps(
    app: tauri::AppHandle,
    ids: Vec<String>,
) -> Result<CloseAppsResponse, AppError> {
    let request = {
        let state = app.state::<AppState>();
        resolve_close_targets(&state, ids)?
    };
    let targets = request.targets;
    let outcome = super::run_blocking("Application close", move || {
        closer::close_processes(&targets)
    })
    .await?;
    Ok(CloseAppsResponse {
        closed: outcome.closed,
        not_running: outcome.not_running,
        unavailable: request.unavailable,
        blocked: request.blocked,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_state::{cached_app, remember_catalog};
    use crate::commands::MAX_CATALOG_ID_LENGTH;

    fn executables(request: &CloseRequest) -> Vec<PathBuf> {
        request
            .targets
            .iter()
            .map(|target| target.executable.clone())
            .collect()
    }

    fn state_with(ids: &[&str]) -> AppState {
        let apps = ids
            .iter()
            .map(|id| {
                let mut app = cached_app("Editor", &format!(r"C:\{id}.exe"));
                app.id = (*id).into();
                app
            })
            .collect::<Vec<_>>();
        let state = AppState::default();
        remember_catalog(&state, &apps);
        state
    }

    #[test]
    fn resolves_known_catalog_ids_to_their_executables() {
        let state = state_with(&["editor", "player"]);

        let request =
            resolve_close_targets(&state, vec!["editor".into(), "player".into()]).unwrap();

        assert_eq!(
            executables(&request),
            vec![
                PathBuf::from(r"C:\editor.exe"),
                PathBuf::from(r"C:\player.exe")
            ]
        );
        assert_eq!(request.unavailable, 0);
        assert_eq!(request.blocked, 0);
    }

    #[test]
    fn counts_invalid_and_unknown_ids_as_unavailable_without_looking_them_up() {
        let state = state_with(&["editor"]);
        let oversized = "x".repeat(MAX_CATALOG_ID_LENGTH + 1);

        let request =
            resolve_close_targets(&state, vec![oversized, "   ".into(), "missing".into()]).unwrap();

        assert!(request.targets.is_empty());
        assert_eq!(request.unavailable, 3);
        assert_eq!(request.blocked, 0);
    }

    #[test]
    fn resolves_a_repeated_id_once_without_calling_it_unavailable() {
        let state = state_with(&["editor"]);

        let request =
            resolve_close_targets(&state, vec!["editor".into(), "editor".into()]).unwrap();

        assert_eq!(executables(&request), vec![PathBuf::from(r"C:\editor.exe")]);
        assert_eq!(request.unavailable, 0);
    }

    #[test]
    fn bounds_the_batch_and_reports_the_remainder() {
        let ids = (0..MAX_CLOSE_BATCH + 3)
            .map(|index| format!("app-{index}"))
            .collect::<Vec<_>>();
        let state = state_with(&ids.iter().map(String::as_str).collect::<Vec<_>>());

        let request = resolve_close_targets(&state, ids).unwrap();

        assert_eq!(request.targets.len(), MAX_CLOSE_BATCH);
        assert_eq!(request.unavailable, 3);
    }

    #[test]
    fn a_process_that_would_end_windows_is_refused_and_counted_apart() {
        let mut editor = cached_app("Editor", r"C:\Editor\editor.exe");
        editor.id = "editor".into();
        let mut security = cached_app("Local Security Authority", r"C:\Windows\System32\lsass.exe");
        security.id = "lsass".into();
        let state = AppState::default();
        remember_catalog(&state, &[editor, security]);

        let request = resolve_close_targets(&state, vec!["lsass".into(), "editor".into()]).unwrap();

        assert_eq!(request.blocked, 1);
        assert_eq!(request.unavailable, 0);
        assert_eq!(
            executables(&request),
            vec![PathBuf::from(r"C:\Editor\editor.exe")]
        );
    }

    #[test]
    fn the_desktop_shell_is_still_closable() {
        let mut explorer = cached_app("Проводник", r"C:\Windows\explorer.exe");
        explorer.id = "explorer".into();
        let state = AppState::default();
        remember_catalog(&state, &[explorer]);

        let request = resolve_close_targets(&state, vec!["explorer".into()]).unwrap();

        assert_eq!(request.blocked, 0);
        assert_eq!(
            executables(&request),
            vec![PathBuf::from(r"C:\Windows\explorer.exe")]
        );
    }
}
