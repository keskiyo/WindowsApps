//! Developer-only visibility diagnostics: dumps the entries the classifier rejected so borderline
//! rules can be inspected. Gated behind the dedup dev-report flag and never part of release flow.

use super::{VisibilityClass, VisibilityReason};
use crate::catalog::{AppInfo, SourceKind};
use serde::Serialize;
use std::path::Path;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RejectedEntry<'a> {
    source: SourceKind,
    name: &'a str,
    target: String,
    score: i16,
    reasons: &'a [VisibilityReason],
    product_name: Option<&'a str>,
    original_filename: Option<&'a str>,
}

pub(crate) fn write_dev_report(apps: &[AppInfo]) {
    if !crate::catalog::dedup::dev_report_enabled() {
        return;
    }
    let Ok(base) = std::env::var("LOCALAPPDATA") else {
        return;
    };
    let user_profile = std::env::var("USERPROFILE").ok();
    let rejected = apps
        .iter()
        .filter(|app| app.visibility_class == VisibilityClass::Rejected)
        .map(|app| RejectedEntry {
            source: app.source_kind,
            name: &app.name,
            target: redact_path(
                app.resolved_path.as_deref().unwrap_or(&app.path),
                user_profile.as_deref(),
            ),
            score: app.visibility_score,
            reasons: &app.visibility_reasons,
            product_name: app.product_name.as_deref(),
            original_filename: app.original_filename.as_deref(),
        })
        .collect::<Vec<_>>();
    let path = Path::new(&base)
        .join("WindowsApps")
        .join("visibility-report.json");
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(&rejected) {
        let _ = std::fs::write(path, json);
    }
}

fn redact_path(value: &str, user_profile: Option<&str>) -> String {
    let Some(profile) = user_profile else {
        return value.to_string();
    };
    if value
        .get(..profile.len())
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(profile))
    {
        format!("<USERPROFILE>{}", &value[profile.len()..])
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_report_redacts_the_user_profile_prefix() {
        let profile = r"C:\Users\Maks";
        assert_eq!(
            redact_path(r"C:\Users\Maks\Downloads\setup.exe", Some(profile)),
            r"<USERPROFILE>\Downloads\setup.exe"
        );
    }
}
