use super::uninstaller::UninstallOutcome;
use windows::core::HSTRING;
use windows::Management::Deployment::{PackageManager, RemovalOptions};
use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_MULTITHREADED};

const UNAVAILABLE: &str = "Package removal is unavailable on this system";
const INTERRUPTED: &str = "Package removal did not finish";

pub(super) fn remove(package_full_name: &str) -> Result<UninstallOutcome, String> {
    let package_full_name = package_full_name.to_string();
    std::thread::spawn(move || remove_in_worker(&package_full_name))
        .join()
        .map_err(|_| INTERRUPTED.to_string())?
}

fn remove_in_worker(package_full_name: &str) -> Result<UninstallOutcome, String> {
    let _apartment = MultithreadedApartment::enter();
    let manager = PackageManager::new().map_err(|_| UNAVAILABLE.to_string())?;
    let operation = manager
        .RemovePackageWithOptionsAsync(&HSTRING::from(package_full_name), RemovalOptions::None)
        .map_err(|_| UNAVAILABLE.to_string())?;
    let result = operation.get().map_err(|_| INTERRUPTED.to_string())?;
    classify_deployment(result.ExtendedErrorCode().map(|code| code.0).unwrap_or(0))
}

fn classify_deployment(extended_error: i32) -> Result<UninstallOutcome, String> {
    if extended_error == 0 {
        return Ok(UninstallOutcome::Completed);
    }
    Err(format!(
        "Package removal reported error 0x{:08X}",
        extended_error as u32
    ))
}

struct MultithreadedApartment(bool);

impl MultithreadedApartment {
    fn enter() -> Self {
        // SAFETY: `remove` calls this on a thread it has just spawned, so nothing has joined an
        // apartment on it yet and there is no incompatible model to clash with. `CoInitializeEx`
        // takes no pointer from us — the reserved argument is `None`. Its success is recorded so
        // `Drop` only unwinds an apartment this thread actually owns. A multithreaded apartment is
        // required here rather than the process-wide single-threaded one: `get()` below blocks
        // without pumping messages, which would deadlock a completion marshalled into an STA.
        Self(unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }.is_ok())
    }
}

impl Drop for MultithreadedApartment {
    fn drop(&mut self) {
        if self.0 {
            // SAFETY: `CoUninitialize` must balance a `CoInitializeEx` that succeeded on this
            // thread. The flag is set only by `enter`, this value is neither `Send` nor `Clone`, and
            // it is dropped exactly once when the worker returns, so the call is paired and happens
            // on the initializing thread.
            unsafe { CoUninitialize() };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_deployment_without_an_extended_error_completed() {
        assert_eq!(classify_deployment(0), Ok(UninstallOutcome::Completed));
    }

    #[test]
    fn a_deployment_error_is_reported_without_a_package_name_or_path() {
        let message = classify_deployment(0x80073CFA_u32 as i32).unwrap_err();

        assert_eq!(message, "Package removal reported error 0x80073CFA");
        assert!(!message.contains('\\'));
    }

    #[test]
    fn removing_an_absent_package_reaches_deployment_without_an_interpreter() {
        let outcome = remove("WindowsApps.Absent_0.0.0.0_x64__0000000000000");

        assert_eq!(
            outcome,
            Err("Package removal reported error 0x80073CF1".to_string()),
            "expected ERROR_INSTALL_PACKAGE_NOT_FOUND from the deployment API itself; \
             a different message means the worker never reached it"
        );
    }
}
