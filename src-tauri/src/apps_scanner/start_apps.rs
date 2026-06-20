use super::{classify, is_invalid_display_name, is_maintenance_entry, stable_id, AppInfo, LaunchKind, SourceKind, UninstallTarget};
use serde::Deserialize;
use serde_json::Value;
use std::os::windows::process::CommandExt;
use std::process::Command;

const CREATE_NO_WINDOW: u32 = 0x08000000;
const UTF8_PREFIX: &str = "$OutputEncoding = [Console]::OutputEncoding = [Text.UTF8Encoding]::new($false); ";
const START_APPS_SCRIPT: &str = "Get-StartApps | Select-Object Name,AppID | ConvertTo-Json -Compress";
const PACKAGES_SCRIPT: &str = "Get-AppxPackage | Select-Object PackageFullName,PackageFamilyName,Name,Publisher,Version,InstallLocation | ConvertTo-Json -Compress";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct StartAppRow { name: String, #[serde(rename = "AppID")] app_id: String }

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct PackageRow {
    package_full_name: String,
    package_family_name: String,
    name: String,
    publisher: Option<String>,
    version: Option<String>,
    install_location: Option<String>,
}

pub(super) fn scan() -> Vec<AppInfo> {
    let apps_json = run_powershell(START_APPS_SCRIPT).unwrap_or_default();
    let packages_json = run_powershell(PACKAGES_SCRIPT).unwrap_or_default();
    let mut apps = parse_start_apps(&apps_json).unwrap_or_default();
    let packages: Vec<PackageRow> = parse_rows(&packages_json).unwrap_or_default();
    for app in &mut apps {
        if let Some(package) = packages.iter().find(|package| app.path.to_lowercase().starts_with(&package.package_family_name.to_lowercase())) {
            app.source_kind = SourceKind::Msix;
            app.publisher = display_publisher(&package.name, package.publisher.clone());
            app.version = package.version.clone().filter(|value| !value.trim().is_empty());
            app.install_location = package.install_location.clone().filter(|value| !value.trim().is_empty());
            app.description = Some(package.name.clone());
            app.can_uninstall = true;
            app.uninstall = Some(UninstallTarget::Msix { package_full_name: package.package_full_name.clone() });
        }
    }
    apps
}

fn display_publisher(package_name: &str, publisher: Option<String>) -> Option<String> {
    let publisher = publisher.map(|value| value.trim().to_string()).filter(|value| !value.is_empty());
    if publisher.as_deref().is_some_and(|value| value.starts_with("CN=")) {
        return package_name.split('.').next().map(str::trim).filter(|value| !value.is_empty()).map(str::to_string);
    }
    publisher
}

fn run_powershell(script: &str) -> Option<String> {
    let script = format!("{UTF8_PREFIX}{script}");
    let output = Command::new("powershell.exe")
        .args(["-NoLogo", "-NoProfile", "-NonInteractive", "-Command", &script])
        .creation_flags(CREATE_NO_WINDOW)
        .output().ok()?;
    output.status.success().then(|| decode_output(output.stdout)).flatten()
}

fn decode_output(bytes: Vec<u8>) -> Option<String> {
    String::from_utf8(bytes).ok().map(|value| value.trim().to_string())
}

pub(super) fn parse_start_apps(json: &str) -> Result<Vec<AppInfo>, serde_json::Error> {
    let rows: Vec<StartAppRow> = parse_rows(json)?;
    Ok(rows.into_iter().filter_map(|row| {
        let name = row.name.trim().to_string();
        let app_id = row.app_id.trim().to_string();
        if is_invalid_display_name(&name) || app_id.is_empty() || is_maintenance_entry(&name, &app_id, None) { return None }
        Some(AppInfo {
            id: stable_id(&app_id), category: classify(&name, &app_id), name,
            path: app_id, icon_base64: None, launch_kind: LaunchKind::AppUserModelId,
            source_kind: SourceKind::StartApps, description: None, version: None,
            publisher: None, install_location: None, can_uninstall: false, uninstall: None,
            resolved_path: None, shortcut_icon_path: None,
        })
    }).collect())
}

fn parse_rows<T: for<'de> Deserialize<'de>>(json: &str) -> Result<Vec<T>, serde_json::Error> {
    if json.trim().is_empty() { return Ok(Vec::new()) }
    let value: Value = serde_json::from_str(json)?;
    match value {
        Value::Array(_) => serde_json::from_value(value),
        Value::Null => Ok(Vec::new()),
        other => Ok(vec![serde_json::from_value(other)?]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_invalid_utf8_output() {
        assert_eq!(decode_output(vec![0xff, 0xfe]), None);
    }

    #[test]
    fn ignores_resource_and_replacement_character_names() {
        let apps = parse_start_apps(
            r#"[{"Name":"ms-resource:AppName","AppID":"Package!App"},{"Name":"Broken�Name","AppID":"Broken!App"}]"#,
        ).unwrap();
        assert!(apps.is_empty());
    }

    #[test]
    fn parses_start_apps_json() {
        let apps = parse_start_apps(r#"[{"Name":"Codex","AppID":"OpenAI.Codex_abc!App"}]"#).unwrap();
        assert_eq!(apps[0].name, "Codex");
        assert_eq!(apps[0].path, "OpenAI.Codex_abc!App");
        assert_eq!(apps[0].launch_kind, LaunchKind::AppUserModelId);
    }

    #[test]
    fn filters_start_apps_maintenance_entries() {
        let apps = parse_start_apps(
            r#"[{"Name":"Visual Studio Installer","AppID":"Microsoft.VisualStudio.Installer"},{"Name":"Uninstall Node.js","AppID":"Microsoft.AutoGenerated.Uninstall"},{"Name":"Visual Studio Code","AppID":"Microsoft.VisualStudioCode"}]"#,
        ).unwrap();
        assert_eq!(apps.iter().map(|app| app.name.as_str()).collect::<Vec<_>>(), vec!["Visual Studio Code"]);
    }

    #[test]
    fn accepts_single_start_app_object() {
        let apps = parse_start_apps(r#"{"Name":"Codex","AppID":"OpenAI.Codex_abc!App"}"#).unwrap();
        assert_eq!(apps.len(), 1);
    }

    #[test]
    fn replaces_certificate_publisher_with_package_vendor() {
        assert_eq!(display_publisher("OpenAI.Codex", Some("CN=certificate".into())).as_deref(), Some("OpenAI"));
    }
}
