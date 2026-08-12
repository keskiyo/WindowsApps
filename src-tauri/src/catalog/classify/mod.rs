mod score;
mod signals;
mod tables;

use super::{AppCategory, AppInfo, LaunchKind, SourceKind};
use signals::Signals;
use std::path::Path;

pub(crate) fn classify_app(app: &AppInfo) -> AppCategory {
    if app.source_kind == SourceKind::Steam {
        return AppCategory::Games;
    }
    if is_wsl_start_app(app) {
        return AppCategory::Development;
    }
    score::best(&Signals::from_app(app))
}

pub(crate) fn category_reasons(app: &AppInfo) -> Vec<String> {
    if app.source_kind == SourceKind::Steam {
        return vec!["source=steam".into()];
    }
    if is_wsl_start_app(app) {
        return vec!["rule=wsl-start-app".into()];
    }
    let reasons = score::evidence(&Signals::from_app(app), app.category);
    if reasons.is_empty() {
        return vec!["default=no-signal".into()];
    }
    reasons
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

pub(super) fn classify(name: &str, path: &str) -> AppCategory {
    score::best(&Signals::from_name_path(name, path))
}

#[cfg(test)]
mod tests {
    use super::super::{AppInfo, LaunchKind};
    use super::{category_reasons, classify_app, AppCategory, SourceKind};
    use serde::Deserialize;

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
            target_availability: None,
            category_reasons: Vec::new(),
            close_risk: None,
        }
    }

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
    fn localised_command_prompt_name_is_classified_without_path_hints() {
        assert_eq!(
            super::classify("\u{41a}\u{43e}\u{43c}\u{430}\u{43d}\u{434}\u{43d}\u{430} \u{441}\u{442}\u{440}\u{43e}\u{43a}\u{430}", r"D:\\Portable\\app.exe"),
            AppCategory::System
        );
    }

    #[test]
    fn every_classified_record_can_say_why() {
        let fixtures: Vec<Fixture> = serde_json::from_str(include_str!(
            "../../../tests/fixtures/catalog_categories.json"
        ))
        .expect("category fixtures parse");
        for fixture in &fixtures {
            let mut app = build(fixture);
            app.category = classify_app(&app);
            let reasons = category_reasons(&app);

            assert!(!reasons.is_empty(), "{}", fixture.name);
            for reason in &reasons {
                let (field, needle) = reason.split_once('=').unwrap_or_else(|| {
                    panic!("{} produced an unparseable reason {reason}", fixture.name)
                });
                assert!(!field.is_empty() && !needle.is_empty(), "{}", fixture.name);
            }
            if app.category == AppCategory::Other {
                assert_eq!(reasons, ["default=no-signal"], "{}", fixture.name);
            } else {
                assert_ne!(reasons, ["default=no-signal"], "{}", fixture.name);
            }
        }
    }

    #[test]
    fn the_reason_explains_the_category_the_record_carries() {
        let mut chrome = build(
            &serde_json::from_str(
                r#"{"name":"Google Chrome","path":"C:\\Program Files\\Google\\Chrome\\chrome.exe","expected":"browsers"}"#,
            )
            .unwrap(),
        );
        chrome.category = AppCategory::Browsers;

        let browsers = category_reasons(&chrome);
        chrome.category = AppCategory::Games;
        let games = category_reasons(&chrome);

        assert!(browsers.iter().any(|reason| reason.contains("chrome")));
        assert_eq!(games, ["default=no-signal"]);
    }

    #[test]
    fn an_override_reports_itself_rather_than_a_rule() {
        let mut steam = build(
            &serde_json::from_str(
                r#"{"name":"Some Title","path":"steam://rungameid/1","source":"steam","expected":"games"}"#,
            )
            .unwrap(),
        );
        steam.category = classify_app(&steam);

        assert_eq!(category_reasons(&steam), ["source=steam"]);
    }

    #[test]
    fn a_wsl_start_app_reports_the_rule_that_moved_it() {
        let mut wsl = build(
            &serde_json::from_str(
                r#"{"name":"Opaque distribution","path":"Vendor.Opaque!App","source":"start_apps","resolvedPath":"C:\\Program Files\\WSL\\wsl.exe","expected":"development"}"#,
            )
            .unwrap(),
        );
        wsl.launch_kind = LaunchKind::AppUserModelId;
        wsl.category = classify_app(&wsl);

        assert_eq!(category_reasons(&wsl), ["rule=wsl-start-app"]);
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
