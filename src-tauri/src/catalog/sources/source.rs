use crate::catalog::sync::scan_control::StageStop;
use crate::catalog::AppInfo;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub(crate) const REGISTRY_SOURCE: &str = "registry";
pub(crate) const START_MENU_SOURCE: &str = "start-menu";
pub(crate) const START_APPS_SOURCE: &str = "start-apps";
pub(crate) const INSTALLER_CACHE_SOURCE: &str = "installer-cache";

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

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SourceHealthState {
    NeverRun,
    Fresh,
    Stale,
    Incomplete,
    FailedWithoutSnapshot,
    #[serde(other)]
    Unknown,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SourceErrorKind {
    Cancelled,
    TimedOut,
    EntryLimit,
    ProviderFailed,
    #[serde(other)]
    Unknown,
}

impl From<StageStop> for SourceErrorKind {
    fn from(stop: StageStop) -> Self {
        match stop {
            StageStop::Cancelled => Self::Cancelled,
            StageStop::TimedOut => Self::TimedOut,
            StageStop::EntryLimit => Self::EntryLimit,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SourceHealth {
    pub key: SourceKey,
    pub state: SourceHealthState,
    pub last_attempt_at: Option<u64>,
    pub last_success_at: Option<u64>,
    pub consecutive_failures: u32,
    pub last_duration_ms: Option<u64>,
    pub last_error: Option<SourceErrorKind>,
    pub record_count: usize,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SourceSnapshot {
    pub key: SourceKey,
    pub fingerprint: Option<SourceFingerprint>,
    pub apps: Vec<AppInfo>,
    #[serde(default)]
    pub health: Option<SourceHealth>,
}

pub(crate) struct MergedSources {
    pub sources: Vec<SourceSnapshot>,
    pub apps: Vec<AppInfo>,
}

pub(crate) fn apply_health(sources: &mut Vec<SourceSnapshot>, health: Vec<SourceHealth>) {
    for entry in health {
        match sources
            .iter_mut()
            .find(|snapshot| snapshot.key == entry.key)
        {
            Some(snapshot) => snapshot.health = Some(entry),
            None => sources.push(SourceSnapshot {
                key: entry.key.clone(),
                fingerprint: None,
                apps: Vec::new(),
                health: Some(entry),
            }),
        }
    }
    sources.sort_by(|left, right| left.key.cmp(&right.key));
}

pub(crate) fn previous_failures(previous: &[SourceSnapshot], key: &SourceKey) -> u32 {
    previous
        .iter()
        .find(|snapshot| &snapshot.key == key)
        .and_then(|snapshot| snapshot.health.as_ref())
        .map_or(0, |health| health.consecutive_failures)
}

pub(crate) fn previous_success(previous: &[SourceSnapshot], key: &SourceKey) -> Option<u64> {
    previous
        .iter()
        .find(|snapshot| &snapshot.key == key)
        .and_then(|snapshot| snapshot.health.as_ref())
        .and_then(|health| health.last_success_at)
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
            artifact_kind: Default::default(),
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
            target_availability: None,
            category_reasons: Vec::new(),
            close_risk: None,
        }
    }

    fn snapshot(key: &str, apps: Vec<AppInfo>) -> SourceSnapshot {
        SourceSnapshot {
            key: SourceKey(key.into()),
            fingerprint: None,
            health: None,
            apps,
        }
    }

    #[test]
    fn a_source_left_out_of_the_updates_keeps_its_previous_apps() {
        let previous = vec![
            snapshot(START_APPS_SOURCE, vec![app("Store App", "store.aumid")]),
            snapshot(REGISTRY_SOURCE, vec![app("Editor", "editor.exe")]),
        ];
        let updates = vec![snapshot(
            REGISTRY_SOURCE,
            vec![app("Editor", "editor.exe"), app("Viewer", "viewer.exe")],
        )];

        let merged = merge_sources(previous, updates);

        assert!(merged.apps.iter().any(|app| app.name == "Store App"));
        assert!(merged.apps.iter().any(|app| app.name == "Viewer"));
    }

    fn health(key: &str, state: SourceHealthState) -> SourceHealth {
        SourceHealth {
            key: SourceKey(key.into()),
            state,
            last_attempt_at: Some(10),
            last_success_at: None,
            consecutive_failures: 1,
            last_duration_ms: Some(1),
            last_error: Some(SourceErrorKind::ProviderFailed),
            record_count: 0,
        }
    }

    #[test]
    fn recording_health_does_not_touch_what_a_source_serves() {
        let mut sources = vec![snapshot(
            START_APPS_SOURCE,
            vec![app("Store App", "store.aumid")],
        )];

        apply_health(
            &mut sources,
            vec![health(START_APPS_SOURCE, SourceHealthState::Stale)],
        );

        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].apps.len(), 1);
        assert_eq!(
            sources[0].health.as_ref().map(|health| health.state),
            Some(SourceHealthState::Stale)
        );
    }

    #[test]
    fn a_source_without_a_snapshot_still_gets_a_health_entry() {
        let mut sources = vec![snapshot(REGISTRY_SOURCE, vec![app("Editor", "editor.exe")])];

        apply_health(
            &mut sources,
            vec![health(
                START_APPS_SOURCE,
                SourceHealthState::FailedWithoutSnapshot,
            )],
        );

        let entry = sources
            .iter()
            .find(|snapshot| snapshot.key.0 == START_APPS_SOURCE)
            .expect("a health-only entry exists");
        assert!(entry.apps.is_empty());
        assert_eq!(
            sources
                .iter()
                .map(|snapshot| snapshot.apps.len())
                .sum::<usize>(),
            1,
            "no application was invented"
        );
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
