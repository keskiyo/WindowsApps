use super::is_valid_catalog_id;
use crate::app_state::AppState;
use crate::error::AppError;
use crate::platform::windows::launcher;
use serde::Serialize;
use tauri::{Emitter, Manager};

#[derive(Clone, Serialize)]
pub(crate) struct LaunchStatusPayload {
    id: String,
    state: &'static str,
}

/// Best-effort readiness: blocks until the launched GUI process finishes its startup and is
/// waiting for input (or the timeout/no-message-queue case returns). Always resolves to
/// "ready" so the UI clears its launching state as soon as any signal arrives; genuine
/// launch failures surface earlier via the `launch_app` error path.
fn wait_for_launch_ready(handle: &launcher::OwnedProcessHandle) -> &'static str {
    launcher::wait_for_input_idle(handle, 12000);
    "ready"
}

/// Resolves the trusted launch target for an id coming from the webview.
///
/// Validation happens before the lookup: an id that cannot name a catalog entry is rejected
/// without being used as a map key. The target itself always comes from `AppState`, never from
/// the request — the webview passes an id and nothing else.
fn resolve_launch_target(
    state: &AppState,
    id: &str,
) -> Result<(crate::catalog::LaunchKind, String), AppError> {
    if !is_valid_catalog_id(id) {
        return Err(AppError::LaunchUnavailable);
    }
    let stored = state
        .launch_targets
        .lock()
        .map_err(|_| AppError::LaunchDataUnavailable)?;
    stored.get(id).cloned().ok_or(AppError::LaunchUnavailable)
}

#[tauri::command]
pub(crate) async fn launch_app(app: tauri::AppHandle, id: String) -> Result<(), AppError> {
    let (launch_kind, path, launch_waits) = {
        let state = app.state::<AppState>();
        let (kind, path) = resolve_launch_target(&state, &id)?;
        (kind, path, state.launch_waits.clone())
    };
    let handle = tauri::async_runtime::spawn_blocking(move || launcher::launch(launch_kind, &path))
        .await
        .map_err(|error| AppError::Interrupted {
            context: "Application launch",
            source: error.to_string(),
        })??;
    if let Some(handle) = handle {
        let emitter = app.clone();
        let launch_id = id.clone();
        // Held for the whole wait; when no slot is free the process handle is still closed and
        // the UI falls back to its ceiling timer.
        let Some(permit) = launch_waits.acquire() else {
            return Ok(());
        };
        tauri::async_runtime::spawn_blocking(move || {
            let _permit = permit;
            let state = wait_for_launch_ready(&handle);
            let _ = emitter.emit(
                "launch://status",
                LaunchStatusPayload {
                    id: launch_id,
                    state,
                },
            );
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_state::{cached_app, remember_catalog};
    use crate::commands::MAX_CATALOG_ID_LENGTH;

    #[test]
    fn resolves_a_known_catalog_id_to_its_stored_target() {
        let state = AppState::default();
        let mut app = cached_app("Editor", r"C:\Editor.exe");
        app.id = "editor".into();
        remember_catalog(&state, &[app]);

        let (_, path) = resolve_launch_target(&state, "editor").unwrap();

        assert_eq!(path, r"C:\Editor.exe");
    }

    // The id was an unbounded String used directly as a map key. Length is judged before the
    // lookup so an oversized value never reaches trusted state.
    #[test]
    fn rejects_an_oversized_id_before_the_lookup() {
        let state = AppState::default();
        let mut app = cached_app("Editor", r"C:\Editor.exe");
        app.id = "x".repeat(MAX_CATALOG_ID_LENGTH + 1);
        remember_catalog(&state, &[app.clone()]);

        assert!(matches!(
            resolve_launch_target(&state, &app.id),
            Err(AppError::LaunchUnavailable)
        ));
    }

    #[test]
    fn accepts_an_id_at_the_length_limit() {
        let state = AppState::default();
        let mut app = cached_app("Editor", r"C:\Editor.exe");
        app.id = "я".repeat(MAX_CATALOG_ID_LENGTH);
        remember_catalog(&state, &[app.clone()]);

        assert!(resolve_launch_target(&state, &app.id).is_ok());
    }

    #[test]
    fn rejects_a_blank_id() {
        let state = AppState::default();

        assert!(matches!(
            resolve_launch_target(&state, "   "),
            Err(AppError::LaunchUnavailable)
        ));
    }

    #[test]
    fn rejects_an_unknown_id() {
        let state = AppState::default();

        assert!(matches!(
            resolve_launch_target(&state, "missing"),
            Err(AppError::LaunchUnavailable)
        ));
    }
}
