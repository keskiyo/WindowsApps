use super::{
    classify, is_invalid_display_name, is_maintenance_entry, stable_id, AppInfo, LaunchKind,
    SourceKind, UninstallTarget,
};
use serde::Deserialize;
use serde_json::Value;
use std::os::windows::process::CommandExt;
use std::process::Command;

const CREATE_NO_WINDOW: u32 = 0x08000000;
const UTF8_PREFIX: &str =
    "$OutputEncoding = [Console]::OutputEncoding = [Text.UTF8Encoding]::new($false); ";
// Enumerate the Apps folder so we can read each item's real launch target
// (System.Link.TargetParsingPath). Get-StartApps only exposes Name/AppID; the target lets
// deduplication merge a localized Start-App (e.g. "Просмотр событий") with the English
// Start-Menu shortcut (Event Viewer) that resolves to the same file (eventvwr.msc).
const START_APPS_SCRIPT: &str = "$f=(New-Object -ComObject Shell.Application).NameSpace('shell:AppsFolder'); $f.Items() | ForEach-Object { [pscustomobject]@{ Name=$_.Name; AppID=$_.ExtendedProperty('System.AppUserModel.ID'); Target=$_.ExtendedProperty('System.Link.TargetParsingPath') } } | ConvertTo-Json -Compress";
// Fallback used only if the Apps-folder enumeration fails (returns no target).
const START_APPS_FALLBACK_SCRIPT: &str =
    "Get-StartApps | Select-Object Name,AppID | ConvertTo-Json -Compress";
const PACKAGES_SCRIPT: &str = "Get-AppxPackage | Select-Object PackageFullName,PackageFamilyName,Name,Publisher,Version,InstallLocation | ConvertTo-Json -Compress";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct StartAppRow {
    name: String,
    #[serde(rename = "AppID")]
    app_id: String,
    #[serde(default)]
    target: Option<String>,
}

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
    let mut apps = parse_start_apps(&apps_json).unwrap_or_default();
    if apps.is_empty() {
        // Apps-folder enumeration failed; fall back to Get-StartApps (no launch target).
        let fallback = run_powershell(START_APPS_FALLBACK_SCRIPT).unwrap_or_default();
        apps = parse_start_apps(&fallback).unwrap_or_default();
    }
    let packages_json = run_powershell(PACKAGES_SCRIPT).unwrap_or_default();
    let packages: Vec<PackageRow> = parse_rows(&packages_json).unwrap_or_default();
    for app in &mut apps {
        if let Some(package) = packages.iter().find(|package| {
            app.path
                .to_lowercase()
                .starts_with(&package.package_family_name.to_lowercase())
        }) {
            app.source_kind = SourceKind::Msix;
            app.publisher = display_publisher(&package.name, package.publisher.clone());
            app.version = package
                .version
                .clone()
                .filter(|value| !value.trim().is_empty());
            app.install_location = package
                .install_location
                .clone()
                .filter(|value| !value.trim().is_empty());
            app.description = Some(package.name.clone());
            app.product_name = Some(package.name.clone());
            app.can_uninstall = true;
            app.uninstall = Some(UninstallTarget::Msix {
                package_full_name: package.package_full_name.clone(),
            });
        }
    }
    apps
}

fn display_publisher(package_name: &str, publisher: Option<String>) -> Option<String> {
    let publisher = publisher
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    if publisher
        .as_deref()
        .is_some_and(|value| value.starts_with("CN="))
    {
        return package_name
            .split('.')
            .next()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
    }
    publisher
}

/// Interpreters/hosts whose behavior is defined by arguments a Start-App target does not
/// carry. Merging by these would collapse distinct tools (Command Prompt vs Node.js prompt
/// vs x64 Native Tools prompt — all cmd.exe). `.msc`/`.cpl` and dedicated tool exes are safe.
fn is_generic_host_target(target: &str) -> bool {
    const HOSTS: &[&str] = &[
        "cmd.exe",
        "powershell.exe",
        "pwsh.exe",
        "mmc.exe",
        "wscript.exe",
        "cscript.exe",
        "rundll32.exe",
        "mshta.exe",
        "conhost.exe",
        "control.exe",
        "explorer.exe",
        "python.exe",
        "pythonw.exe",
        "py.exe",
        "node.exe",
        "java.exe",
        "javaw.exe",
        "mysql.exe",
        "wsl.exe",
        "bash.exe",
        "sh.exe",
    ];
    let file = target
        .trim_matches('"')
        .rsplit(['\\', '/'])
        .next()
        .unwrap_or(target)
        .trim()
        .to_lowercase();
    HOSTS.contains(&file.as_str())
}

fn run_powershell(script: &str) -> Option<String> {
    let script = format!("{UTF8_PREFIX}{script}");
    let output = Command::new("powershell.exe")
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            &script,
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| decode_output(output.stdout))
        .flatten()
}

fn decode_output(bytes: Vec<u8>) -> Option<String> {
    String::from_utf8(bytes)
        .ok()
        .map(|value| value.trim().to_string())
}

