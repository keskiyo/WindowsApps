use crate::catalog::{icon_cache, icon_source, AppInfo, LaunchKind};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppHydrationPatch {
    pub id: String,
    pub generation: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_base64: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publisher: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub install_location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub can_uninstall: Option<bool>,
}

pub fn batch_patches(patches: Vec<AppHydrationPatch>, limit: usize) -> Vec<Vec<AppHydrationPatch>> {
    let limit = limit.max(1);
    let mut batches = Vec::new();
    let mut current = Vec::with_capacity(limit);
    for patch in patches {
        current.push(patch);
        if current.len() == limit {
            batches.push(std::mem::take(&mut current));
            current = Vec::with_capacity(limit);
        }
    }
    if !current.is_empty() {
        batches.push(current);
    }
    batches
}

pub fn is_current_generation(generation: u64, patch: &AppHydrationPatch) -> bool {
    generation == patch.generation
}

pub fn hydrate(app_data_dir: &Path, apps: &[AppInfo], generation: u64) -> Vec<AppHydrationPatch> {
    apps.iter()
        .map(|app| hydrate_app(app_data_dir, app, generation))
        .collect()
}

pub fn prioritize_apps(mut apps: Vec<AppInfo>, priority_ids: &[String]) -> Vec<AppInfo> {
    let priority = priority_ids
        .iter()
        .enumerate()
        .map(|(index, id)| (id.as_str(), index))
        .collect::<HashMap<_, _>>();
    apps.sort_by_key(|app| {
        priority
            .get(app.id.as_str())
            .copied()
            .unwrap_or(priority_ids.len() + 1)
    });
    apps
}

fn hydrate_app(app_data_dir: &Path, app: &AppInfo, generation: u64) -> AppHydrationPatch {
    let target = app.resolved_path.as_deref().unwrap_or(&app.path);
    let metadata = Path::new(target)
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
        .then(|| crate::platform::windows::executable_metadata::read(Path::new(target)));
    let icon_base64 = hydrate_icon(app_data_dir, app);
    AppHydrationPatch {
        id: app.id.clone(),
        generation,
        icon_base64,
        description: app.description.clone().or_else(|| {
            metadata
                .as_ref()
                .and_then(|value| value.description.clone())
        }),
        version: app
            .version
            .clone()
            .or_else(|| metadata.as_ref().and_then(|value| value.version.clone())),
        publisher: app
            .publisher
            .clone()
            .or_else(|| metadata.as_ref().and_then(|value| value.publisher.clone())),
        install_location: app.install_location.clone().or_else(|| {
            Path::new(target)
                .parent()
                .map(|path| path.to_string_lossy().into_owned())
        }),
        can_uninstall: Some(app.uninstall.is_some()),
    }
}

fn hydrate_icon(app_data_dir: &Path, app: &AppInfo) -> Option<String> {
    let source = icon_source(app).unwrap_or_else(|| app.path.clone());
    let fingerprint = icon_cache::source_fingerprint(&source);
    if let Some(bytes) = icon_cache::read_icon(app_data_dir, &app.id, &fingerprint) {
        return Some(format!("data:image/png;base64,{}", STANDARD.encode(bytes)));
    }
    let data_url = if app.launch_kind == LaunchKind::AppUserModelId {
        crate::platform::windows::icon_extractor::extract_app_id_icon(&app.path)
    } else {
        crate::platform::windows::icon_extractor::extract_icon(Path::new(&source))
    }?;
    let encoded = data_url.split_once(',')?.1;
    let bytes = STANDARD.decode(encoded).ok()?;
    let _ = icon_cache::write_icon(app_data_dir, &app.id, &fingerprint, &bytes);
    Some(data_url)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn patch(index: usize) -> AppHydrationPatch {
        AppHydrationPatch {
            id: format!("app-{index}"),
            generation: 1,
            icon_base64: None,
            description: None,
            version: None,
            publisher: None,
            install_location: None,
            can_uninstall: None,
        }
    }

    #[test]
    fn batches_patches_without_exceeding_limit() {
        let batches = batch_patches((0..65).map(patch).collect(), 32);
        assert_eq!(
            batches.iter().map(Vec::len).collect::<Vec<_>>(),
            vec![32, 32, 1]
        );
    }

    #[test]
    fn rejects_patches_from_a_stale_generation() {
        assert!(!is_current_generation(4, &patch(0)));
        let mut current = patch(0);
        current.generation = 4;
        assert!(is_current_generation(4, &current));
    }

    #[test]
    fn prioritizes_visible_apps_before_background_hydration() {
        let apps = ["app-1", "app-2", "app-3"]
            .into_iter()
            .map(|id| AppInfo {
                id: id.into(),
                name: id.into(),
                path: format!(r"C:\{id}.exe"),
                icon_base64: None,
                category: crate::catalog::AppCategory::Other,
                launch_kind: LaunchKind::Executable,
                source_kind: crate::catalog::SourceKind::Registry,
                description: None,
                version: None,
                publisher: None,
                install_location: None,
                can_uninstall: false,
                uninstall: None,
                resolved_path: None,
                shortcut_icon_path: None,
            })
            .collect::<Vec<_>>();

        let ordered = prioritize_apps(apps, &["app-3".into(), "app-1".into()]);

        assert_eq!(
            ordered
                .iter()
                .map(|app| app.id.as_str())
                .collect::<Vec<_>>(),
            vec!["app-3", "app-1", "app-2"]
        );
    }
}
