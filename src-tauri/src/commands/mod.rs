use crate::error::AppError;

pub(crate) mod catalog;
pub(crate) mod close;
pub(crate) mod details;
pub(crate) mod launch;
pub(crate) mod links;
pub(crate) mod settings;
pub(crate) mod uninstall;

pub(super) const MAX_CATALOG_ID_LENGTH: usize = 512;

pub(super) fn is_valid_catalog_id(id: &str) -> bool {
    !id.trim().is_empty() && id.chars().count() <= MAX_CATALOG_ID_LENGTH
}

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
