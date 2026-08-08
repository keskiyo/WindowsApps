use super::candidate::candidate_score;
use super::candidate::AppCandidate;
use super::evidence::score_evidence;
use super::evidence::Evidence;
use super::family::{is_32bit_variant, version_key};
use crate::catalog::{AppCategory, AppInfo, ArtifactKind, SourceKind};

#[derive(Clone, Debug)]
pub(super) struct ResolvedApp {
    pub(super) app: AppInfo,
    pub(super) candidates: Vec<AppCandidate>,
    pub(super) evidence: Vec<Evidence>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) struct ResolverReport {
    pub candidates: usize,
    pub merged: usize,
    pub evidence: Vec<Evidence>,
    pub possible_duplicates: usize,
}

pub(super) fn merge_resolved(
    existing: &mut ResolvedApp,
    candidate: AppCandidate,
    report: &mut ResolverReport,
) {
    let (evidence, _score) = existing
        .candidates
        .iter()
        .map(|left| score_evidence(left, &candidate))
        .max_by_key(|(_, score)| *score)
        .unwrap_or_default();
    report.merged += 1;
    report.evidence.extend(evidence.iter().cloned());
    existing.evidence.extend(evidence);
    existing.app = merge_app(existing.app.clone(), candidate.app.clone());
    existing.candidates.push(candidate);
}

pub(super) fn merge_app(left: AppInfo, right: AppInfo) -> AppInfo {
    let scores_tie = candidate_score(&right) == candidate_score(&left);
    let prefer_right = candidate_score(&right) > candidate_score(&left)
        || (scores_tie
            && left.source_kind == SourceKind::Portable
            && right.source_kind == SourceKind::Portable
            && version_key(right.version.as_deref()) > version_key(left.version.as_deref()))
        || (scores_tie && is_32bit_variant(&left) && !is_32bit_variant(&right));
    let (mut primary, secondary) = if prefer_right {
        (right, left)
    } else {
        (left, right)
    };
    let secondary_is_side_action = is_side_action(&secondary.visibility_reasons)
        || super::target::is_squirrel_stub(&secondary);
    let same_target = describes_same_target(&primary, &secondary);
    if primary.description.is_none() {
        primary.description = secondary.description;
    }
    if primary.version.is_none() {
        primary.version = secondary.version;
    }
    if (primary.publisher.is_none()
        || primary
            .publisher
            .as_deref()
            .is_some_and(|value| value.starts_with("CN=")))
        && secondary
            .publisher
            .as_deref()
            .is_some_and(|value| !value.starts_with("CN="))
    {
        primary.publisher = secondary.publisher;
    }
    if primary.product_name.is_none() {
        primary.product_name = secondary.product_name;
    }
    if primary.original_filename.is_none() {
        primary.original_filename = secondary.original_filename;
    }
    if artifact_rank(secondary.artifact_kind) > artifact_rank(primary.artifact_kind) {
        primary.artifact_kind = secondary.artifact_kind;
    }
    if primary.artifact_kind != ArtifactKind::Application {
        primary.category = AppCategory::InstallersDocs;
    }
    if primary.install_location.is_none() {
        primary.install_location = secondary.install_location;
    }
    if primary.icon_base64.is_none() {
        primary.icon_base64 = secondary.icon_base64;
    }
    if primary.uninstall.is_none() {
        primary.uninstall = secondary.uninstall;
    }
    if primary.resolved_path.is_none() {
        primary.resolved_path = secondary.resolved_path;
    }
    if primary.shortcut_icon_path.is_none() {
        primary.shortcut_icon_path = secondary.shortcut_icon_path;
    }
    if primary.launch_arguments.is_none() && !secondary_is_side_action {
        primary.launch_arguments = secondary.launch_arguments;
    }
    if visibility_rank(secondary.visibility_class) > visibility_rank(primary.visibility_class) {
        primary.visibility_class = secondary.visibility_class;
    }
    primary.visibility_score = primary.visibility_score.max(secondary.visibility_score);
    let card_is_sticky = primary
        .visibility_reasons
        .iter()
        .any(crate::catalog::visibility::is_sticky_auxiliary);
    for reason in secondary.visibility_reasons {
        let survives_merge = crate::catalog::visibility::is_sticky_auxiliary(&reason)
            || reason == crate::catalog::VisibilityReason::ConsoleApplication;
        if survives_merge && same_target && !primary.visibility_reasons.contains(&reason) {
            primary.visibility_reasons.push(reason);
        }
    }
    if primary.visibility_class == crate::catalog::VisibilityClass::Primary && card_is_sticky {
        primary.visibility_class = crate::catalog::VisibilityClass::Auxiliary;
    }
    primary.can_uninstall |= secondary.can_uninstall || primary.uninstall.is_some();
    primary
}