pub(super) fn parse_start_apps(json: &str) -> Result<Vec<AppInfo>, serde_json::Error> {
    let rows: Vec<StartAppRow> = parse_rows(json)?;
    Ok(rows
        .into_iter()
        .filter_map(|row| {
            let name = row.name.trim().to_string();
            let app_id = row.app_id.trim().to_string();
            // Real launch target from System.Link.TargetParsingPath — the bridge that lets
            // dedup merge this localized Start-App with the equivalent English shortcut.
            // A Start-App's target has NO arguments, so drop targets that are generic hosts
            // (cmd/powershell/mysql/…): several distinct tools share one interpreter and would
            // wrongly collapse into one. Self-contained targets (.msc/.cpl/mstsc/perfmon/…)
            // are safe and stay.
            let resolved_path = row
                .target
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .filter(|value| !is_generic_host_target(value))
                .map(str::to_string);
            if is_invalid_display_name(&name)
                || app_id.is_empty()
                || is_maintenance_entry(&name, &app_id, resolved_path.as_deref())
            {
                return None;
            }
            Some(AppInfo {
                id: stable_id(&app_id),
                category: classify(&name, &app_id),
                name,
                path: app_id,
                icon_base64: None,
                launch_kind: LaunchKind::AppUserModelId,
                source_kind: SourceKind::StartApps,
                description: None,
                version: None,
                publisher: None,
                product_name: None,
                original_filename: None,
                install_location: None,
                can_uninstall: false,
                uninstall: None,
                resolved_path,
                shortcut_icon_path: None,
                launch_arguments: None,
                canonical_identity: None,
                visibility_class: Default::default(),
                visibility_score: 0,
                visibility_reasons: Vec::new(),
            })
        })
        .collect())
}

fn parse_rows<T: for<'de> Deserialize<'de>>(json: &str) -> Result<Vec<T>, serde_json::Error> {
    if json.trim().is_empty() {
        return Ok(Vec::new());
    }
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
        let apps =
            parse_start_apps(r#"[{"Name":"Codex","AppID":"OpenAI.Codex_abc!App"}]"#).unwrap();
        assert_eq!(apps[0].name, "Codex");
        assert_eq!(apps[0].path, "OpenAI.Codex_abc!App");
        assert_eq!(apps[0].launch_kind, LaunchKind::AppUserModelId);
    }

    #[test]
    fn drops_generic_host_targets_to_avoid_over_merge() {
        // cmd.exe-hosted Start-Apps must NOT get a target (they differ only by arguments a
        // Start-App does not carry) — otherwise Command Prompt, Node.js prompt, etc. collapse.
        let apps = parse_start_apps(
            r#"[{"Name":"Командная строка","AppID":"X\\cmd.exe","Target":"C:\\Windows\\system32\\cmd.exe"},{"Name":"Reload Configuration","AppID":"Microsoft.AutoGenerated.{F5}","Target":"C:\\Program Files\\PostgreSQL\\15\\bin\\pg_ctl.exe"}]"#,
        )
        .unwrap();
        let cmd = apps.iter().find(|a| a.name == "Командная строка").unwrap();
        assert_eq!(cmd.resolved_path, None, "generic host target dropped");
        let reload = apps
            .iter()
            .find(|a| a.name == "Reload Configuration")
            .unwrap();
        assert!(reload.resolved_path.is_some(), "dedicated tool target kept");
    }

    #[test]
    fn resolves_start_app_target_into_resolved_path() {
        let apps = parse_start_apps(
            r#"[{"Name":"Просмотр событий","AppID":"Microsoft.AutoGenerated.{BB044BFD}","Target":"C:\\Windows\\system32\\eventvwr.msc"}]"#,
        )
        .unwrap();
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].name, "Просмотр событий");
        assert_eq!(
            apps[0].resolved_path.as_deref(),
            Some("C:\\Windows\\system32\\eventvwr.msc")
        );
    }

    #[test]
    fn filters_start_apps_maintenance_entries() {
        let apps = parse_start_apps(
            r#"[{"Name":"Visual Studio Installer","AppID":"Microsoft.VisualStudio.Installer"},{"Name":"Uninstall Node.js","AppID":"Microsoft.AutoGenerated.Uninstall"},{"Name":"Visual Studio Code","AppID":"Microsoft.VisualStudioCode"}]"#,
        ).unwrap();
        assert_eq!(
            apps.iter().map(|app| app.name.as_str()).collect::<Vec<_>>(),
            vec!["Visual Studio Code"]
        );
    }

    #[test]
    fn accepts_single_start_app_object() {
        let apps = parse_start_apps(r#"{"Name":"Codex","AppID":"OpenAI.Codex_abc!App"}"#).unwrap();
        assert_eq!(apps.len(), 1);
    }

    #[test]
    fn replaces_certificate_publisher_with_package_vendor() {
        assert_eq!(
            display_publisher("OpenAI.Codex", Some("CN=certificate".into())).as_deref(),
            Some("OpenAI")
        );
    }
}
