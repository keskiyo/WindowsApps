use super::is_valid_catalog_id;
use crate::app_state::AppState;
use crate::error::AppError;
use crate::platform::windows::closer;
use serde::Serialize;
use std::collections::HashSet;
use std::path::PathBuf;
use tauri::Manager;

/// Ceiling on one close request. A scenario already caps its own lists, so this only bounds what
/// the webview can ask for directly: the enumeration work is shared across the batch, but the
/// terminations are not, and an unbounded list would let one call touch every process on the
/// machine.
const MAX_CLOSE_BATCH: usize = 32;

/// What a close request achieved. Counts rather than a bare success: "it was not running", "it is
/// closed now" and "there is nothing to close it by" are three different answers.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CloseAppsResponse {
    closed: usize,
    not_running: usize,
    unavailable: usize,
}

/// The trusted executables for the requested ids, plus how many of them named nothing closable.
///
/// Validation happens before the lookup, so an id that cannot name a catalog entry is never used
/// as a map key. Targets always come from `AppState` — the webview passes ids and nothing else —
/// and an entry with no identifiable running image (a `steam://` target, a Store package whose
/// manifest resolved no executable) is simply absent and counts as unavailable. The same id twice
/// is one program to close and is neither closed twice nor reported as a problem.
fn resolve_close_targets(
    state: &AppState,
    ids: Vec<String>,
) -> Result<(Vec<PathBuf>, usize), AppError> {
    let stored = state
        .close_targets
        .lock()
        .map_err(|_| AppError::CloseDataUnavailable)?;
    let mut seen = HashSet::with_capacity(ids.len());
    let mut targets = Vec::new();
    let mut unavailable = 0;
    for id in ids {
        if !seen.insert(id.clone()) {
            continue;
        }
        if !is_valid_catalog_id(&id) {
            unavailable += 1;
            continue;
        }
        match stored.get(&id) {
            // Past the cap the request is answered honestly rather than quietly truncated.
            Some(path) if targets.len() < MAX_CLOSE_BATCH => targets.push(PathBuf::from(path)),
            _ => unavailable += 1,
        }
    }
    Ok((targets, unavailable))
}

#[tauri::command]
pub(crate) async fn close_apps(
    app: tauri::AppHandle,
    ids: Vec<String>,
) -> Result<CloseAppsResponse, AppError> {
    let (targets, unavailable) = {
        let state = app.state::<AppState>();
        resolve_close_targets(&state, ids)?
    };
    // The closer sleeps through its grace period, so it never runs on the async runtime.
    let outcome = super::run_blocking("Application close", move || {
        closer::close_processes(&targets)
    })
    .await?;
    Ok(CloseAppsResponse {
        closed: outcome.closed,
        not_running: outcome.not_running,
        unavailable,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_state::{cached_app, remember_catalog};
    use crate::commands::MAX_CATALOG_ID_LENGTH;

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

        let (targets, unavailable) =
            resolve_close_targets(&state, vec!["editor".into(), "player".into()]).unwrap();

        assert_eq!(
            targets,
            vec![
                PathBuf::from(r"C:\editor.exe"),
                PathBuf::from(r"C:\player.exe")
            ]
        );
        assert_eq!(unavailable, 0);
    }

    // Same rule as the launch path: an id is judged before it is used as a key into trusted state,
    // so an oversized or blank value never reaches the map.
    #[test]
    fn counts_invalid_and_unknown_ids_as_unavailable_without_looking_them_up() {
        let state = state_with(&["editor"]);
        let oversized = "x".repeat(MAX_CATALOG_ID_LENGTH + 1);

        let (targets, unavailable) =
            resolve_close_targets(&state, vec![oversized, "   ".into(), "missing".into()]).unwrap();

        assert!(targets.is_empty());
        assert_eq!(unavailable, 3);
    }

    // The same app twice is one program to close, not two — and not a problem to report either.
    #[test]
    fn resolves_a_repeated_id_once_without_calling_it_unavailable() {
        let state = state_with(&["editor"]);

        let (targets, unavailable) =
            resolve_close_targets(&state, vec!["editor".into(), "editor".into()]).unwrap();

        assert_eq!(targets, vec![PathBuf::from(r"C:\editor.exe")]);
        assert_eq!(unavailable, 0);
    }

    #[test]
    fn bounds_the_batch_and_reports_the_remainder() {
        let ids = (0..MAX_CLOSE_BATCH + 3)
            .map(|index| format!("app-{index}"))
            .collect::<Vec<_>>();
        let state = state_with(&ids.iter().map(String::as_str).collect::<Vec<_>>());

        let (targets, unavailable) = resolve_close_targets(&state, ids).unwrap();

        assert_eq!(targets.len(), MAX_CLOSE_BATCH);
        assert_eq!(unavailable, 3);
    }
}
