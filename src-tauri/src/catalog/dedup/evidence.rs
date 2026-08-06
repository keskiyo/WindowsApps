//! The merge decision: what counts as evidence that two candidates are one application, how
//! strong each signal is, and the vetoes that outrank a weak signal.

use super::candidate::AppCandidate;

use super::signals::{
    both_unversioned_portable_copies, conflicting_install_roots, conflicting_versions,
    nested_install_root_and_family, publishers_conflict, registry_install_contains_exe,
    same_folder_and_family, same_folder_helper_variant, same_package_family, same_portable_root,
    same_version_portable_copy, shared, shortcut_same_family, shortcut_targets_executable,
    versioned_portable_copy,
};
use super::target::system_tool_alias;

use super::merge::ResolvedApp;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct Evidence {
    pub(super) reason: EvidenceReason,
    pub(super) score: u16,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum EvidenceReason {
    SamePath,
    SameLaunchTarget,
    SameSystemToolAlias,
    ShortcutTargetsExecutable,
    SameSteamAppId,
    SameAumid,
    SameSquirrelPackage,
    SameFolderAndFamily,
    SameInstallRootAndFamily,
    NestedInstallRootAndFamily,
    RegistryInstallContainsExecutable,
    SamePublisherAndFamily,
    SamePackagedFamily,
    ShortcutSameFamily,
    VersionedPortableCopy,
    SameVersionPortableCopy,
    SameFolderHelperVariant,
    SameFamily,
    NamePrefixOnly,
}

/// The strongest signals, which merge a pair even when the numeric score alone would not.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct EvidenceSummary {
    pub(super) score: u16,
    pub(super) has_identity_match: bool,
}

pub(super) fn should_merge(existing: &ResolvedApp, candidate: &AppCandidate) -> bool {
    if existing.candidates.iter().any(|left| {
        left.app.artifact_kind != candidate.app.artifact_kind
            && !summarize_evidence(left, candidate).has_identity_match
    }) {
        return false;
    }
    if existing.candidates.iter().any(|left| {
        both_unversioned_portable_copies(left, candidate) && !same_portable_root(left, candidate)
    }) {
        return false;
    }
    if existing.candidates.iter().any(|left| {
        left.family == candidate.family
            && matches!(
                (
                    left.identity.launch_mode.as_ref(),
                    candidate.identity.launch_mode.as_ref()
                ),
                (Some(left), Some(right)) if left != right
            )
    }) {
        return false;
    }
    let best = existing
        .candidates
        .iter()
        .map(|left| summarize_evidence(left, candidate))
        .max_by_key(|summary| summary.score);
    let Some(summary) = best else {
        return false;
    };
    // Weak, name-only evidence (score < 75) must not merge across a conflicting install root or a
    // conflicting version: those are two different installs, or two different releases the user
    // treats as different applications (7-Zip 22 vs 24, an old and new portable copy). Strong
    // structural evidence (>= 75: same or nested install root, same product folder, identity)
    // overrides — a launcher and its game exe in one install tree carry different versions but are
    // one application, so they still merge.
    if summary.score < 75
        && existing.candidates.iter().any(|left| {
            conflicting_install_roots(left, candidate) || conflicting_versions(left, candidate)
        })
    {
        return false;
    }
    if summary.score >= 80 {
        return true;
    }
    if summary.score >= 50 && !publishers_conflict(&existing.app, &candidate.app) {
        return true;
    }
    summary.has_identity_match
}

/// Score without building an evidence list. `should_merge` runs this for every candidate pair,
/// so allocating a `Vec` per comparison cost one allocation per pair across the whole catalog;
/// only the pair that actually wins a merge needs the itemized evidence.
fn summarize_evidence(left: &AppCandidate, right: &AppCandidate) -> EvidenceSummary {
    let mut summary = EvidenceSummary::default();
    collect_evidence(left, right, |reason, score| {
        summary.score = summary.score.max(score);
        summary.has_identity_match |= matches!(
            reason,
            EvidenceReason::SamePath
                | EvidenceReason::SameLaunchTarget
                | EvidenceReason::ShortcutTargetsExecutable
                | EvidenceReason::SameSteamAppId
                | EvidenceReason::SameAumid
                | EvidenceReason::SameSquirrelPackage
        );
    });
    summary
}

