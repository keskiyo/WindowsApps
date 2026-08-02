//! App categorization. `classify_app` weighs every reliable signal on a fully merged record;
//! `classify` is the reduced scan-time path with only a name and path. Both run the same weighted
//! scorer (`score`) over tokenized signals (`signals`) and declarative rules (`tables`), so a keyword
//! never fires on a substring of an unrelated word and corroborating signals accumulate.

mod score;
mod signals;
mod tables;

use super::{AppCategory, AppInfo, LaunchKind, SourceKind};
use signals::Signals;
use std::path::Path;

/// Final category for a fully resolved catalog entry (runs after deduplication). A Steam entry is
/// Games by construction; everything else is decided by the weighted scorer, which reads the
/// publisher, install tree, resolved executable, product name, file description, name, and path.
pub(crate) fn classify_app(app: &AppInfo) -> AppCategory {
    if app.source_kind == SourceKind::Steam {
        return AppCategory::Games;
    }
    if is_wsl_start_app(app) {
        return AppCategory::Development;
    }
    score::best(&Signals::from_app(app))
}

fn is_wsl_start_app(app: &AppInfo) -> bool {
    app.source_kind == SourceKind::StartApps
        && app.launch_kind == LaunchKind::AppUserModelId
        && app.resolved_path.as_deref().is_some_and(|target| {
            Path::new(target.trim().trim_matches('"'))
                .file_name()
                .and_then(|file| file.to_str())
                .is_some_and(|file| file.eq_ignore_ascii_case("wsl.exe"))
        })
}

/// Provisional category during scanning, when only a display name and a path are known. The merged
/// record is reclassified by `classify_app` after deduplication.
pub(super) fn classify(name: &str, path: &str) -> AppCategory {
    score::best(&Signals::from_name_path(name, path))
}

#[cfg(test)]
mod tests {
    use super::super::{AppInfo, LaunchKind};
    use super::{classify_app, AppCategory, SourceKind};
    use serde::Deserialize;

    /// One labelled catalog record. Mirrors the visibility corpus fixture shape; `expected`
    /// deserializes straight into `AppCategory` via its snake_case serde naming.
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Fixture {
        name: String,
        path: String,
        expected: AppCategory,
        #[serde(default)]
        source: SourceKind,
        #[serde(default)]
        description: Option<String>,
        #[serde(default)]
        publisher: Option<String>,
        #[serde(default)]
        product_name: Option<String>,
        #[serde(default)]
        original_filename: Option<String>,
        #[serde(default)]
        resolved_path: Option<String>,
        #[serde(default)]
        install_location: Option<String>,
    }

    fn build(fixture: &Fixture) -> AppInfo {
        AppInfo {
            id: fixture.name.clone(),
            name: fixture.name.clone(),
            path: fixture.path.clone(),
            icon_base64: None,
            artifact_kind: Default::default(),
            category: AppCategory::default(),
            launch_kind: if fixture.path.ends_with(".lnk") {
                LaunchKind::Shortcut
            } else {
                LaunchKind::Executable
            },
            source_kind: fixture.source,
            description: fixture.description.clone(),
            version: None,
            publisher: fixture.publisher.clone(),
            product_name: fixture.product_name.clone(),
            original_filename: fixture.original_filename.clone(),
            install_location: fixture.install_location.clone(),
            can_uninstall: false,
            uninstall: None,
            resolved_path: fixture.resolved_path.clone(),
            shortcut_icon_path: None,
            launch_arguments: None,
            canonical_identity: None,
            preference_identity: None,
            visibility_class: Default::default(),
            visibility_score: 0,
            visibility_reasons: Vec::new(),
        }
    }

    /// Regression corpus: a labelled set of "signals -> category" records guards accuracy so a later
    /// weight or table change cannot silently regress it. Extend the JSON when adding coverage.
    #[test]
    fn category_fixture_corpus_matches_manual_labels() {
        let fixtures: Vec<Fixture> = serde_json::from_str(include_str!(
            "../../../tests/fixtures/catalog_categories.json"
        ))
        .expect("category fixtures parse");
        assert!(
            fixtures.len() >= 60,
            "corpus should stay a meaningful size ({} entries)",
            fixtures.len()
        );
        for fixture in &fixtures {
            assert_eq!(
                classify_app(&build(fixture)),
                fixture.expected,
                "{}",
                fixture.name
            );
        }
    }

    #[test]
    fn wsl_backed_start_app_is_development() {
        let fixture: Fixture = serde_json::from_str(
            r#"{"name":"Opaque distribution","path":"Vendor.Opaque!App","source":"start_apps","resolvedPath":"C:\\Program Files\\WSL\\wsl.exe","expected":"development"}"#,
        )
        .unwrap();
        let mut app = build(&fixture);
        app.launch_kind = LaunchKind::AppUserModelId;

        assert_eq!(classify_app(&app), AppCategory::Development);
    }
}
