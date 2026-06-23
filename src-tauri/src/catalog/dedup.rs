use crate::catalog::{AppCategory, AppInfo, LaunchKind, SourceKind};
use sha2::{Digest, Sha256};
use std::path::Path;

pub(super) fn deduplicate(
    apps: Vec<AppInfo>,
    classify: impl Fn(&str, &str) -> AppCategory,
) -> Vec<AppInfo> {
    let mut unique = Vec::<AppInfo>::new();
    for app in apps {
        if let Some(index) = unique
            .iter()
            .position(|existing| same_application(existing, &app))
        {
            let existing = unique.remove(index);
            unique.insert(index, merge_app(existing, app));
        } else {
            unique.push(app);
        }
    }
    let mut apps = unique;
    for app in &mut apps {
        app.name = app.name.split_whitespace().collect::<Vec<_>>().join(" ");
        app.id = format!("{:x}", Sha256::digest(app.path.to_lowercase().as_bytes()));
        app.category = classify(&app.name, &app.path);
    }
    apps.sort_by_cached_key(|app| (category_rank(app.category), app.name.to_lowercase()));
    apps
}

fn same_application(left: &AppInfo, right: &AppInfo) -> bool {
    if left.path.eq_ignore_ascii_case(&right.path) {
        return true;
    }
    let left_identity = left.resolved_path.as_deref().unwrap_or(&left.path);
    let right_identity = right.resolved_path.as_deref().unwrap_or(&right.path);
    if left_identity.eq_ignore_ascii_case(right_identity) {
        return true;
    }
    let left_name = normalized_product_family(&left.name);
    let right_name = normalized_product_family(&right.name);
    if left_name != right_name {
        if !(one_is_shortcut(left, right)
            && same_parent_folder(left_identity, right_identity)
            && launcher_product_family(&left_name) == launcher_product_family(&right_name))
        {
            return false;
        }
    }
    if left.launch_kind == LaunchKind::AppUserModelId
        || right.launch_kind == LaunchKind::AppUserModelId
    {
        return true;
    }
    if one_is_shortcut(left, right) {
        return true;
    }
    match (&left.publisher, &right.publisher) {
        (Some(left), Some(right)) => left.eq_ignore_ascii_case(right),
        _ => true,
    }
}

fn one_is_shortcut(left: &AppInfo, right: &AppInfo) -> bool {
    left.launch_kind == LaunchKind::Shortcut || right.launch_kind == LaunchKind::Shortcut
}

fn same_parent_folder(left: &str, right: &str) -> bool {
    let left_parent = Path::new(left).parent();
    let right_parent = Path::new(right).parent();
    matches!((left_parent, right_parent), (Some(left), Some(right)) if left.to_string_lossy().eq_ignore_ascii_case(&right.to_string_lossy()))
}

fn launcher_product_family(name: &str) -> &str {
    name.strip_suffix(" launcher").unwrap_or(name).trim()
}

fn merge_app(left: AppInfo, right: AppInfo) -> AppInfo {
    let prefer_right = candidate_score(&right) > candidate_score(&left)
        || (candidate_score(&right) == candidate_score(&left)
            && left.source_kind == SourceKind::Portable
            && right.source_kind == SourceKind::Portable
            && version_key(right.version.as_deref()) > version_key(left.version.as_deref()));
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
    if primary.publisher.is_none()
        || primary
            .publisher
            .as_deref()
            .is_some_and(|value| value.starts_with("CN="))
    {
        if secondary
            .publisher
            .as_deref()
            .is_some_and(|value| !value.starts_with("CN="))
        {
            primary.publisher = secondary.publisher;
        }
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
    primary.can_uninstall |= secondary.can_uninstall || primary.uninstall.is_some();
    primary
}

fn version_key(version: Option<&str>) -> Vec<u64> {
    version
        .unwrap_or_default()
        .split(|character: char| !character.is_ascii_digit())
        .filter_map(|segment| segment.parse().ok())
        .collect()
}

pub(super) fn normalize_name(name: &str) -> String {
    name.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

pub(super) fn normalized_product_family(name: &str) -> String {
    let mut value = normalize_name(name);
    for marker in [
        " (64bit)",
        " (32bit)",
        " (64-bit)",
        " (32-bit)",
        " x64",
        " x86",
    ] {
        if value.ends_with(marker) {
            value.truncate(value.len() - marker.len());
        }
    }
    if let Some((family, suffix)) = value.split_once(" - ") {
        let generic_suffix = ["proxy utility", "desktop app", "application"]
            .iter()
            .any(|marker| suffix.starts_with(marker));
        let has_version = suffix.chars().any(|character| character.is_ascii_digit());
        if generic_suffix && has_version {
            return family.trim().to_string();
        }
    }
    let mut family = version_family(&value).trim().to_string();
    if family.ends_with(" version") {
        family.truncate(family.len() - " version".len());
    }
    family
}

fn version_family(name: &str) -> &str {
    let Some((family, suffix)) = name.rsplit_once(' ') else {
        return name;
    };
    if !suffix.starts_with(|character: char| character.is_ascii_digit()) {
        return name;
    }
    let numeric_segments = suffix
        .split(|character: char| !character.is_ascii_digit())
        .filter(|segment| !segment.is_empty())
        .count();
    if numeric_segments >= 2 {
        family
    } else {
        name
    }
}

fn candidate_score(app: &AppInfo) -> u8 {
    if app.source_kind == SourceKind::Steam {
        return 5;
    }
    match Path::new(&app.path)
        .extension()
        .and_then(|value| value.to_str())
    {
        Some(extension) if extension.eq_ignore_ascii_case("lnk") => return 4,
        Some(extension) if extension.eq_ignore_ascii_case("exe") => return 3,
        _ => {}
    }
    if app.launch_kind == LaunchKind::AppUserModelId {
        2
    } else {
        0
    }
}

fn category_rank(category: AppCategory) -> u8 {
    match category {
        AppCategory::Games => 0,
        AppCategory::Ai => 1,
        AppCategory::Editors => 2,
        AppCategory::Development => 3,
        AppCategory::Browsers => 4,
        AppCategory::Media => 5,
        AppCategory::Communication => 6,
        AppCategory::Utilities => 7,
        AppCategory::System => 8,
        AppCategory::WindowsFeatures => 9,
        AppCategory::Other => 10,
    }
}