pub(super) fn score_evidence(left: &AppCandidate, right: &AppCandidate) -> (Vec<Evidence>, u16) {
    let mut evidence = Vec::new();
    collect_evidence(left, right, |reason, score| {
        evidence.push(Evidence { reason, score })
    });
    let score = evidence.iter().map(|item| item.score).max().unwrap_or(0);
    (evidence, score)
}

/// Single source of truth for what counts as evidence. Both the allocating and the
/// non-allocating consumers above feed off this, so the two can never drift apart.
fn collect_evidence(
    left: &AppCandidate,
    right: &AppCandidate,
    mut add: impl FnMut(EvidenceReason, u16),
) {
    if left.identity.path == right.identity.path {
        add(EvidenceReason::SamePath, 100);
    }
    if shared(
        left.identity.launch_target.as_ref(),
        right.identity.launch_target.as_ref(),
    ) && left.identity.launch_mode == right.identity.launch_mode
    {
        add(EvidenceReason::SameLaunchTarget, 100);
    }
    if shortcut_targets_executable(&left.app, &right.app) {
        add(EvidenceReason::ShortcutTargetsExecutable, 100);
    }
    if let (Some(left_alias), Some(right_alias)) =
        (system_tool_alias(&left.app), system_tool_alias(&right.app))
    {
        if left_alias == right_alias {
            add(EvidenceReason::SameSystemToolAlias, 90);
        }
    }
    if shared(
        left.identity.steam_app_id.as_ref(),
        right.identity.steam_app_id.as_ref(),
    ) {
        add(EvidenceReason::SameSteamAppId, 100);
    }
    if shared(left.identity.aumid.as_ref(), right.identity.aumid.as_ref()) {
        add(EvidenceReason::SameAumid, 100);
    }
    if shared(
        left.squirrel_package.as_ref(),
        right.squirrel_package.as_ref(),
    ) {
        add(EvidenceReason::SameSquirrelPackage, 100);
    }
    if left.family == right.family
        && shared(
            left.identity.install_root.as_ref(),
            right.identity.install_root.as_ref(),
        )
    {
        add(EvidenceReason::SameInstallRootAndFamily, 80);
    }
    if nested_install_root_and_family(left, right) {
        add(EvidenceReason::NestedInstallRootAndFamily, 75);
    }
    if same_folder_and_family(left, right) {
        add(EvidenceReason::SameFolderAndFamily, 80);
    }
    if registry_install_contains_exe(left, right) {
        add(EvidenceReason::RegistryInstallContainsExecutable, 75);
    }
    if left.family == right.family
        && !left.publisher_key.is_empty()
        && left.publisher_key == right.publisher_key
    {
        add(EvidenceReason::SamePublisherAndFamily, 60);
    }
    if same_package_family(left, right) {
        add(EvidenceReason::SamePackagedFamily, 80);
    }
    if shortcut_same_family(left, right) {
        add(EvidenceReason::ShortcutSameFamily, 60);
    }
    if versioned_portable_copy(left, right) {
        add(EvidenceReason::VersionedPortableCopy, 60);
    }
    // Same product, same exact version, with a portable copy involved: one program in two places.
    // Score 80 so it merges outright — past the install-root veto (roots differ by definition) and
    // past a vendor-name variant. Different versions are different programs and never reach here.
    if same_version_portable_copy(left, right) {
        add(EvidenceReason::SameVersionPortableCopy, 80);
    }
    if same_folder_helper_variant(left, right) {
        add(EvidenceReason::SameFolderHelperVariant, 80);
    }
    if left.family == right.family {
        add(EvidenceReason::SameFamily, 60);
    } else if left.launcher_family == right.launcher_family {
        add(EvidenceReason::NamePrefixOnly, 10);
    }
}
