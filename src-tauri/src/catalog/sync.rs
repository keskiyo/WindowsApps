use crate::catalog::cache::CatalogCache;
use crate::catalog::incremental::{scan_root, FilesystemIndex, ScanMode};
use crate::catalog::scan_settings::ScanSettings;
use crate::catalog::source::{merge_sources, SourceKey, SourceSnapshot};
use crate::catalog::{self, AppInfo, ScanProgress};
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum SyncRequest {
    Watch,
    Startup,
    Refresh,
    Force,
}

impl SyncRequest {
    pub fn is_interactive(self) -> bool {
        matches!(self, Self::Refresh | Self::Force)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogChangeSummary {
    pub added: usize,
    pub removed: usize,
    pub updated: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogDelta {
    pub generation: u64,
    pub upserted: Vec<AppInfo>,
    pub removed_ids: Vec<String>,
    pub summary: CatalogChangeSummary,
}

pub fn compute_delta(generation: u64, previous: &[AppInfo], current: &[AppInfo]) -> CatalogDelta {
    let old = previous
        .iter()
        .map(|app| (app.id.as_str(), app))
        .collect::<HashMap<_, _>>();
    let new = current
        .iter()
        .map(|app| (app.id.as_str(), app))
        .collect::<HashMap<_, _>>();
    let mut upserted = current
        .iter()
        .filter(|app| old.get(app.id.as_str()).map_or(true, |old| *old != *app))
        .cloned()
        .collect::<Vec<_>>();
    let mut removed_ids = previous
        .iter()
        .filter(|app| !new.contains_key(app.id.as_str()))
        .map(|app| app.id.clone())
        .collect::<Vec<_>>();
    upserted.sort_by_cached_key(|app| app.id.clone());
    removed_ids.sort();
    let added = upserted
        .iter()
        .filter(|app| !old.contains_key(app.id.as_str()))
        .count();
    let updated = upserted.len().saturating_sub(added);
    CatalogDelta {
        generation,
        upserted,
        summary: CatalogChangeSummary {
            added,
            removed: removed_ids.len(),
            updated,
        },
        removed_ids,
    }
}

pub fn synchronize(
    previous: &CatalogCache,
    settings: &ScanSettings,
    request: SyncRequest,
    progress: impl Fn(ScanProgress),
    is_cancelled: impl Fn() -> bool + Sync,
) -> CatalogCache {
    progress(ScanProgress {
        stage: "Windows applications".into(),
        location: None,
        completed_roots: 0,
        total_roots: 0,
    });
    let (mut windows_apps, registry_metadata) = catalog::scan_registry();
    windows_apps.extend(catalog::scan_start_menu());
    windows_apps.extend(catalog::start_apps::scan());

    let libraries = catalog::steam::installed_libraries();
    let mut steam_apps = Vec::new();
    progress(ScanProgress {
        stage: "Steam libraries".into(),
        location: None,
        completed_roots: 0,
        total_roots: libraries.len(),
    });
    for (index, library) in libraries.iter().enumerate() {
        if is_cancelled() {
            break;
        }
        steam_apps.extend(
            catalog::steam::scan_library(library)
                .into_iter()
                .map(catalog::steam_app),
        );
        progress(ScanProgress {
            stage: "Steam libraries".into(),
            location: Some(library.to_string_lossy().into_owned()),
            completed_roots: index + 1,
            total_roots: libraries.len(),
        });
    }

    let mut roots = if settings.auto_scan_fixed_drives {
        crate::platform::windows::drives::fixed_drive_roots()
    } else {
        Vec::new()
    };
    roots.extend(
        settings
            .included_paths
            .iter()
            .map(PathBuf::from)
            .filter(|path| path.is_dir()),
    );
    roots.sort_by_cached_key(|path| path.to_string_lossy().to_lowercase());
    roots.dedup_by(|left, right| {
        left.to_string_lossy()
            .eq_ignore_ascii_case(&right.to_string_lossy())
    });
    let mut excluded = catalog::default_portable_exclusions();
    excluded.extend(settings.excluded_paths.iter().map(PathBuf::from));
    excluded.extend(libraries);
    let mode = if request == SyncRequest::Force {
        ScanMode::Force
    } else {
        ScanMode::Incremental
    };
    let mut portable_apps = Vec::new();
    let mut filesystem_index = FilesystemIndex::default();
    progress(ScanProgress {
        stage: "Portable applications".into(),
        location: None,
        completed_roots: 0,
        total_roots: roots.len(),
    });
    for (index, root) in roots.iter().enumerate() {
        if is_cancelled() {
            break;
        }
        let scanned = scan_root(
            root,
            &previous.filesystem_index,
            mode,
            &excluded,
            &is_cancelled,
        );
        let limit_reached = scanned.limit_reached;
        portable_apps.extend(scanned.apps);
        filesystem_index
            .directories
            .extend(scanned.index.directories);
        progress(ScanProgress {
            stage: limit_reached.map_or_else(
                || "Portable applications".into(),
                |limit| format!("Portable applications · {}", limit.message()),
            ),
            location: Some(root.to_string_lossy().into_owned()),
            completed_roots: index + 1,
            total_roots: roots.len(),
        });
    }

    let updates = vec![
        SourceSnapshot {
            key: SourceKey("windows".into()),
            fingerprint: None,
            apps: windows_apps,
        },
        SourceSnapshot {
            key: SourceKey("steam".into()),
            fingerprint: None,
            apps: steam_apps,
        },
        SourceSnapshot {
            key: SourceKey("portable".into()),
            fingerprint: None,
            apps: portable_apps,
        },
    ];
    let merged = merge_sources(previous.sources.clone(), updates);
    let mut apps = merged.apps;
    catalog::attach_registry_metadata(&mut apps, &registry_metadata);
    apps = catalog::sanitize(apps);
    for app in &mut apps {
        app.icon_base64 = None;
    }
    CatalogCache {
        schema_version: crate::catalog::cache::CACHE_SCHEMA_VERSION,
        generation: previous.generation.saturating_add(1),
        apps,
        sources: merged.sources,
        filesystem_index,
        last_successful_sync: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()
            .map(|duration| duration.as_secs()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{AppCategory, AppInfo, LaunchKind, SourceKind};

    fn app(id: &str, name: &str) -> AppInfo {
        AppInfo {
            id: id.into(),
            name: name.into(),
            path: format!(r"C:\{name}.exe"),
            icon_base64: None,
            category: AppCategory::Other,
            launch_kind: LaunchKind::Executable,
            source_kind: SourceKind::Registry,
            description: None,
            version: None,
            publisher: None,
            install_location: None,
            can_uninstall: false,
            uninstall: None,
            resolved_path: None,
            shortcut_icon_path: None,
        }
    }

    #[test]
    fn computes_stable_id_delta_and_summary() {
        let old = vec![app("removed", "Old"), app("same", "Editor")];
        let mut changed = app("same", "Editor");
        changed.version = Some("2".into());
        let new = vec![changed, app("added", "New")];

        let delta = compute_delta(4, &old, &new);

        assert_eq!(delta.generation, 4);
        assert_eq!(delta.removed_ids, vec!["removed"]);
        assert_eq!(delta.upserted.len(), 2);
        assert_eq!(delta.summary.added, 1);
        assert_eq!(delta.summary.removed, 1);
        assert_eq!(delta.summary.updated, 1);
    }

    #[test]
    fn post_metadata_sanitize_collapses_shortcut_registry_duplicates() {
        let mut shortcut = app("firefox-shortcut", "Firefox");
        shortcut.path = r"C:\Menu\Firefox.lnk".into();
        shortcut.launch_kind = LaunchKind::Shortcut;
        shortcut.source_kind = SourceKind::StartMenu;
        shortcut.resolved_path = Some(r"C:\Program Files\Mozilla Firefox\firefox.exe".into());
        shortcut.publisher = Some("Mozilla Foundation".into());
        shortcut.install_location = Some(r"C:\Users\Maks\Desktop".into());
        let mut registry = app("firefox-registry", "Mozilla Firefox (x64 ru)");
        registry.path = r"C:\Program Files\Mozilla Firefox\firefox.exe".into();
        registry.publisher = Some("Mozilla".into());
        registry.install_location = Some(r"C:\Program Files\Mozilla Firefox".into());

        let apps = catalog::sanitize(vec![shortcut, registry]);

        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].name, "Firefox");
    }
}
