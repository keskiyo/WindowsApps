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

#[derive(Clone, Debug, Eq, PartialEq)]
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
    /// The user cancelled an application scan.
    ScanCancelled,
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
    /// The icon hydration request exceeded its bounded IPC contract.
    InvalidHydrationRequest,
    /// The launch target map is unavailable (poisoned).
    LaunchDataUnavailable,
    /// No trusted launch target exists for the requested id.
    LaunchUnavailable,
    /// The close target map is unavailable (poisoned).
    ///
    /// An id with no trusted executable is not an error: `close_apps` closes a batch and counts
    /// the ids it cannot match, so "this one cannot be closed" is part of a successful answer.
    CloseDataUnavailable,
    /// No trusted details target exists for the requested id.
    AppDetailsUnavailable,
    /// No trusted local folder exists for the requested id.
    OpenFolderUnavailable,
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
            Self::ScanCancelled => "SCAN_CANCELLED",
            Self::SaveScanSettings(_) => "SAVE_SCAN_SETTINGS_FAILED",
            Self::ResetCatalogCache(_) => "RESET_CATALOG_CACHE_FAILED",
            Self::ResetIconCache(_) => "RESET_ICON_CACHE_FAILED",
            Self::ClearIconCache(_) => "CLEAR_ICON_CACHE_FAILED",
            Self::ClearUninstallHistory(_) => "CLEAR_UNINSTALL_HISTORY_FAILED",
            Self::ScanPathNotAbsolute(_) => "SCAN_PATH_NOT_ABSOLUTE",
            Self::InvalidReleaseVersion => "INVALID_RELEASE_VERSION",
            Self::InvalidHydrationRequest => "INVALID_HYDRATION_REQUEST",
            Self::LaunchDataUnavailable => "LAUNCH_DATA_UNAVAILABLE",
            Self::LaunchUnavailable => "LAUNCH_UNAVAILABLE",
            Self::CloseDataUnavailable => "CLOSE_DATA_UNAVAILABLE",
            Self::AppDetailsUnavailable => "APP_DETAILS_UNAVAILABLE",
            Self::OpenFolderUnavailable => "OPEN_FOLDER_UNAVAILABLE",
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
            Self::ScanCancelled => "Application scan cancelled.",
            Self::SaveScanSettings(_source) => "Could not save scan settings. Try again.",
            Self::ResetCatalogCache(_source) => "Could not reset the catalog cache. Try again.",
            Self::ResetIconCache(_source) => "Could not reset the icon cache. Try again.",
            Self::ClearIconCache(_source) => "Could not clear the icon cache. Try again.",
            Self::ClearUninstallHistory(_source) => "Could not clear uninstall history. Try again.",
            Self::ScanPathNotAbsolute(_path) => "Scan paths must be absolute.",
            Self::InvalidReleaseVersion => "The release version is invalid.",
            Self::InvalidHydrationRequest => "The icon hydration request is invalid.",
            Self::LaunchDataUnavailable => "Launch data is temporarily unavailable.",
            Self::LaunchUnavailable => "This application is not available for launch.",
            Self::CloseDataUnavailable => "Close data is temporarily unavailable.",
            Self::AppDetailsUnavailable => "Application details are unavailable.",
            Self::OpenFolderUnavailable => "The application folder is unavailable.",
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

    // The error-code set is a cross-language contract mirrored in `src/shared/api/tauri/errors.ts`
    // (`AppErrorCode`). This pins the Rust half: every code is a unique, non-empty
    // SCREAMING_SNAKE value with a non-empty safe message, and the whole set is exactly the
    // documented list. A drift here (renamed or duplicated code) fails the build; the frontend
    // has the mirror test.
    #[test]
    fn error_codes_form_the_expected_stable_contract() {
        let all = [
            AppError::AppDataDir(String::new()),
            AppError::Interrupted {
                context: "x",
                source: String::new(),
            },
            AppError::Coalesced { what: "x" },
            AppError::ScanCancelled,
            AppError::SaveScanSettings(String::new()),
            AppError::ResetCatalogCache(String::new()),
            AppError::ResetIconCache(String::new()),
            AppError::ClearIconCache(String::new()),
            AppError::ClearUninstallHistory(String::new()),
            AppError::ScanPathNotAbsolute(String::new()),
            AppError::InvalidReleaseVersion,
            AppError::InvalidHydrationRequest,
            AppError::LaunchDataUnavailable,
            AppError::LaunchUnavailable,
            AppError::CloseDataUnavailable,
            AppError::AppDetailsUnavailable,
            AppError::OpenFolderUnavailable,
            AppError::UninstallDataUnavailable,
            AppError::UninstallUnavailable,
            AppError::ProductNameMissing,
            AppError::NoNewerCopy,
            AppError::Other(String::new()),
        ];
        for error in &all {
            let code = error.code();
            assert!(!code.is_empty());
            assert!(
                code.chars()
                    .all(|c| c.is_ascii_uppercase() || c == '_' || c.is_ascii_digit()),
                "code not SCREAMING_SNAKE: {code}"
            );
            assert!(!error.safe_message().is_empty());
        }
        let mut codes = all.iter().map(AppError::code).collect::<Vec<_>>();
        let total = codes.len();
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(codes.len(), total, "duplicate error codes");
        assert_eq!(
            codes,
            [
                "APP_DATA_UNAVAILABLE",
                "APP_DETAILS_UNAVAILABLE",
                "CLEAR_ICON_CACHE_FAILED",
                "CLEAR_UNINSTALL_HISTORY_FAILED",
                "CLOSE_DATA_UNAVAILABLE",
                "INVALID_HYDRATION_REQUEST",
                "INVALID_RELEASE_VERSION",
                "LAUNCH_DATA_UNAVAILABLE",
                "LAUNCH_UNAVAILABLE",
                "NO_NEWER_COPY",
                "OPEN_FOLDER_UNAVAILABLE",
                "OPERATION_FAILED",
                "OPERATION_INTERRUPTED",
                "PRODUCT_NAME_MISSING",
                "RESET_CATALOG_CACHE_FAILED",
                "RESET_ICON_CACHE_FAILED",
                "SAVE_SCAN_SETTINGS_FAILED",
                "SCAN_CANCELLED",
                "SCAN_COALESCED",
                "SCAN_PATH_NOT_ABSOLUTE",
                "UNINSTALL_DATA_UNAVAILABLE",
                "UNINSTALL_UNAVAILABLE",
            ]
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

    #[test]
    fn scan_cancellation_has_a_stable_safe_envelope() {
        assert_eq!(
            serde_json::to_value(AppError::ScanCancelled).unwrap(),
            serde_json::json!({
                "code": "SCAN_CANCELLED",
                "message": "Application scan cancelled.",
            })
        );
    }

    #[test]
    fn invalid_hydration_requests_have_a_stable_safe_envelope() {
        assert_eq!(
            serde_json::to_value(AppError::InvalidHydrationRequest).unwrap(),
            serde_json::json!({
                "code": "INVALID_HYDRATION_REQUEST",
                "message": "The icon hydration request is invalid.",
            })
        );
    }
}