fn describes_same_target(left: &AppInfo, right: &AppInfo) -> bool {
    fn target(app: &AppInfo) -> String {
        crate::catalog::place::normalized_path(
            app.resolved_path.as_deref().unwrap_or(app.path.as_str()),
        )
    }
    let left = target(left);
    !left.is_empty() && left == target(right)
}

fn artifact_rank(kind: ArtifactKind) -> u8 {
    match kind {
        ArtifactKind::Application => 0,
        ArtifactKind::Documentation => 1,
        ArtifactKind::Installer => 2,
    }
}

fn is_side_action(reasons: &[crate::catalog::VisibilityReason]) -> bool {
    use crate::catalog::VisibilityReason::{DocumentationShortcut, MaintenanceExecutable};
    reasons
        .iter()
        .any(|reason| matches!(reason, MaintenanceExecutable | DocumentationShortcut))
}

fn visibility_rank(class: crate::catalog::VisibilityClass) -> u8 {
    match class {
        crate::catalog::VisibilityClass::Rejected => 0,
        crate::catalog::VisibilityClass::Auxiliary => 1,
        crate::catalog::VisibilityClass::Primary => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{
        AppCategory, ArtifactKind, LaunchKind, VisibilityClass, VisibilityReason,
    };

    fn app(reasons: Vec<VisibilityReason>) -> AppInfo {
        AppInfo {
            id: "app".into(),
            name: "Example".into(),
            path: r"C:\Apps\Example\example.exe".into(),
            icon_base64: None,
            artifact_kind: Default::default(),
            category: AppCategory::Other,
            launch_kind: LaunchKind::Executable,
            source_kind: SourceKind::Registry,
            description: None,
            version: None,
            publisher: None,
            product_name: None,
            original_filename: None,
            install_location: Some(r"C:\Apps\Example".into()),
            can_uninstall: false,
            uninstall: None,
            resolved_path: None,
            shortcut_icon_path: None,
            launch_arguments: None,
            canonical_identity: None,
            preference_identity: None,
            visibility_class: VisibilityClass::Primary,
            visibility_score: 50,
            visibility_reasons: reasons,
            target_availability: None,
            category_reasons: Vec::new(),
            close_risk: None,
        }
    }

    #[test]
    fn merged_card_does_not_inherit_non_sticky_visibility_reasons() {
        let primary = app(Vec::new());
        let secondary = app(vec![VisibilityReason::DocumentationShortcut]);

        let merged = merge_app(primary, secondary);

        assert!(!merged
            .visibility_reasons
            .contains(&VisibilityReason::DocumentationShortcut));
    }

    #[test]
    fn a_siblings_component_reason_does_not_demote_a_registered_card() {
        let mut card = app(Vec::new());
        card.source_kind = SourceKind::StartMenu;
        card.path = r"C:\Menu\Visual Studio Code.lnk".into();
        card.resolved_path = Some(r"D:\Microsoft VS Code\Code.exe".into());
        card.visibility_score = 85;
        let mut component = app(vec![VisibilityReason::ProductComponent]);
        component.path = r"D:\Microsoft VS Code\resources\app\vsce-sign.exe".into();
        component.visibility_class = VisibilityClass::Auxiliary;

        let merged = merge_app(card, component);

        assert_eq!(merged.visibility_class, VisibilityClass::Primary);
        assert!(!merged
            .visibility_reasons
            .contains(&VisibilityReason::ProductComponent));
    }

    #[test]
    fn the_cards_own_sticky_reason_still_survives_a_promotion() {
        let mut prompt = app(vec![VisibilityReason::CommandEnvironment]);
        prompt.visibility_class = VisibilityClass::Auxiliary;
        let mut aumid_sibling = app(Vec::new());
        aumid_sibling.launch_kind = LaunchKind::AppUserModelId;
        aumid_sibling.visibility_class = VisibilityClass::Primary;

        let merged = merge_app(prompt, aumid_sibling);

        assert_eq!(merged.visibility_class, VisibilityClass::Auxiliary);
    }

    #[test]
    fn merged_card_preserves_artifact_kind_and_reserved_category() {
        let primary = app(Vec::new());
        let mut secondary = app(Vec::new());
        secondary.artifact_kind = ArtifactKind::Installer;

        let merged = merge_app(primary, secondary);

        assert_eq!(merged.artifact_kind, ArtifactKind::Installer);
        assert_eq!(merged.category, AppCategory::InstallersDocs);
    }
}
