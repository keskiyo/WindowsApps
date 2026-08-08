use super::family::{launcher_product_family, normalized_product_family};
use super::identity::resolved_canonical_id;
use super::merge::ResolverReport;
use super::resolve_apps;
use crate::catalog::AppInfo;
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;

#[derive(Serialize)]
struct DedupMember {
    name: String,
    path: String,
    source: String,
}

#[derive(Serialize)]
struct DedupGroup {
    canonical_id: String,
    name: String,
    members: Vec<DedupMember>,
    evidence: Vec<String>,
}

#[derive(Serialize)]
struct PossibleConfusion {
    family: String,
    entries: Vec<String>,
}

#[derive(Serialize)]
struct DedupReport {
    input: usize,
    output: usize,
    merged_groups: Vec<DedupGroup>,
    possible_confusions: Vec<PossibleConfusion>,
}

pub(crate) fn dev_report_enabled() -> bool {
    cfg!(debug_assertions)
}

pub(crate) fn write_dev_report(apps: &[AppInfo]) {
    if apps.len() < 30 {
        return;
    }
    let report = analyze(apps.to_vec());
    let Ok(base) = std::env::var("LOCALAPPDATA") else {
        return;
    };
    let path = Path::new(&base)
        .join("WindowsApps")
        .join("dedup-report.json");
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(&report) {
        let _ = std::fs::write(path, json);
    }
}

#[cfg(test)]
pub(in crate::catalog) fn resolved_groups(mut apps: Vec<AppInfo>) -> Vec<(String, Vec<String>)> {
    apps.sort_by_cached_key(super::canonical_order_key);
    let mut report = ResolverReport::default();
    let mut groups = resolve_apps(apps, &mut report)
        .iter()
        .map(|group| {
            let mut members = group
                .candidates
                .iter()
                .map(|candidate| candidate.app.path.clone())
                .collect::<Vec<_>>();
            members.sort();
            (resolved_canonical_id(group), members)
        })
        .collect::<Vec<_>>();
    groups.sort();
    groups
}

fn analyze(apps: Vec<AppInfo>) -> DedupReport {
    let input = apps.len();
    let mut report = ResolverReport::default();
    let resolved = resolve_apps(apps, &mut report);
    let output = resolved.len();
    let merged_groups = resolved
        .iter()
        .filter(|group| group.candidates.len() > 1)
        .map(|group| DedupGroup {
            canonical_id: resolved_canonical_id(group),
            name: group.app.name.clone(),
            members: group
                .candidates
                .iter()
                .map(|candidate| DedupMember {
                    name: candidate.app.name.clone(),
                    path: candidate.app.path.clone(),
                    source: format!("{:?}", candidate.app.source_kind),
                })
                .collect(),
            evidence: group
                .evidence
                .iter()
                .map(|item| format!("{:?} ({})", item.reason, item.score))
                .collect(),
        })
        .collect();
    let mut by_family: HashMap<String, Vec<String>> = HashMap::new();
    for group in &resolved {
        let family =
            launcher_product_family(&normalized_product_family(&group.app.name)).to_string();
        by_family
            .entry(family)
            .or_default()
            .push(format!("{} [{}]", group.app.name, group.app.path));
    }
    let mut possible_confusions = by_family
        .into_iter()
        .filter(|(_, entries)| entries.len() > 1)
        .map(|(family, entries)| PossibleConfusion { family, entries })
        .collect::<Vec<_>>();
    possible_confusions.sort_by(|left, right| left.family.cmp(&right.family));
    DedupReport {
        input,
        output,
        merged_groups,
        possible_confusions,
    }
}
