//! Tauri IPC command surface. Every command resolves real launch/uninstall targets
//! and filesystem paths server-side; the webview only ever passes catalog ids.
//!
//! Commands are grouped by domain into submodules. `lib.rs`'s `invoke_handler!` references each
//! command by its submodule path (`commands::<domain>::<name>`) — the `#[tauri::command]` macro
//! generates hidden items alongside each command, so a plain re-export cannot stand in for it.

use crate::error::AppError;

pub(crate) mod catalog;
pub(crate) mod launch;
pub(crate) mod links;
pub(crate) mod settings;
pub(crate) mod uninstall;

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
