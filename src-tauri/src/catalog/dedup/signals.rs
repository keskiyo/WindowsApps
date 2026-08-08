use super::arguments::meaningful_launch_arguments;
use super::candidate::AppCandidate;
use super::family::normalize_name;
use super::family::normalized_publisher;
use super::target::normalize_path;
use crate::catalog::{path_is_within, AppInfo, LaunchKind, SourceKind};
use std::path::{Component, Path};

pub(super) fn conflicting_versions(left: &AppCandidate, right: &AppCandidate) -> bool {
    matches!(
        (
            normalized_version_string(&left.app),
            normalized_version_string(&right.app)
        ),
        (Some(left), Some(right)) if left != right
    )
}

pub(super) fn conflicting_install_roots(left: &AppCandidate, right: &AppCandidate) -> bool {
    matches!(
        (
            left.identity.install_root.as_ref(),
            right.identity.install_root.as_ref()
        ),
        (Some(left), Some(right))
            if left != right
                && !left.starts_with(&format!("{right}\\"))
                && !right.starts_with(&format!("{left}\\"))
    )
}

pub(super) fn same_folder_and_family(left: &AppCandidate, right: &AppCandidate) -> bool {
    left.parent.is_some()
        && left.parent == right.parent
        && left.launcher_family == right.launcher_family
}

pub(super) fn nested_install_root_and_family(left: &AppCandidate, right: &AppCandidate) -> bool {
    let left_root = left.identity.install_root.as_ref();
    let right_root = right.identity.install_root.as_ref();
    let same_family = left.launcher_family == right.launcher_family;
    same_family
        && matches!(
            (left_root, right_root),
            (Some(left), Some(right))
                if left.starts_with(&format!("{right}\\"))
                    || right.starts_with(&format!("{left}\\"))
        )
}

pub(super) fn same_folder_helper_variant(left: &AppCandidate, right: &AppCandidate) -> bool {
    left.parent.is_some()
        && left.parent == right.parent
        && left.helper_family == right.helper_family
        && (is_helper_candidate(&left.app) || is_helper_candidate(&right.app))
}

pub(super) fn shortcut_same_family(left: &AppCandidate, right: &AppCandidate) -> bool {
    (left.app.launch_kind == LaunchKind::Shortcut || right.app.launch_kind == LaunchKind::Shortcut)
        && left.launcher_family == right.launcher_family
}

pub(super) fn versioned_portable_copy(left: &AppCandidate, right: &AppCandidate) -> bool {
    left.app.source_kind == SourceKind::Portable
        && right.app.source_kind == SourceKind::Portable
        && left.family == right.family
        && (left.app.version.is_some() || right.app.version.is_some())
}

pub(super) fn same_version_portable_copy(left: &AppCandidate, right: &AppCandidate) -> bool {
    (left.app.source_kind == SourceKind::Portable || right.app.source_kind == SourceKind::Portable)
        && left.family == right.family
        && matches!(
            (
                normalized_version_string(&left.app),
                normalized_version_string(&right.app)
            ),
            (Some(left), Some(right)) if left == right
        )
}

pub(super) fn normalized_version_string(app: &AppInfo) -> Option<String> {
    app.version
        .as_deref()
        .map(|value| {
            value
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
                .to_lowercase()
        })
        .filter(|value| !value.is_empty())
}
pub(super) fn both_unversioned_portable_copies(left: &AppCandidate, right: &AppCandidate) -> bool {
    left.app.source_kind == SourceKind::Portable
        && right.app.source_kind == SourceKind::Portable
        && left.family == right.family
        && left.app.version.is_none()
        && right.app.version.is_none()
}

pub(super) fn same_portable_root(left: &AppCandidate, right: &AppCandidate) -> bool {
    shared(
        left.identity.install_root.as_ref(),
        right.identity.install_root.as_ref(),
    )
}

pub(super) fn is_helper_candidate(app: &AppInfo) -> bool {
    let value = format!("{} {}", normalize_name(&app.name), app.path.to_lowercase());
    [
        " helper",
        " updater",
        " update.exe",
        " crash reporter",
        " crashhandler",
        " service.exe",
    ]
    .iter()
    .any(|marker| value.contains(marker))
}

pub(super) fn shared(left: Option<&String>, right: Option<&String>) -> bool {
    matches!((left, right), (Some(left), Some(right)) if !left.is_empty() && left == right)
}

pub(super) fn shortcut_targets_executable(left: &AppInfo, right: &AppInfo) -> bool {
    let (shortcut, executable) = if left.launch_kind == LaunchKind::Shortcut {
        (left, right)
    } else if right.launch_kind == LaunchKind::Shortcut {
        (right, left)
    } else {
        return false;
    };
    let Some(target) = shortcut.resolved_path.as_deref() else {
        return false;
    };
    if meaningful_launch_arguments(shortcut.launch_arguments.as_deref()).is_some() {
        return false;
    }
    normalize_path(target) == normalize_path(&executable.path)
}

pub(super) fn registry_install_contains_exe(left: &AppCandidate, right: &AppCandidate) -> bool {
    let pairs = [(left, right), (right, left)];
    pairs.iter().any(|(registry, executable)| {
        registry.app.source_kind == SourceKind::Registry
            && executable.app.path.to_lowercase().ends_with(".exe")
            && registry.launcher_family == executable.launcher_family
            && registry.identity.install_root.as_ref().is_some_and(|root| {
                is_specific_install_root(root) && path_is_within(&executable.identity.path, root)
            })
    })
}

fn is_specific_install_root(root: &str) -> bool {
    Path::new(root)
        .components()
        .filter(|component| matches!(component, Component::Normal(_)))
        .take(2)
        .count()
        == 2
}

pub(super) fn same_package_family(left: &AppCandidate, right: &AppCandidate) -> bool {
    match (
        left.identity.aumid.as_deref(),
        right.identity.aumid.as_deref(),
    ) {
        (Some(left), Some(right)) => package_family(left) == package_family(right),
        _ => false,
    }
}

fn package_family(aumid: &str) -> &str {
    aumid.split_once('!').map_or(aumid, |(family, _)| family)
}

pub(super) fn publishers_conflict(left: &AppInfo, right: &AppInfo) -> bool {
    let left = normalized_publisher(left.publisher.as_deref());
    let right = normalized_publisher(right.publisher.as_deref());
    !left.is_empty() && !right.is_empty() && left != right
}
