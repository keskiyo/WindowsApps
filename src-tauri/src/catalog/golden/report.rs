use crate::catalog::dedup::normalize_path;
use crate::catalog::AppInfo;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub(super) const REPORT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub(super) struct GoldenReport {
    pub schema_version: u32,
    pub input_record_count: usize,
    pub output_app_count: usize,
    pub records: Vec<GoldenRecord>,
    pub dedup_groups: Vec<GoldenGroup>,
    pub source_counts: BTreeMap<String, usize>,
    pub visibility_counts: BTreeMap<String, usize>,
    pub category_counts: BTreeMap<String, usize>,
    pub diagnostics: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub(super) struct GoldenRecord {
    pub canonical_id: String,
    pub preference_identity: String,
    pub canonical_identity: String,
    pub launch: String,
    pub source_kind: String,
    pub artifact_kind: String,
    pub visibility_class: String,
    pub category: String,
    pub display_name: String,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub(super) struct GoldenGroup {
    pub canonical_id: String,
    pub members: Vec<String>,
}

fn launch_descriptor(app: &AppInfo) -> String {
    let target = normalize_path(app.resolved_path.as_deref().unwrap_or(&app.path));
    let arguments = app
        .launch_arguments
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or_default()
        .to_lowercase();
    format!("{:?}|{target}|{arguments}", app.launch_kind)
}

fn count_by(values: impl Iterator<Item = String>) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for value in values {
        *counts.entry(value).or_insert(0) += 1;
    }
    counts
}

pub(super) fn build(
    input: &[AppInfo],
    output: &[AppInfo],
    groups: Vec<(String, Vec<String>)>,
    diagnostics: BTreeMap<String, String>,
) -> GoldenReport {
    let mut records = output
        .iter()
        .map(|app| GoldenRecord {
            canonical_id: app.id.clone(),
            preference_identity: app.preference_identity.clone().unwrap_or_default(),
            canonical_identity: app.canonical_identity.clone().unwrap_or_default(),
            launch: launch_descriptor(app),
            source_kind: format!("{:?}", app.source_kind),
            artifact_kind: format!("{:?}", app.artifact_kind),
            visibility_class: format!("{:?}", app.visibility_class),
            category: format!("{:?}", app.category),
            display_name: app.name.clone(),
        })
        .collect::<Vec<_>>();
    records.sort_by(|left, right| left.canonical_id.cmp(&right.canonical_id));
    GoldenReport {
        schema_version: REPORT_SCHEMA_VERSION,
        input_record_count: input.len(),
        output_app_count: output.len(),
        source_counts: count_by(records.iter().map(|record| record.source_kind.clone())),
        visibility_counts: count_by(records.iter().map(|record| record.visibility_class.clone())),
        category_counts: count_by(records.iter().map(|record| record.category.clone())),
        records,
        dedup_groups: groups
            .into_iter()
            .map(|(canonical_id, members)| GoldenGroup {
                canonical_id,
                members: members.iter().map(|path| normalize_path(path)).collect(),
            })
            .collect(),
        diagnostics,
    }
}
