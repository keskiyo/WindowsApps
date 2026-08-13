use super::{
    AppCategory, AppInfo, ArtifactKind, LaunchKind, SourceKind, VisibilityClass, VisibilityReason,
};
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CatalogAppDto {
    pub id: String,
    pub name: String,
    pub path: String,
    pub icon_base64: Option<String>,
    pub artifact_kind: ArtifactKind,
    pub category: AppCategory,
    pub launch_kind: LaunchKind,
    pub source_kind: SourceKind,
    pub description: Option<String>,
    pub version: Option<String>,
    pub publisher: Option<String>,
    pub product_name: Option<String>,
    pub original_filename: Option<String>,
    pub install_location: Option<String>,
    pub can_uninstall: bool,
    pub canonical_identity: Option<String>,
    pub preference_identity: Option<String>,
    pub visibility_class: VisibilityClass,
    pub visibility_score: i16,
    pub visibility_reasons: Vec<VisibilityReason>,
    pub target_availability: Option<String>,
    pub category_reasons: Vec<String>,
    pub close_risk: Option<String>,
}

impl From<&AppInfo> for CatalogAppDto {
    fn from(app: &AppInfo) -> Self {
        Self {
            id: app.id.clone(),
            name: app.name.clone(),
            path: app.path.clone(),
            icon_base64: app.icon_base64.clone(),
            artifact_kind: app.artifact_kind,
            category: app.category,
            launch_kind: app.launch_kind,
            source_kind: app.source_kind,
            description: app.description.clone(),
            version: app.version.clone(),
            publisher: app.publisher.clone(),
            product_name: app.product_name.clone(),
            original_filename: app.original_filename.clone(),
            install_location: app.install_location.clone(),
            can_uninstall: app.can_uninstall,
            canonical_identity: app.canonical_identity.clone(),
            preference_identity: app.preference_identity.clone(),
            visibility_class: app.visibility_class,
            visibility_score: app.visibility_score,
            visibility_reasons: app.visibility_reasons.clone(),
            target_availability: app.target_availability.clone(),
            category_reasons: app.category_reasons.clone(),
            close_risk: app.close_risk.clone(),
        }
    }
}
