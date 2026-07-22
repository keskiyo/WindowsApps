//! Typed error model for the IPC command surface.
//!
//! Every variant serializes to the exact user-facing string it has always produced, so
//! the frontend contract is unchanged — the enum only makes the error categories
//! explicit in Rust. Failures from lower layers that already carry a finished,
//! user-facing message bubble through [`AppError::Other`] (via `From<String>`),
//! This legacy behavior is replaced by stable codes and safe English messages.

use serde::{Serialize, Serializer};
use std::fmt;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AppErrorPayload {
    code: &'static str,
    message: &'static str,
}

#[derive(Debug)]
pub(crate) enum AppError {
    /// The per-user application data directory could not be resolved.
    AppDataDir(String),
    /// A `spawn_blocking` task join failed before the work reported a result.
    Interrupted {
        context: &'static str,
        source: String,
    },
    /// A scan request was coalesced into another in-flight scan and produced no result.
    Coalesced { what: &'static str },
    /// Persisting scan settings failed.
    SaveScanSettings(String),
    /// Resetting the catalog cache failed.
    ResetCatalogCache(String),
    /// Resetting the icon cache failed.
    ResetIconCache(String),
    /// Clearing the icon cache failed.
    ClearIconCache(String),
    /// Clearing the uninstall history failed.
    ClearUninstallHistory(String),
    /// A configured scan path was not absolute.
    ScanPathNotAbsolute(String),
    /// The release-notes version argument failed validation.
    InvalidReleaseVersion,
    /// The launch target map is unavailable (poisoned).
    LaunchDataUnavailable,
    /// No trusted launch target exists for the requested id.
    LaunchUnavailable,
    /// The uninstall target map is unavailable (poisoned).
    UninstallDataUnavailable,
    /// No trusted uninstall target exists for the requested id.
    UninstallUnavailable,
    /// The bundle product name is not configured.
    ProductNameMissing,
    /// No newer installed copy was found in the registry.
    NoNewerCopy,
    /// A lower-layer failure that already carries a finished, user-facing message.
    Other(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.safe_message())
    }
}

impl AppError {
    fn code(&self) -> &'static str {
        match self {
            Self::AppDataDir(_) => "APP_DATA_UNAVAILABLE",
            Self::Interrupted { .. } => "OPERATION_INTERRUPTED",
            Self::Coalesced { .. } => "SCAN_COALESCED",
            Self::SaveScanSettings(_) => "SAVE_SCAN_SETTINGS_FAILED",
            Self::ResetCatalogCache(_) => "RESET_CATALOG_CACHE_FAILED",
            Self::ResetIconCache(_) => "RESET_ICON_CACHE_FAILED",
            Self::ClearIconCache(_) => "CLEAR_ICON_CACHE_FAILED",
            Self::ClearUninstallHistory(_) => "CLEAR_UNINSTALL_HISTORY_FAILED",
            Self::ScanPathNotAbsolute(_) => "SCAN_PATH_NOT_ABSOLUTE",
            Self::InvalidReleaseVersion => "INVALID_RELEASE_VERSION",
            Self::LaunchDataUnavailable => "LAUNCH_DATA_UNAVAILABLE",
            Self::LaunchUnavailable => "LAUNCH_UNAVAILABLE",
            Self::UninstallDataUnavailable => "UNINSTALL_DATA_UNAVAILABLE",
            Self::UninstallUnavailable => "UNINSTALL_UNAVAILABLE",
            Self::ProductNameMissing => "PRODUCT_NAME_MISSING",
            Self::NoNewerCopy => "NO_NEWER_COPY",
            Self::Other(_) => "OPERATION_FAILED",
        }
    }

    fn safe_message(&self) -> &'static str {
        match self {
            Self::AppDataDir(_source) => "Could not access application data. Try again.",
            Self::Interrupted {
                context: _context,
                source: _source,
            } => "The operation was interrupted. Try again.",
            Self::Coalesced { what: _what } => "The scan could not be completed. Try again.",
            Self::SaveScanSettings(_source) => "Could not save scan settings. Try again.",
            Self::ResetCatalogCache(_source) => "Could not reset the catalog cache. Try again.",
            Self::ResetIconCache(_source) => "Could not reset the icon cache. Try again.",
            Self::ClearIconCache(_source) => "Could not clear the icon cache. Try again.",
            Self::ClearUninstallHistory(_source) => "Could not clear uninstall history. Try again.",
            Self::ScanPathNotAbsolute(_path) => "Scan paths must be absolute.",
            Self::InvalidReleaseVersion => "The release version is invalid.",
            Self::LaunchDataUnavailable => "Launch data is temporarily unavailable.",
            Self::LaunchUnavailable => "This application is not available for launch.",
            Self::UninstallDataUnavailable => "Uninstall data is temporarily unavailable.",
            Self::UninstallUnavailable => "Uninstall is unavailable for this application.",
            Self::ProductNameMissing => "The installed application could not be identified.",
            Self::NoNewerCopy => "No newer installed copy was found.",
            Self::Other(_message) => "The operation could not be completed. Try again.",
        }
    }
}

impl std::error::Error for AppError {}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        AppErrorPayload {
            code: self.code(),
            message: self.safe_message(),
        }
        .serialize(serializer)
    }
}

impl From<String> for AppError {
    fn from(message: String) -> Self {
        AppError::Other(message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_to_a_safe_error_envelope() {
        let error = AppError::AppDataDir("denied".into());
        assert_eq!(
            serde_json::to_value(&error).unwrap(),
            serde_json::json!({
                "code": "APP_DATA_UNAVAILABLE",
                "message": "Could not access application data. Try again.",
            })
        );
    }

    #[test]
    fn bubbled_strings_do_not_reach_the_webview() {
        let error: AppError = "This uninstaller was blocked for safety."
            .to_string()
            .into();
        assert_eq!(
            error.to_string(),
            "The operation could not be completed. Try again."
        );
    }

    #[test]
    fn context_variants_have_safe_messages() {
        assert_eq!(
            AppError::Interrupted {
                context: "Application scanning",
                source: "panicked".into(),
            }
            .to_string(),
            "The operation was interrupted. Try again."
        );
        assert_eq!(
            AppError::Coalesced {
                what: "Application refresh",
            }
            .to_string(),
            "The scan could not be completed. Try again."
        );
    }
}
