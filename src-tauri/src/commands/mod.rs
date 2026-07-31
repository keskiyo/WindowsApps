//! Tauri IPC command surface. Every command resolves real launch/uninstall targets
//! and filesystem paths server-side; the webview only ever passes catalog ids.
//!
//! Commands are grouped by domain into submodules. `lib.rs`'s `invoke_handler!` references each
//! command by its submodule path (`commands::<domain>::<name>`) — the `#[tauri::command]` macro
//! generates hidden items alongside each command, so a plain re-export cannot stand in for it.

use crate::error::AppError;

pub(crate) mod catalog;
pub(crate) mod details;
pub(crate) mod launch;
pub(crate) mod links;
pub(crate) mod settings;
pub(crate) mod uninstall;

/// Upper bound on an inbound catalog id, shared by every command that resolves one.
///
/// Real ids are a SHA-256 hex digest or an AppUserModelId, far below this. The bound exists so an
/// unbounded `String` from the webview is never used as a lookup key or copied into an error path
/// first and judged second. It matches the per-id limit the icon-hydration contract already
/// enforces, so the whole IPC surface has one answer for "how long may an id be".
pub(super) const MAX_CATALOG_ID_LENGTH: usize = 512;

/// Whether a value can name a catalog entry at all. Checked before the id reaches trusted state;
/// an id that fails here cannot match anything the backend stored, so callers report the same
/// "unavailable" error they already return for an unknown id and the IPC error contract is
/// unchanged.
pub(super) fn is_valid_catalog_id(id: &str) -> bool {
    !id.trim().is_empty() && id.chars().count() <= MAX_CATALOG_ID_LENGTH
}

/// Runs blocking work off the async runtime, mapping join failures to `Interrupted`.
/// Private to `commands`; its domain submodules reach it as `super::run_blocking`.
async fn run_blocking<T, F>(context: &'static str, work: F) -> Result<T, AppError>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|error| AppError::Interrupted {
            context,
            source: error.to_string(),
        })
}

#[cfg(test)]
mod tests {
    use super::run_blocking;

    #[test]
    fn blocking_adapter_runs_work_off_the_calling_thread() {
        let calling_thread = std::thread::current().id();
        let worker_thread = tauri::async_runtime::block_on(run_blocking("Test operation", || {
            std::thread::current().id()
        }))
        .unwrap();

        assert_ne!(calling_thread, worker_thread);
    }
}
