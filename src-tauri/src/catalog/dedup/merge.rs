//! Combining two records that resolved to the same application, and the resolved group itself.

use super::candidate::candidate_score;
use super::candidate::AppCandidate;
use super::evidence::score_evidence;
use super::evidence::Evidence;
use super::family::{is_32bit_variant, version_key};
use crate::catalog::{AppInfo, SourceKind};

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
    if primary.launch_arguments.is_none() {
        primary.launch_arguments = secondary.launch_arguments;
    }
    if visibility_rank(secondary.visibility_class) > visibility_rank(primary.visibility_class) {
        primary.visibility_class = secondary.visibility_class;
    }
    primary.visibility_score = primary.visibility_score.max(secondary.visibility_score);
    for reason in secondary.visibility_reasons {
        if !primary.visibility_reasons.contains(&reason) {
            primary.visibility_reasons.push(reason);
        }
    }
    // A sticky-auxiliary reason survives a merge. Otherwise an AUMID sibling (Primary by the
    // launch-kind fast-path, since it carries no arguments to reveal itself) promoted the whole
    // card back to Primary — this is why a merged IDLE / Python prompt reappeared in the main
    // catalog. The predicate is shared with `classify_visibility` so the two cannot disagree.
    if primary.visibility_class == crate::catalog::VisibilityClass::Primary
        && primary
            .visibility_reasons
            .iter()
            .any(crate::catalog::visibility::is_sticky_auxiliary)
    {
        primary.visibility_class = crate::catalog::VisibilityClass::Auxiliary;
    }
    primary.can_uninstall |= secondary.can_uninstall || primary.uninstall.is_some();
    primary
}

fn visibility_rank(class: crate::catalog::VisibilityClass) -> u8 {
    match class {
        crate::catalog::VisibilityClass::Rejected => 0,
        crate::catalog::VisibilityClass::Auxiliary => 1,
        crate::catalog::VisibilityClass::Primary => 2,
    }
}
