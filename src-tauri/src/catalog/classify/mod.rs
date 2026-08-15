mod score;
mod signals;
mod tables;
mod vocabulary;

use super::machine::Associations;
use super::{AppCategory, AppInfo, LaunchKind, SourceKind};
use signals::Signals;
use std::path::Path;

pub(crate) fn classify_app(app: &AppInfo, associations: &Associations) -> AppCategory {
    if app.source_kind == SourceKind::Steam {
        return AppCategory::Games;
    }
    if is_wsl_start_app(app) {
        return AppCategory::Development;
    }
    score::best(&Signals::from_app(app, associations))
}

pub(crate) fn category_reasons(app: &AppInfo, associations: &Associations) -> Vec<String> {
    if app.source_kind == SourceKind::Steam {
        return vec!["source=steam".into()];
    }
    if is_wsl_start_app(app) {
        return vec!["rule=wsl-start-app".into()];
    }
    let reasons = score::evidence(&Signals::from_app(app, associations), app.category);
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
pub(super) fn category_scores(app: &AppInfo) -> Vec<(AppCategory, i32)> {
    score::ranked(&Signals::from_app(app, &Associations::empty()))
}

#[cfg(test)]
mod tests {
    use super::super::{AppInfo, LaunchKind};
    use super::{AppCategory, Associations, SourceKind};
    use serde::Deserialize;

    fn classify_app(app: &AppInfo) -> AppCategory {
        super::classify_app(app, &Associations::empty())
    }

    fn category_reasons(app: &AppInfo) -> Vec<String> {
        super::category_reasons(app, &Associations::empty())
    }

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

    fn scores_for(json: &str) -> Vec<(AppCategory, i32)> {
        super::category_scores(&build(&serde_json::from_str(json).unwrap()))
    }

    fn margin(scores: &[(AppCategory, i32)]) -> i32 {
        match scores {
            [(_, winner), (_, runner_up), ..] => winner - runner_up,
            [(_, winner)] => *winner,
            [] => 0,
        }
    }

    #[test]
    fn the_ranking_and_the_chosen_category_cannot_disagree() {
        let fixtures: Vec<Fixture> = serde_json::from_str(include_str!(
            "../../../tests/fixtures/catalog_categories.json"
        ))
        .expect("category fixtures parse");
        for fixture in fixtures.iter().filter(|f| f.source != SourceKind::Steam) {
            let app = build(fixture);
            let expected = super::category_scores(&app)
                .first()
                .filter(|(_, score)| *score >= super::score::THRESHOLD)
                .map_or(AppCategory::Other, |(category, _)| *category);

            assert_eq!(classify_app(&app), expected, "{}", fixture.name);
        }
    }

    #[test]
    fn a_localised_name_wins_its_category_outright() {
        let scores = scores_for(
            r#"{"name":"Яндекс Музыка","path":"C:\\Users\\U\\AppData\\Local\\Programs\\YandexMusic\\Яндекс Музыка.exe","expected":"media"}"#,
        );

        assert_eq!(
            scores.first().map(|(category, _)| *category),
            Some(AppCategory::Media)
        );
        assert!(margin(&scores) > 0, "{scores:?}");
    }

    #[test]
    fn a_windows_feature_name_never_matches_a_vendor_install_path() {
        for json in [
            r#"{"name":"NVIDIA Control Panel","path":"C:\\Program Files\\NVIDIA Corporation\\Control Panel Client\\nvcplui.exe","publisher":"NVIDIA Corporation","expected":"utilities"}"#,
            r#"{"name":"Logitech Options Calculator Widget","path":"C:\\Program Files\\Logitech\\Options\\widgets\\calculator\\widget.exe","publisher":"Logitech","expected":"utilities"}"#,
            r#"{"name":"Notepad++","path":"C:\\Program Files\\Notepad++\\notepad++.exe","expected":"development"}"#,
        ] {
            let scores = scores_for(json);

            assert!(
                !scores
                    .iter()
                    .any(|(category, _)| *category == AppCategory::WindowsFeatures),
                "{json} still scores as a Windows feature: {scores:?}"
            );
        }
    }

    #[test]
    fn localised_command_prompt_name_is_classified_without_path_hints() {
        assert_eq!(
            super::classify("\u{41a}\u{43e}\u{43c}\u{430}\u{43d}\u{434}\u{43d}\u{430}\u{44f} \u{441}\u{442}\u{440}\u{43e}\u{43a}\u{430}", r"D:\Portable\app.exe"),
            AppCategory::WindowsFeatures
        );
    }

    #[test]
    fn a_bitness_qualifier_does_not_hide_a_windows_feature() {
        assert_eq!(
            super::classify("Windows PowerShell (x86)", r"D:\Menu\shell.lnk"),
            AppCategory::WindowsFeatures
        );
        assert_eq!(
            super::classify("Windows PowerShell", r"D:\Menu\shell.lnk"),
            AppCategory::WindowsFeatures
        );
    }

    #[test]
    fn a_name_that_is_only_a_qualifier_matches_nothing() {
        assert_eq!(
            super::classify("(x86)", r"D:\Portable\app.exe"),
            AppCategory::Other
        );
    }

    #[test]
    fn a_registered_file_type_classifies_a_product_no_table_names() {
        let app = build(
            &serde_json::from_str(
                r#"{"name":"Zvukoved","path":"C:\\Program Files\\Zvukoved\\zvukoved.exe","expected":"media"}"#,
            )
            .unwrap(),
        );
        let associations =
            Associations::from_pairs(vec![("zvukoved".into(), vec![".flac".into()])]);

        assert_eq!(super::classify_app(&app, &associations), AppCategory::Media);
        assert_eq!(classify_app(&app), AppCategory::Other);
    }

    #[test]
    fn a_registered_protocol_names_the_purpose_of_an_unknown_client() {
        let app = build(
            &serde_json::from_str(
                r#"{"name":"Pismo","path":"C:\\Program Files\\Pismo\\pismo.exe","expected":"communication"}"#,
            )
            .unwrap(),
        );
        let associations =
            Associations::from_pairs(vec![("pismo".into(), vec!["url:mailto".into()])]);

        assert_eq!(
            super::classify_app(&app, &associations),
            AppCategory::Communication
        );
    }

    #[test]
    fn a_micro_sign_reads_as_the_latin_letter_it_imitates() {
        assert_eq!(
            super::classify("\u{b5}Torrent", r"D:\Portable\app.exe"),
            AppCategory::FileCloud
        );
        assert_eq!(
            super::classify("\u{3bc}Torrent", r"D:\Portable\app.exe"),
            AppCategory::FileCloud
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
