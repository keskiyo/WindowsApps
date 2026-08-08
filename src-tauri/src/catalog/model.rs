use super::visibility::{VisibilityClass, VisibilityReason};
use crate::platform::windows::{AppArchitecture, AppSignatureStatus};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AppCategory {
    Games,
    Ai,
    Editors,
    Development,
    Productivity,
    Browsers,
    Media,
    Communication,
    FileCloud,
    Security,
    Utilities,
    System,
    WindowsFeatures,
    InstallersDocs,
    #[default]
    #[serde(other)]
    Other,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ArtifactKind {
    #[default]
    Application,
    Installer,
    Documentation,
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
    pub artifact_kind: ArtifactKind,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preference_identity: Option<String>,
    #[serde(default)]
    pub visibility_class: VisibilityClass,
    #[serde(default)]
    pub visibility_score: i16,
    #[serde(default)]
    pub visibility_reasons: Vec<VisibilityReason>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_availability: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub category_reasons: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub close_risk: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AppDetails {
    pub file_size_bytes: Option<u64>,
    pub file_created_at: Option<u64>,
    pub file_modified_at: Option<u64>,
    pub architecture: AppArchitecture,
    pub signature: AppSignatureStatus,
    pub executable_exists: Option<bool>,
    pub install_location_exists: Option<bool>,
    #[serde(default)]
    pub can_open_folder: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ScanProgress {
    pub stage: String,
    pub location: Option<String>,
    pub completed_roots: usize,
    pub total_roots: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_persisted_enum_values_degrade_instead_of_failing() {
        let json = serde_json::json!({
            "id": "x",
            "name": "X",
            "path": "C:\\x.exe",
            "iconBase64": null,
            "category": "quantum_future",
            "visibilityReasons": ["teleportation", "product_component"]
        });

        let app: AppInfo =
            serde_json::from_value(json).expect("unknown enum values degrade, not fail");

        assert_eq!(app.category, AppCategory::Other);
        assert!(app.visibility_reasons.contains(&VisibilityReason::Unknown));
        assert!(app
            .visibility_reasons
            .contains(&VisibilityReason::ProductComponent));
    }
}
