mod compare;
mod corpus;
mod properties;
mod report;
mod timings;

use crate::catalog::machine::Registrations;
use crate::catalog::AppInfo;
use crate::platform::windows::NameScript;
use report::GoldenReport;
use std::collections::BTreeMap;
use std::path::PathBuf;

const UPDATE_ENV: &str = "WINDOWSAPPS_GOLDEN_UPDATE";

fn pinned_diagnostics() -> BTreeMap<String, String> {
    BTreeMap::from([
        ("os_script".to_string(), "latin".to_string()),
        ("registrations".to_string(), "empty".to_string()),
        ("dev_reports".to_string(), "disabled".to_string()),
    ])
}

fn pinned_sanitize(records: Vec<AppInfo>) -> Vec<AppInfo> {
    crate::catalog::sanitize_pinned(records, &Registrations::empty(), NameScript::Latin)
}

fn visible_entries(records: Vec<AppInfo>) -> Vec<AppInfo> {
    crate::catalog::retain_visible(crate::catalog::classify_entries(
        records,
        &Registrations::empty(),
    ))
}

fn pinned_report(records: Vec<AppInfo>) -> GoldenReport {
    let groups = crate::catalog::dedup::resolved_groups(visible_entries(records.clone()));
    let output = pinned_sanitize(records.clone());
    report::build(&records, &output, groups, pinned_diagnostics())
}

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("golden")
        .join(name)
}

fn read_fixture(case: &str) -> Vec<AppInfo> {
    let path = fixture_path(&format!("{case}.json"));
    let bytes = std::fs::read(&path)
        .unwrap_or_else(|error| panic!("golden fixture {} is unreadable: {error}", path.display()));
    serde_json::from_slice(&bytes)
        .unwrap_or_else(|error| panic!("golden fixture {} does not parse: {error}", path.display()))
}

fn assert_matches_baseline(case: &str, allowed: &[&str]) {
    let current = pinned_report(read_fixture(case));
    let baseline_path = fixture_path(&format!("{case}.expected.json"));

    if std::env::var_os(UPDATE_ENV).is_some() {
        let json = serde_json::to_string_pretty(&current).expect("the golden report serializes");
        std::fs::write(&baseline_path, format!("{json}\n"))
            .unwrap_or_else(|error| panic!("cannot write {}: {error}", baseline_path.display()));
        return;
    }

    let bytes = std::fs::read(&baseline_path).unwrap_or_else(|error| {
        panic!(
            "no baseline at {} ({error}). Record one with {UPDATE_ENV}=1 and review the diff before committing it.",
            baseline_path.display()
        )
    });
    let baseline: GoldenReport = serde_json::from_slice(&bytes).unwrap_or_else(|error| {
        panic!(
            "baseline {} does not parse: {error}",
            baseline_path.display()
        )
    });

    let changes = compare::compare(&baseline, &current, allowed);
    let forbidden = compare::forbidden(&changes);
    assert!(
        forbidden.is_empty(),
        "{case}: the catalog changed in ways this stage did not declare:\n  {}",
        forbidden.join("\n  ")
    );
}

#[test]
fn squirrel_and_office_match_the_baseline() {
    assert_matches_baseline("squirrel-and-office", &[]);
}

#[test]
fn visibility_mix_matches_the_baseline() {
    assert_matches_baseline("visibility-mix", &[]);
}

#[test]
fn portable_and_store_match_the_baseline() {
    assert_matches_baseline("portable-and-store", &[]);
}

#[test]
fn a_changed_category_is_reported_as_forbidden() {
    let records = read_fixture("squirrel-and-office");
    let baseline = pinned_report(records.clone());
    let mut altered = pinned_report(records);
    altered.records[0].category = "Games".into();

    let changes = compare::compare(&baseline, &altered, &[]);
    let forbidden = compare::forbidden(&changes);

    assert_eq!(forbidden.len(), 1, "{forbidden:?}");
    assert!(forbidden[0].contains("category"));
}

#[test]
fn a_declared_case_is_allowed_rather_than_forbidden() {
    let records = read_fixture("squirrel-and-office");
    let baseline = pinned_report(records.clone());
    let mut altered = pinned_report(records);
    altered.records[0].category = "Games".into();
    let declared = altered.records[0].canonical_id.clone();

    let changes = compare::compare(&baseline, &altered, &[declared.as_str()]);

    assert!(compare::forbidden(&changes).is_empty());
    assert_eq!(changes.len(), 1);
}
