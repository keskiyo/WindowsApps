//! Combining two records that resolved to the same application, and the resolved group itself.

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
    // Only the winning pair needs itemized evidence, so this is the one place that builds it.
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
        // On a tie, keep the 64-bit build when the two differ only by architecture.
        || (scores_tie && is_32bit_variant(&left) && !is_32bit_variant(&right));
    let (mut primary, secondary) = if prefer_right {
        (right, left)
    } else {
        (left, right)
    };
    // A Squirrel stub counts here for the same reason a maintenance shortcut does: its
    // `--processStart App.exe` describes how *the updater* reaches the product, and copying it onto
    // a card that already launches the product directly would pass the flag to the product itself.
    let secondary_is_side_action = is_side_action(&secondary.visibility_reasons)
        || super::target::is_squirrel_stub(&secondary);
    // Read before any field moves out of `secondary`, and before `primary` inherits its target.
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
    // Arguments decide what launching the card actually does, so they are never taken from an
    // entry that performs a side action on the product — a "reset preferences and cache files"
    // shortcut resolves to the very executable it wipes the settings of, and filling this blank
    // from it turned the merged player card into a settings eraser. Every other field here is
    // descriptive and safe to complete from a sibling.
    if primary.launch_arguments.is_none() && !secondary_is_side_action {
        primary.launch_arguments = secondary.launch_arguments;
    }
    if visibility_rank(secondary.visibility_class) > visibility_rank(primary.visibility_class) {
        primary.visibility_class = secondary.visibility_class;
    }
    primary.visibility_score = primary.visibility_score.max(secondary.visibility_score);
    // Only the record that became the card may pull the card out of the main catalog. A sibling's
    // reasons describe the sibling's own file, and letting them demote produced the worst class of
    // error this catalog has: World of Warcraft, 7-Zip File Manager, Visual Studio Code, TablePlus
    // and AIDA64 each scored 85 on their own Start Menu registration and still sat in Auxiliary
    // tools, because one component executable merged in and brought `ProductComponent` with it.
    let card_is_sticky = primary
        .visibility_reasons
        .iter()
        .any(crate::catalog::visibility::is_sticky_auxiliary);
    // A visibility reason describes an executable, so it may only travel between records that
    // describe the *same* executable. Two records merged by product family point at different
    // files: the Visual Studio Code shortcut launches `Code.exe`, while the `vsce-sign.exe` that
    // merged into it is a component of the same product. Copying regardless meant the card carried
    // a component reason into the next merge of the same group, where it read as the card's own and
    // demoted a registered application. Same-target siblings — a Start Menu shortcut and the Apps
    // Folder entry for it — still transfer, which is what keeps a merged Python IDLE auxiliary.
    for reason in secondary.visibility_reasons {
        let survives_merge = crate::catalog::visibility::is_sticky_auxiliary(&reason)
            || reason == crate::catalog::VisibilityReason::ConsoleApplication;
        if survives_merge && same_target && !primary.visibility_reasons.contains(&reason) {
            primary.visibility_reasons.push(reason);
        }
    }
    // The card's own sticky reason still survives promotion. Otherwise an AUMID sibling (Primary by
    // the launch-kind fast-path, since it carries no arguments to reveal itself) promoted the whole
    // card back to Primary — this is why a merged IDLE / Python prompt reappeared in the main
    // catalog. The predicate is shared with `classify_visibility` so the two cannot disagree.
    if primary.visibility_class == crate::catalog::VisibilityClass::Primary && card_is_sticky {
        primary.visibility_class = crate::catalog::VisibilityClass::Auxiliary;
    }
    primary.can_uninstall |= secondary.can_uninstall || primary.uninstall.is_some();
    primary
}

/// Whether two records launch the same executable. The resolved target is what a card actually
/// runs; its own path is the fallback for a record that resolves to nothing.
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

/// Reasons marking an entry as an action performed *on* a product rather than the product itself.
/// Shared with `candidate::candidate_score`, which keeps such a record from representing the card.
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

    /// A sibling's component reason must not pull a registered application out of the catalog.
    /// This is how World of Warcraft, 7-Zip File Manager, Visual Studio Code, TablePlus and
    /// AIDA64 Extreme ended up in Auxiliary tools while scoring 85 on their own Start Menu
    /// registration: one component executable merged in and brought `ProductComponent` along.
    #[test]
    fn a_siblings_component_reason_does_not_demote_a_registered_card() {
        let mut card = app(Vec::new());
        card.source_kind = SourceKind::StartMenu;
        card.path = r"C:\Menu\Visual Studio Code.lnk".into();
        card.resolved_path = Some(r"D:\Microsoft VS Code\Code.exe".into());
        card.visibility_score = 85;
        // A component of the same product, merged in by product family: a different executable.
        let mut component = app(vec![VisibilityReason::ProductComponent]);
        component.path = r"D:\Microsoft VS Code\resources\app\vsce-sign.exe".into();
        component.visibility_class = VisibilityClass::Auxiliary;

        let merged = merge_app(card, component);

        assert_eq!(merged.visibility_class, VisibilityClass::Primary);
        // The reason must not stick to the card either: the next merge in the same group would
        // read it as the card's own and demote what this merge just kept.
        assert!(!merged
            .visibility_reasons
            .contains(&VisibilityReason::ProductComponent));
    }

    /// The case the sticky rule exists for still holds: when the card itself is the command
    /// environment, an AUMID sibling promoted by the launch-kind fast-path cannot restore it.
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
