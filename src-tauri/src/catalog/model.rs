//! Core catalog domain types shared across the backend and mirrored by the frontend `types/`.
//! Pure data with serde derives — no behavior. Re-exported from `catalog` so existing
//! `crate::catalog::AppInfo` paths stay unchanged.

use super::visibility::{VisibilityClass, VisibilityReason};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AppCategory {
    Games,
    Ai,
    Editors,
    Development,
    Browsers,
    Media,
    Communication,
    Utilities,
    System,
    WindowsFeatures,
    #[default]
    Other,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum LaunchKind {
    #[default]
    Executable,
    Shortcut,
    AppUserModelId,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SourceKind {
    #[default]
    Registry,
    StartMenu,
    StartApps,
    Msix,
    Steam,
    Portable,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum UninstallTarget {
    Command {
        executable: String,
        arguments: String,
    },
    Msix {
        package_full_name: String,
    },
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AppInfo {
    pub id: String,
    pub name: String,
    pub path: String,
    pub icon_base64: Option<String>,
    #[serde(default)]
    pub category: AppCategory,
    #[serde(default)]
    pub launch_kind: LaunchKind,
    #[serde(default)]
    pub source_kind: SourceKind,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub publisher: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub product_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub original_filename: Option<String>,
    #[serde(default)]
    pub install_location: Option<String>,
    #[serde(default)]
    pub can_uninstall: bool,
    #[serde(default)]
    pub uninstall: Option<UninstallTarget>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shortcut_icon_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub launch_arguments: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub canonical_identity: Option<String>,
    #[serde(default)]
    pub visibility_class: VisibilityClass,
    #[serde(default)]
    pub visibility_score: i16,
    #[serde(default)]
    pub visibility_reasons: Vec<VisibilityReason>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ScanProgress {
    pub stage: String,
    pub location: Option<String>,
    pub completed_roots: usize,
    pub total_roots: usize,
}
