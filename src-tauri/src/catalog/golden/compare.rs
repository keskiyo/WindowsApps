use super::report::{GoldenRecord, GoldenReport};
use std::collections::BTreeMap;

#[derive(Debug, Eq, PartialEq)]
pub(super) enum Severity {
    Forbidden,
    Allowed,
    DiagnosticOnly,
}

#[derive(Debug)]
pub(super) struct Change {
    pub severity: Severity,
    pub detail: String,
}

fn by_id(report: &GoldenReport) -> BTreeMap<&str, &GoldenRecord> {
    report
        .records
        .iter()
        .map(|record| (record.canonical_id.as_str(), record))
        .collect()
}

fn field_changes(baseline: &GoldenRecord, current: &GoldenRecord) -> Vec<String> {
    [
        (
            "preference_identity",
            &baseline.preference_identity,
            &current.preference_identity,
        ),
        (
            "canonical_identity",
            &baseline.canonical_identity,
            &current.canonical_identity,
        ),
        ("launch", &baseline.launch, &current.launch),
        ("source_kind", &baseline.source_kind, &current.source_kind),
        (
            "artifact_kind",
            &baseline.artifact_kind,
            &current.artifact_kind,
        ),
        (
            "visibility_class",
            &baseline.visibility_class,
            &current.visibility_class,
        ),
        ("category", &baseline.category, &current.category),
        (
            "display_name",
            &baseline.display_name,
            &current.display_name,
        ),
    ]
    .into_iter()
    .filter(|(_, was, now)| was != now)
    .map(|(field, was, now)| format!("{} {field}: {was} -> {now}", baseline.canonical_id))
    .collect()
}

pub(super) fn compare(
    baseline: &GoldenReport,
    current: &GoldenReport,
    allowed: &[&str],
) -> Vec<Change> {
    let mut changes = Vec::new();
    let severity_for = |id: &str| {
        if allowed.contains(&id) {
            Severity::Allowed
        } else {
            Severity::Forbidden
        }
    };

    let was = by_id(baseline);
    let now = by_id(current);
    for (id, record) in &was {
        match now.get(id) {
            None => changes.push(Change {
                severity: severity_for(id),
                detail: format!("record removed: {id}"),
            }),
            Some(current) => {
                for detail in field_changes(record, current) {
                    changes.push(Change {
                        severity: severity_for(id),
                        detail,
                    });
                }
            }
        }
    }
    for id in now.keys().filter(|id| !was.contains_key(*id)) {
        changes.push(Change {
            severity: severity_for(id),
            detail: format!("record added: {id}"),
        });
    }
    if baseline.dedup_groups != current.dedup_groups {
        changes.push(Change {
            severity: Severity::Forbidden,
            detail: "dedup grouping changed".into(),
        });
    }
    if baseline.input_record_count != current.input_record_count {
        changes.push(Change {
            severity: Severity::Forbidden,
            detail: format!(
                "input record count: {} -> {}",
                baseline.input_record_count, current.input_record_count
            ),
        });
    }
    if baseline.diagnostics != current.diagnostics {
        changes.push(Change {
            severity: Severity::DiagnosticOnly,
            detail: "diagnostics changed".into(),
        });
    }
    changes
}

pub(super) fn forbidden(changes: &[Change]) -> Vec<&str> {
    changes
        .iter()
        .filter(|change| change.severity == Severity::Forbidden)
        .map(|change| change.detail.as_str())
        .collect()
}
