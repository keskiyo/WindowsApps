mod documentation;
mod installer;

use super::machine::MachineFacts;
use super::{AppInfo, ArtifactKind};

pub(super) use installer::has_installer_filename_evidence;

pub(crate) fn classify(
    app: &AppInfo,
    internal_name: Option<&str>,
    facts: &MachineFacts,
) -> ArtifactKind {
    if documentation::is_documentation_shortcut(app) {
        ArtifactKind::Documentation
    } else if installer::is_installation_artifact(app, internal_name, facts) {
        ArtifactKind::Installer
    } else {
        ArtifactKind::Application
    }
}

#[cfg(test)]
pub(super) mod testing {
    use crate::catalog::{AppCategory, AppInfo, ArtifactKind, LaunchKind, SourceKind};

    pub(super) fn candidate(name: &str, path: &str) -> AppInfo {
        AppInfo {
            id: "test".into(),
            name: name.into(),
            path: path.into(),
            icon_base64: None,
            artifact_kind: ArtifactKind::Application,
            category: AppCategory::Other,
            launch_kind: LaunchKind::Executable,
            source_kind: SourceKind::Portable,
            description: None,
            version: None,
            publisher: None,
            product_name: None,
            original_filename: None,
            install_location: None,
            can_uninstall: false,
            uninstall: None,
            resolved_path: None,
            shortcut_icon_path: None,
            launch_arguments: None,
            canonical_identity: None,
            preference_identity: None,
            visibility_class: Default::default(),
            visibility_score: 0,
            visibility_reasons: Vec::new(),
            target_availability: None,
            category_reasons: Vec::new(),
            close_risk: None,
        }
    }

    pub(super) fn start_menu_shortcut(name: &str, path: &str, target: &str) -> AppInfo {
        let mut app = candidate(name, path);
        app.source_kind = SourceKind::StartMenu;
        app.launch_kind = LaunchKind::Shortcut;
        app.resolved_path = Some(target.into());
        app
    }
}
