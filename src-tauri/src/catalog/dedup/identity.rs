//! Stable identity: the catalog id a card keeps across scans, the preference identity that
//! survives a dedup rule change, and the launch target that identifies what actually runs.

use super::arguments::meaningful_launch_arguments;
use super::candidate::CandidateIdentity;
use super::family::{normalized_product_family, normalized_publisher};
use super::merge::ResolvedApp;
use super::target::{launch_target, normalize_path, steam_app_id};
use crate::catalog::{AppInfo, LaunchKind, SourceKind};
use sha2::{Digest, Sha256};
use std::path::Path;

pub(in crate::catalog) fn canonical_id(app: &AppInfo) -> String {
    let identity = CandidateIdentity::from_app(app);
    if let Some(app_id) = identity.steam_app_id {
        return format!("steam:{}", app_id.to_lowercase());
    }
    if let Some(aumid) = identity.aumid {
        return format!("aumid:{aumid}");
    }
    if let Some(target) = identity.launch_target {
        return canonical_target_id(&target, app.launch_arguments.as_deref());
    }
    if let Some(registry_product) = identity.registry_product {
        if !registry_product.trim_matches('|').is_empty() {
            return format!("registry:{registry_product}");
        }
    }
    if let Some(portable_product) = identity.portable_product {
        if !portable_product.trim_matches('|').is_empty() {
            return format!("portable:{portable_product}");
        }
    }
    format!("path:{}", identity.path)
}

pub(super) fn resolved_canonical_id(resolved: &ResolvedApp) -> String {
    let identities = resolved
        .candidates
        .iter()
        .map(|candidate| &candidate.identity)
        .collect::<Vec<_>>();
    if let Some(app_id) = identities
        .iter()
        .find_map(|identity| identity.steam_app_id.as_ref())
    {
        return format!("steam:{}", app_id.to_lowercase());
    }
    if let Some(aumid) = identities
        .iter()
        .find_map(|identity| identity.aumid.as_ref())
    {
        return format!("aumid:{aumid}");
    }
    if let Some(target) = identities
        .iter()
        .find_map(|identity| identity.launch_target.as_ref())
    {
        return canonical_target_id(target, resolved.app.launch_arguments.as_deref());
    }
    if let Some(registry_product) = identities
        .iter()
        .find_map(|identity| identity.registry_product.as_ref())
        .filter(|value| !value.trim_matches('|').is_empty())
    {
        return format!("registry:{registry_product}");
    }
    if let Some(portable_product) = identities
        .iter()
        .find_map(|identity| identity.portable_product.as_ref())
        .filter(|value| !value.trim_matches('|').is_empty())
    {
        return format!("portable:{portable_product}");
    }
    canonical_id(&resolved.app)
}

fn canonical_target_id(target: &str, arguments: Option<&str>) -> String {
    match meaningful_launch_arguments(arguments) {
        Some(mode) => format!("target:{target}|mode:{:x}", Sha256::digest(mode.as_bytes())),
        None => format!("target:{target}"),
    }
}

pub(in crate::catalog) fn preference_identity(app: &AppInfo) -> String {
    let raw = if let Some(app_id) = steam_app_id(app) {
        format!("steam:{}", app_id.to_lowercase())
    } else if app.launch_kind == LaunchKind::AppUserModelId {
        format!("aumid:{}", app.path.trim().to_lowercase())
    } else {
        let product = app
            .product_name
            .as_deref()
            .map(normalized_product_family)
            .filter(|value| !value.is_empty());
        let publisher = normalized_publisher(app.publisher.as_deref());
        let install_root = app.install_location.as_deref().map(normalize_path);
        if let (Some(product), Some(root)) = (product, install_root.filter(|root| !root.is_empty()))
        {
            if !publisher.is_empty() {
                format!("product:{publisher}|{product}|{root}")
            } else if app.source_kind == SourceKind::Portable {
                format!("portable:{product}|{root}")
            } else if let Some(target) = preference_target(app) {
                format!("target:{target}")
            } else {
                format!("path:{}", normalize_path(&app.path))
            }
        } else if let Some(target) = preference_target(app) {
            format!("target:{target}")
        } else {
            format!("path:{}", normalize_path(&app.path))
        }
    };
    format!("identity:{:x}", Sha256::digest(raw.as_bytes()))
}

pub(super) fn card_preference_identity(app: &AppInfo, product_identity: &str) -> String {
    let launch_role = match (app.source_kind, app.launch_kind) {
        (SourceKind::Steam, _) => "steam".to_owned(),
        (_, LaunchKind::AppUserModelId) => "app_user_model_id".to_owned(),
        _ => {
            let target = app.resolved_path.as_deref().unwrap_or(&app.path);
            Path::new(target.trim().trim_matches('"'))
                .file_stem()
                .and_then(|value| value.to_str())
                .map(str::to_lowercase)
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| normalized_product_family(&app.name))
        }
    };
    let launch_mode =
        meaningful_launch_arguments(app.launch_arguments.as_deref()).unwrap_or_default();
    let raw = format!("{product_identity}|role:{launch_role}|mode:{launch_mode}");
    format!("preference:{:x}", Sha256::digest(raw.as_bytes()))
}

fn preference_target(app: &AppInfo) -> Option<String> {
    let target = normalize_path(launch_target(app)?);
    Some(
        match meaningful_launch_arguments(app.launch_arguments.as_deref()) {
            Some(mode) => format!("{target}|mode:{mode}"),
            None => target,
        },
    )
}
