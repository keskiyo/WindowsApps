use crate::catalog::{AppInfo, CatalogAppDto};
use serde::Serialize;
use std::collections::HashMap;

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CatalogChangeSummary {
    pub added: usize,
    pub removed: usize,
    pub updated: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct CatalogDelta {
    pub generation: u64,
    pub upserted: Vec<AppInfo>,
    pub removed_ids: Vec<String>,
    pub summary: CatalogChangeSummary,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CatalogDeltaDto {
    generation: u64,
    upserted: Vec<CatalogAppDto>,
    removed_ids: Vec<String>,
    summary: CatalogChangeSummary,
}

impl From<&CatalogDelta> for CatalogDeltaDto {
    fn from(delta: &CatalogDelta) -> Self {
        Self {
            generation: delta.generation,
            upserted: delta.upserted.iter().map(CatalogAppDto::from).collect(),
            removed_ids: delta.removed_ids.clone(),
            summary: delta.summary.clone(),
        }
    }
}

pub(crate) fn compute_delta(
    generation: u64,
    previous: &[AppInfo],
    current: &[AppInfo],
) -> CatalogDelta {
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
        .filter(|app| old.get(app.id.as_str()).is_none_or(|old| *old != *app))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::sync::app;
    use crate::catalog::UninstallTarget;

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
    fn catalog_delta_excludes_execution_metadata_from_webview_json() {
        let mut current = app("editor", "Editor");
        current.uninstall = Some(UninstallTarget::Command {
            executable: "TOP_SECRET_DELTA_UNINSTALL_EXECUTABLE".into(),
            arguments: "TOP_SECRET_DELTA_UNINSTALL_ARGUMENTS".into(),
        });
        current.launch_arguments = Some("TOP_SECRET_DELTA_LAUNCH_ARGUMENTS".into());
        current.resolved_path = Some("TOP_SECRET_DELTA_RESOLVED_TARGET".into());
        current.shortcut_icon_path = Some("TOP_SECRET_DELTA_SHORTCUT_ICON".into());

        let delta = compute_delta(1, &[], &[current]);
        let json = serde_json::to_value(CatalogDeltaDto::from(&delta)).unwrap();
        let serialized = json.to_string();

        for secret in [
            "TOP_SECRET_DELTA_UNINSTALL_EXECUTABLE",
            "TOP_SECRET_DELTA_UNINSTALL_ARGUMENTS",
            "TOP_SECRET_DELTA_LAUNCH_ARGUMENTS",
            "TOP_SECRET_DELTA_RESOLVED_TARGET",
            "TOP_SECRET_DELTA_SHORTCUT_ICON",
        ] {
            assert!(!serialized.contains(secret));
        }
        let app = &json["upserted"][0];
        assert!(app.get("uninstall").is_none());
        assert!(app.get("launchArguments").is_none());
        assert!(app.get("resolvedPath").is_none());
        assert!(app.get("shortcutIconPath").is_none());
    }
}
