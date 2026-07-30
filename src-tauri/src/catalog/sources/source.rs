use crate::catalog::AppInfo;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub(crate) const REGISTRY_SOURCE: &str = "registry";
pub(crate) const START_MENU_SOURCE: &str = "start-menu";
pub(crate) const START_APPS_SOURCE: &str = "start-apps";

/// Schema versions before 5 stored the registry, Start Menu and Start-Apps scanners under one
/// combined key. A cache carrying it must have it dropped on upgrade, otherwise its stale apps
/// would be merged in forever alongside the per-scanner snapshots that replaced it.
pub(crate) const LEGACY_COMBINED_SOURCE: &str = "windows";

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub(crate) struct SourceKey(pub String);

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SourceFingerprint {
    pub modified_nanos: u128,
    pub size: u64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SourceSnapshot {
    pub key: SourceKey,
    pub fingerprint: Option<SourceFingerprint>,
    pub apps: Vec<AppInfo>,
}

pub(crate) struct MergedSources {
    pub sources: Vec<SourceSnapshot>,
    /// Raw concatenation of every snapshot, **not** sanitized. Deduplication runs once in
    /// `sync::synchronize`, after registry metadata has been attached: publisher and install
    /// location arrive with that metadata and are exactly what lets duplicates be recognized,
    /// so sanitizing here as well would be both a wasted quadratic pass and a pass over
    /// poorer data.
    pub apps: Vec<AppInfo>,
}

pub(crate) fn merge_sources(
    previous: Vec<SourceSnapshot>,
    updates: Vec<SourceSnapshot>,
) -> MergedSources {
    let mut sources = previous
        .into_iter()
        .map(|snapshot| (snapshot.key.clone(), snapshot))
        .collect::<BTreeMap<_, _>>();
    for snapshot in updates {
        sources.insert(snapshot.key.clone(), snapshot);
    }
    let sources = sources.into_values().collect::<Vec<_>>();
    let apps = sources
        .iter()
        .flat_map(|snapshot| snapshot.apps.iter().cloned())
        .collect();
    MergedSources { sources, apps }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{AppCategory, AppInfo, LaunchKind, SourceKind};

    fn app(name: &str, path: &str) -> AppInfo {
        AppInfo {
            id: path.into(),
            name: name.into(),
            path: path.into(),
            icon_base64: None,
            category: AppCategory::Other,
            launch_kind: LaunchKind::Executable,
            source_kind: SourceKind::Registry,
            description: None,
            version: None,
            publisher: None,
            product_name: None,
            original_filename: None,
            install_location: None,
            can_uninstall: false,
            uninstall: None,
            resolved_path: None,
            shortcut_icon_path: None,
            launch_arguments: None,
            canonical_identity: None,
            preference_identity: None,
            visibility_class: Default::default(),
            visibility_score: 0,
            visibility_reasons: Vec::new(),
        }
    }

    fn snapshot(key: &str, apps: Vec<AppInfo>) -> SourceSnapshot {
        SourceSnapshot {
            key: SourceKey(key.into()),
            fingerprint: None,
            apps,
        }
    }

    // The whole point of per-scanner keys: a scanner that failed is simply left out of the
    // updates, so its previous apps survive instead of being replaced by an empty snapshot.
    // Before the split, one transient PowerShell failure removed every Store application.
    #[test]
    fn a_source_left_out_of_the_updates_keeps_its_previous_apps() {
        let previous = vec![
            snapshot(START_APPS_SOURCE, vec![app("Store App", "store.aumid")]),
            snapshot(REGISTRY_SOURCE, vec![app("Editor", "editor.exe")]),
        ];
        // Start-Apps failed this round, so only the registry snapshot is refreshed.
        let updates = vec![snapshot(
            REGISTRY_SOURCE,
            vec![app("Editor", "editor.exe"), app("Viewer", "viewer.exe")],
        )];

        let merged = merge_sources(previous, updates);

        assert!(merged.apps.iter().any(|app| app.name == "Store App"));
        assert!(merged.apps.iter().any(|app| app.name == "Viewer"));
    }

    #[test]
    fn replaces_only_the_successful_dirty_source() {
        let old = vec![
            snapshot("start-menu", vec![app("Old", "old.lnk")]),
            snapshot("registry:hklm", vec![app("Editor", "editor.exe")]),
        ];
        let updates = vec![snapshot("start-menu", vec![app("New", "new.lnk")])];

        let merged = merge_sources(old, updates);

        assert!(merged.apps.iter().any(|app| app.name == "New"));
        assert!(merged.apps.iter().any(|app| app.name == "Editor"));
        assert!(!merged.apps.iter().any(|app| app.name == "Old"));
    }
}
