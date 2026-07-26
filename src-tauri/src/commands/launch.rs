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

#[tauri::command]
pub(crate) async fn launch_app(app: tauri::AppHandle, id: String) -> Result<(), AppError> {
    let (launch_kind, path, launch_waits) = {
        let state = app.state::<AppState>();
        let stored = state
            .launch_targets
            .lock()
            .map_err(|_| AppError::LaunchDataUnavailable)?;
        let (kind, path) = stored
            .get(&id)
            .cloned()
            .ok_or(AppError::LaunchUnavailable)?;
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
