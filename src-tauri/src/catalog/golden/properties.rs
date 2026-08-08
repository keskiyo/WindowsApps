use super::corpus::{generated_records, shuffled};
use super::pinned_report;
use crate::catalog::AppInfo;

const SEEDS: [u64; 5] = [1, 7, 42, 1337, 90210];
const CI_RECORDS: usize = 100;
const STRESS_RECORDS: usize = 500;

fn signature(records: Vec<AppInfo>) -> String {
    serde_json::to_string(&pinned_report(records)).expect("the golden report serializes")
}

fn cards(apps: &[AppInfo]) -> Vec<String> {
    let mut cards = apps
        .iter()
        .map(|app| {
            format!(
                "{} | {:?} | {:?} | {:?} | {}",
                app.id, app.visibility_class, app.category, app.artifact_kind, app.name
            )
        })
        .collect::<Vec<_>>();
    cards.sort();
    cards
}

fn difference(before: &[String], after: &[String]) -> Vec<String> {
    let mut lines = before
        .iter()
        .filter(|card| !after.contains(card))
        .map(|card| format!("-{card}"))
        .collect::<Vec<_>>();
    lines.extend(
        after
            .iter()
            .filter(|card| !before.contains(card))
            .map(|card| format!("+{card}")),
    );
    lines
}

#[test]
fn the_same_input_produces_a_byte_identical_report() {
    for seed in SEEDS {
        let records = generated_records(seed, CI_RECORDS);
        assert_eq!(
            signature(records.clone()),
            signature(records),
            "seed {seed}: sanitize is not deterministic"
        );
    }
}

#[test]
fn input_order_changes_nothing() {
    for seed in SEEDS {
        let records = generated_records(seed, CI_RECORDS);
        let baseline = signature(records.clone());
        for attempt in 0..4 {
            let reordered = shuffled(&records, seed ^ (attempt + 1));
            assert_eq!(
                signature(reordered),
                baseline,
                "seed {seed}, shuffle {attempt}: input order changed the catalog"
            );
        }
    }
}

#[test]
fn sanitizing_an_already_sanitized_catalog_changes_nothing() {
    for seed in SEEDS {
        let once = super::pinned_sanitize(generated_records(seed, CI_RECORDS));
        let twice = super::pinned_sanitize(once.clone());
        let moved = difference(&cards(&once), &cards(&twice));
        assert!(
            moved.is_empty(),
            "seed {seed}: sanitize is not idempotent\n  {}",
            moved.join("\n  ")
        );
    }
}

#[test]
fn a_cache_round_trip_preserves_every_record() {
    for seed in SEEDS {
        let sanitized = super::pinned_sanitize(generated_records(seed, CI_RECORDS));
        let json = serde_json::to_vec(&sanitized).expect("catalog serializes");
        let restored = serde_json::from_slice::<Vec<AppInfo>>(&json).expect("catalog deserializes");
        let moved = difference(
            &cards(&sanitized),
            &cards(&super::pinned_sanitize(restored)),
        );
        assert!(
            moved.is_empty(),
            "seed {seed}: a cache round trip changed the catalog\n  {}",
            moved.join("\n  ")
        );
    }
}

#[test]
fn diagnostic_fields_do_not_reach_an_identity() {
    for seed in SEEDS {
        let records = generated_records(seed, CI_RECORDS);
        let baseline = identities(&super::pinned_sanitize(records.clone()));

        let annotated = records
            .into_iter()
            .map(|mut record| {
                record.icon_base64 = Some("data:image/png;base64,AAAA".into());
                record.visibility_score = record.visibility_score.saturating_add(1);
                record.target_availability = Some("target.unverifiable.access_denied".into());
                record
            })
            .collect::<Vec<_>>();

        assert_eq!(
            identities(&super::pinned_sanitize(annotated)),
            baseline,
            "seed {seed}: a diagnostic field moved an identity"
        );
    }
}

fn identities(apps: &[AppInfo]) -> Vec<(String, String, String)> {
    let mut identities = apps
        .iter()
        .map(|app| {
            (
                app.id.clone(),
                app.preference_identity.clone().unwrap_or_default(),
                app.canonical_identity.clone().unwrap_or_default(),
            )
        })
        .collect::<Vec<_>>();
    identities.sort();
    identities
}

#[test]
fn every_field_except_the_visibility_score_survives_a_second_sanitize() {
    fn without_score(apps: &[AppInfo]) -> Vec<AppInfo> {
        let mut apps = apps
            .iter()
            .cloned()
            .map(|mut app| {
                app.visibility_score = 0;
                app
            })
            .collect::<Vec<_>>();
        apps.sort_by(|left, right| left.id.cmp(&right.id));
        apps
    }

    for seed in SEEDS {
        let once = super::pinned_sanitize(generated_records(seed, CI_RECORDS));
        let twice = super::pinned_sanitize(once.clone());
        let (before, after) = (without_score(&once), without_score(&twice));
        let moved = before
            .iter()
            .zip(&after)
            .filter(|(before, after)| before != after)
            .map(|(before, _)| before.id.clone())
            .collect::<Vec<_>>();
        assert!(
            moved.is_empty() && before.len() == after.len(),
            "seed {seed}: a second sanitize moved more than the score: {moved:?}"
        );
    }
}

#[test]
fn the_properties_hold_on_a_large_catalog() {
    let records = generated_records(4096, STRESS_RECORDS);
    let baseline = signature(records.clone());

    assert_eq!(signature(shuffled(&records, 5)), baseline);
    let once = super::pinned_sanitize(records);
    let moved = difference(&cards(&once), &cards(&super::pinned_sanitize(once.clone())));
    assert!(moved.is_empty(), "{}", moved.join("\n  "));
}
