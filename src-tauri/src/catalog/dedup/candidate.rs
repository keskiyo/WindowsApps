use super::arguments::meaningful_launch_arguments;
use super::family::{
    helper_variant_family, launcher_product_family, normalized_product_family, normalized_publisher,
};
use super::signals::is_helper_candidate;
use super::target::{
    is_squirrel_stub, is_system_tool_target, launch_target, normalize_path, parent_path,
    squirrel_package, steam_app_id, system_tool_alias,
};
use crate::catalog::{AppInfo, LaunchKind, SourceKind, VisibilityReason};
use std::path::Path;

#[derive(Clone, Debug)]
pub(super) struct AppCandidate {
    pub(super) app: AppInfo,
    pub(super) family: String,
    pub(super) identity: CandidateIdentity,
    pub(super) launcher_family: String,
    pub(super) helper_family: String,
    pub(super) publisher_key: String,
    pub(super) parent: Option<String>,
    pub(super) squirrel_package: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) struct CandidateIdentity {
    pub(super) steam_app_id: Option<String>,
    pub(super) aumid: Option<String>,
    pub(super) launch_target: Option<String>,
    pub(super) launch_mode: Option<String>,
    pub(super) install_root: Option<String>,
    pub(super) registry_product: Option<String>,
    pub(super) portable_product: Option<String>,
    pub(super) path: String,
}

impl From<AppInfo> for AppCandidate {
    fn from(app: AppInfo) -> Self {
        let family = normalized_product_family(&app.name);
        let identity = CandidateIdentity::from_app(&app);
        Self {
            launcher_family: launcher_product_family(&family).to_string(),
            helper_family: helper_variant_family(&family),
            publisher_key: normalized_publisher(app.publisher.as_deref()),
            parent: parent_path(&identity.path),
            squirrel_package: squirrel_package(&app),
            family,
            identity,
            app,
        }
    }
}

impl CandidateIdentity {
    pub(super) fn from_app(app: &AppInfo) -> Self {
        let family = normalized_product_family(&app.name);
        let publisher = normalized_publisher(app.publisher.as_deref());
        let install_root = app.install_location.as_deref().map(normalize_path);
        Self {
            steam_app_id: steam_app_id(app).map(str::to_string),
            aumid: (app.launch_kind == LaunchKind::AppUserModelId)
                .then(|| app.path.trim().to_lowercase()),
            launch_target: launch_target(app).map(normalize_path),
            launch_mode: meaningful_launch_arguments(app.launch_arguments.as_deref()),
            registry_product: (app.source_kind == SourceKind::Registry).then(|| {
                format!(
                    "{}|{}|{}",
                    publisher,
                    family,
                    install_root.clone().unwrap_or_default()
                )
            }),
            portable_product: (app.source_kind == SourceKind::Portable)
                .then(|| format!("{}|{}", install_root.clone().unwrap_or_default(), family)),
            install_root,
            path: normalize_path(&app.path),
        }
    }
}

fn is_side_action(app: &AppInfo) -> bool {
    app.visibility_reasons.iter().any(|reason| {
        matches!(
            reason,
            VisibilityReason::MaintenanceExecutable | VisibilityReason::DocumentationShortcut
        )
    })
}

pub(super) fn candidate_score(app: &AppInfo) -> u8 {
    if is_helper_candidate(app) || is_side_action(app) || is_squirrel_stub(app) {
        return 1;
    }
    if app.source_kind == SourceKind::Steam {
        return 5;
    }
    if app.launch_kind == LaunchKind::AppUserModelId
        && app.source_kind == SourceKind::StartApps
        && (is_system_tool_target(app) || system_tool_alias(app).is_some())
    {
        return 6;
    }
    match Path::new(&app.path)
        .extension()
        .and_then(|value| value.to_str())
    {
        Some(extension) if extension.eq_ignore_ascii_case("lnk") => return 4,
        Some(extension)
            if extension.eq_ignore_ascii_case("url")
                && app.source_kind == SourceKind::StartMenu
                && app.launch_kind == LaunchKind::Shortcut =>
        {
            return 4;
        }
        Some(extension) if extension.eq_ignore_ascii_case("exe") => return 3,
        _ => {}
    }
    if app.launch_kind == LaunchKind::AppUserModelId {
        2
    } else {
        0
    }
}
