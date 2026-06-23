use crate::catalog::{sanitize, AppInfo};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct SourceKey(pub String);

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceFingerprint {
    pub modified_nanos: u128,
    pub size: u64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceSnapshot {
    pub key: SourceKey,
    pub fingerprint: Option<SourceFingerprint>,
    pub apps: Vec<AppInfo>,
}

pub enum SourceUpdate {
    Success(SourceSnapshot),
    Failed { key: SourceKey, message: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceError {
    pub key: SourceKey,
    pub message: String,
}

pub struct MergedSources {
    pub sources: Vec<SourceSnapshot>,
    pub apps: Vec<AppInfo>,
    pub errors: Vec<SourceError>,
}

pub fn merge_sources(previous: Vec<SourceSnapshot>, updates: Vec<SourceUpdate>) -> MergedSources {
    let mut sources = previous
        .into_iter()
        .map(|snapshot| (snapshot.key.clone(), snapshot))
        .collect::<BTreeMap<_, _>>();
    let mut errors = Vec::new();
    for update in updates {
        match update {
            SourceUpdate::Success(snapshot) => {
                sources.insert(snapshot.key.clone(), snapshot);
            }
            SourceUpdate::Failed { key, message } => {
                errors.push(SourceError { key, message });
            }
        }
    }
    let sources = sources.into_values().collect::<Vec<_>>();
    let apps = sanitize(
        sources
            .iter()
            .flat_map(|snapshot| snapshot.apps.iter().cloned())
            .collect(),
    );
    MergedSources {
        sources,
        apps,
        errors,
    }
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
            install_location: None,
            can_uninstall: false,
            uninstall: None,
            resolved_path: None,
            shortcut_icon_path: None,
        }
    }

    fn snapshot(key: &str, apps: Vec<AppInfo>) -> SourceSnapshot {
        SourceSnapshot {
            key: SourceKey(key.into()),
            fingerprint: None,
            apps,
        }
    }

    #[test]
    fn replaces_only_the_successful_dirty_source() {
        let old = vec![
            snapshot("start-menu", vec![app("Old", "old.lnk")]),
            snapshot("registry:hklm", vec![app("Editor", "editor.exe")]),
        ];
        let updates = vec![SourceUpdate::Success(snapshot(
            "start-menu",
            vec![app("New", "new.lnk")],
        ))];

        let merged = merge_sources(old, updates);

        assert!(merged.apps.iter().any(|app| app.name == "New"));
        assert!(merged.apps.iter().any(|app| app.name == "Editor"));
        assert!(!merged.apps.iter().any(|app| app.name == "Old"));
    }

    #[test]
    fn failed_source_keeps_its_last_successful_snapshot() {
        let old = vec![snapshot("registry:hklm", vec![app("Editor", "editor.exe")])];

        let merged = merge_sources(
            old,
            vec![SourceUpdate::Failed {
                key: SourceKey("registry:hklm".into()),
                message: "access denied".into(),
            }],
        );

        assert_eq!(merged.sources.len(), 1);
        assert_eq!(merged.apps[0].name, "Editor");
        assert_eq!(merged.errors.len(), 1);
    }
}
