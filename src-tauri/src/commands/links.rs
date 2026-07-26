use crate::error::AppError;
use crate::platform::windows::{exec_target, install_registry, launcher};
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StaleCopy {
    installed_version: String,
    install_location: String,
}

#[tauri::command]
pub(crate) async fn open_telegram() -> Result<(), AppError> {
    tauri::async_runtime::spawn_blocking(|| launcher::shell_execute("https://t.me/keskiyo"))
        .await
        .map_err(|error| AppError::Interrupted {
            context: "Telegram launch",
            source: error.to_string(),
        })?
        .map_err(AppError::from)
}

#[tauri::command]
pub(crate) async fn open_github() -> Result<(), AppError> {
    tauri::async_runtime::spawn_blocking(|| {
        launcher::shell_execute("https://github.com/keskiyo/WindowsApps")
    })
    .await
    .map_err(|error| AppError::Interrupted {
        context: "GitHub launch",
        source: error.to_string(),
    })?
    .map_err(AppError::from)
}

#[tauri::command]
pub(crate) async fn open_release(version: String) -> Result<(), AppError> {
    if version.is_empty()
        || !version
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '.' | '-'))
    {
        return Err(AppError::InvalidReleaseVersion);
    }
    tauri::async_runtime::spawn_blocking(move || {
        launcher::shell_execute(&format!(
            "https://github.com/keskiyo/WindowsApps/releases/tag/v{version}"
        ))
    })
    .await
    .map_err(|error| AppError::Interrupted {
        context: "Release notes launch",
        source: error.to_string(),
    })?
    .map_err(AppError::from)
}

/// Reports when this process is an outdated leftover copy: the uninstall registry says a
/// newer version is installed in a different directory (e.g. an update landed elsewhere).
#[tauri::command]
pub(crate) fn stale_copy_status(app: tauri::AppHandle) -> Option<StaleCopy> {
    let product = app.config().product_name.clone()?;
    install_registry::stale_copy_info(&product).map(|info| StaleCopy {
        installed_version: info.installed_version,
        install_location: info.install_location,
    })
}

/// Launches the registered (newer) installed copy and exits this outdated one. The target
/// path comes from the registry, not from the webview.
#[tauri::command]
pub(crate) fn open_installed_copy(app: tauri::AppHandle) -> Result<(), AppError> {
    let product = app
        .config()
        .product_name
        .clone()
        .ok_or(AppError::ProductNameMissing)?;
    let info = install_registry::stale_copy_info(&product).ok_or(AppError::NoNewerCopy)?;
    let binary = std::env::current_exe()
        .ok()
        .and_then(|path| path.file_name().map(|name| name.to_os_string()))
        .unwrap_or_else(|| "app.exe".into());
    let target = std::path::Path::new(&info.install_location).join(binary);
    // `InstallLocation` comes from HKCU, which any process running as the user can write.
    // Validate it exactly like an uninstaller target before handing it to the shell, so a
    // poisoned key cannot turn this button into "run an arbitrary executable".
    let target = exec_target::validate_executable_path(&target.to_string_lossy())
        .map_err(|_| AppError::LaunchUnavailable)?;
    launcher::shell_execute(&target.to_string_lossy())?;
    // Give the invoke response a moment to reach the webview before exiting.
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(500));
        app.exit(0);
    });
    Ok(())
}
