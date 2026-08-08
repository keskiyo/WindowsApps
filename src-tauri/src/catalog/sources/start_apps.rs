use crate::catalog::sync::scan_control::ScanControl;
use crate::catalog::{
    classify::classify, filters::is_invalid_display_name, stable_id, AppInfo, LaunchKind,
    SourceKind,
};
use serde::Deserialize;
use serde_json::Value;
use std::os::windows::process::CommandExt;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

mod package;

#[cfg(test)]
use package::display_publisher;
use package::enrich_packages;

const CREATE_NO_WINDOW: u32 = 0x08000000;
const UTF8_PREFIX: &str =
    "$OutputEncoding = [Console]::OutputEncoding = [Text.UTF8Encoding]::new($false); ";
const START_APPS_SCRIPT: &str = "$f=(New-Object -ComObject Shell.Application).NameSpace('shell:AppsFolder'); $f.Items() | ForEach-Object { [pscustomobject]@{ Name=$_.Name; AppID=$_.ExtendedProperty('System.AppUserModel.ID'); Target=$_.ExtendedProperty('System.Link.TargetParsingPath') } } | ConvertTo-Json -Compress";
const START_APPS_FALLBACK_SCRIPT: &str =
    "Get-StartApps | Select-Object Name,AppID | ConvertTo-Json -Compress";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct StartAppRow {
    name: String,
    #[serde(rename = "AppID")]
    app_id: String,
    #[serde(default)]
    target: Option<String>,
}

pub(in crate::catalog) fn scan(control: &ScanControl) -> Option<Vec<AppInfo>> {
    if control.is_cancelled() {
        return None;
    }
    let primary = run_powershell(START_APPS_SCRIPT, control);
    let mut powershell_ran = primary.is_some();
    let mut apps = primary
        .as_deref()
        .and_then(|json| parse_start_apps(json).ok())
        .unwrap_or_default();
    if apps.is_empty() {
        let fallback = run_powershell(START_APPS_FALLBACK_SCRIPT, control);
        powershell_ran = powershell_ran || fallback.is_some();
        apps = fallback
            .as_deref()
            .and_then(|json| parse_start_apps(json).ok())
            .unwrap_or_default();
    }
    if !powershell_ran {
        return None;
    }
    let packages_json = run_powershell(package::QUERY, control).unwrap_or_default();
    enrich_packages(&mut apps, &packages_json);
    Some(apps)
}

const POWERSHELL_TIMEOUT: Duration = Duration::from_secs(25);
const POWERSHELL_POLL_INTERVAL: Duration = Duration::from_millis(50);
const MAX_POWERSHELL_OUTPUT_BYTES: u64 = 16 * 1024 * 1024;

fn run_powershell(script: &str, control: &ScanControl) -> Option<String> {
    let script = format!("{UTF8_PREFIX}{script}");
    let mut child = Command::new(crate::platform::windows::exec_target::system_powershell())
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            &script,
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .ok()?;

    let stdout = child.stdout.take();
    let reader = std::thread::spawn(move || {
        let mut buffer = Vec::new();
        if let Some(stdout) = stdout {
            let _ = std::io::Read::read_to_end(
                &mut std::io::Read::take(stdout, MAX_POWERSHELL_OUTPUT_BYTES),
                &mut buffer,
            );
        }
        buffer
    });

    let deadline = Instant::now() + POWERSHELL_TIMEOUT;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) => {}
            Err(_) => break None,
        }
        if control.is_cancelled() || Instant::now() >= deadline {
            break None;
        }
        std::thread::sleep(POWERSHELL_POLL_INTERVAL);
    };

    if status.is_none() {
        let _ = child.kill();
        let _ = child.wait();
    }
    let stdout = reader.join().ok()?;
    status
        .filter(std::process::ExitStatus::success)
        .and_then(|_| decode_output(stdout))
}

fn decode_output(bytes: Vec<u8>) -> Option<String> {
    String::from_utf8(bytes)
        .ok()
        .map(|value| value.trim().to_string())
}

pub(in crate::catalog) fn parse_start_apps(json: &str) -> Result<Vec<AppInfo>, serde_json::Error> {
    let rows: Vec<StartAppRow> = parse_rows(json)?;
    let facts = crate::catalog::machine::MachineFacts::current();
    Ok(rows
        .into_iter()
        .filter_map(|row| {
            let name = row.name.trim().to_string();
            let app_id = row.app_id.trim().to_string();
            let resolved_path = row
                .target
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string);
            if is_invalid_display_name(&name) || app_id.is_empty() {
                return None;
            }
            let mut app = AppInfo {
                id: stable_id(&app_id),
                category: classify(&name, &app_id),
                name,
                path: app_id,
                icon_base64: None,
                artifact_kind: Default::default(),
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
                preference_identity: None,
                visibility_class: Default::default(),
                visibility_score: 0,
                visibility_reasons: Vec::new(),
                target_availability: None,
                category_reasons: Vec::new(),
                close_risk: None,
            };
            app.artifact_kind = crate::catalog::artifact::classify(&app, None, &facts);
            if app.artifact_kind != crate::catalog::ArtifactKind::Application {
                app.category = crate::catalog::AppCategory::InstallersDocs;
            }
            Some(app)
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
    fn a_cancelled_scan_abandons_a_long_running_interpreter() {
        let cancelled = || true;
        let control = ScanControl::new(&cancelled);
        let started = Instant::now();

        let output = run_powershell("Start-Sleep -Seconds 120", &control);

        assert_eq!(output, None);
        assert!(
            started.elapsed() < POWERSHELL_TIMEOUT,
            "cancellation waited for the timeout instead of returning"
        );
    }

    #[test]
    fn a_completed_interpreter_returns_output_larger_than_the_pipe_buffer() {
        let never = || false;
        let control = ScanControl::new(&never);

        let output = run_powershell("'x' * 200000", &control);

        assert_eq!(output.as_deref().map(str::len), Some(200_000));
    }

    #[test]
    fn a_failed_interpreter_reports_no_output() {
        let never = || false;
        let control = ScanControl::new(&never);

        assert_eq!(run_powershell("exit 3", &control), None);
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
    fn preserves_generic_host_targets_as_launch_evidence() {
        let apps = parse_start_apps(
            r#"[{"Name":"Ubuntu","AppID":"Microsoft.AutoGenerated.{WSL}","Target":"C:\\Program Files\\WSL\\wsl.exe"},{"Name":"Командная строка","AppID":"X\\cmd.exe","Target":"C:\\Windows\\system32\\cmd.exe"},{"Name":"Reload Configuration","AppID":"Microsoft.AutoGenerated.{F5}","Target":"C:\\Program Files\\PostgreSQL\\15\\bin\\pg_ctl.exe"}]"#,
        )
        .unwrap();
        let ubuntu = apps.iter().find(|app| app.name == "Ubuntu").unwrap();
        assert_eq!(
            ubuntu.resolved_path.as_deref(),
            Some(r"C:\Program Files\WSL\wsl.exe")
        );
        let cmd = apps.iter().find(|a| a.name == "Командная строка").unwrap();
        assert_eq!(
            cmd.resolved_path.as_deref(),
            Some(r"C:\Windows\system32\cmd.exe")
        );
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
    fn trusted_mysql_installer_target_is_classified_without_parsing_the_aumid() {
        let apps = parse_start_apps(
            r#"[{"Name":"MySQL Installer - Community","AppID":"Microsoft.AutoGenerated.{2C7C9013-A7FD-5C12-D811-939AFE2A3592}","Target":"C:\\Program Files (x86)\\MySQL\\MySQL Installer for Windows\\MySQLInstaller.exe"}]"#,
        )
        .unwrap();

        assert_eq!(
            apps[0].artifact_kind,
            crate::catalog::ArtifactKind::Installer
        );
        assert_eq!(
            apps[0].category,
            crate::catalog::AppCategory::InstallersDocs
        );
    }

    #[test]
    fn parses_url_backed_start_apps_as_documentation() {
        let apps = parse_start_apps(
            r#"[{"Name":"Node.js website","AppID":"https://nodejs.org/","Target":"https://nodejs.org/"}]"#,
        )
        .unwrap();

        assert_eq!(apps.len(), 1);
        assert_eq!(
            apps[0].artifact_kind,
            crate::catalog::ArtifactKind::Documentation
        );
        assert_eq!(
            apps[0].category,
            crate::catalog::AppCategory::InstallersDocs
        );
    }

    #[test]
    fn keeps_start_apps_with_product_words_in_names_or_aumids() {
        let apps = parse_start_apps(
            r#"[{"Name":"Visual Studio Installer","AppID":"Microsoft.VisualStudio.Installer"},{"Name":"Uninstall Node.js","AppID":"Microsoft.AutoGenerated.Uninstall"},{"Name":"Visual Studio Code","AppID":"Microsoft.VisualStudioCode"}]"#,
        ).unwrap();
        assert_eq!(
            apps.iter().map(|app| app.name.as_str()).collect::<Vec<_>>(),
            vec![
                "Visual Studio Installer",
                "Uninstall Node.js",
                "Visual Studio Code"
            ]
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

    fn package_json(application_id: &str, executable: &str) -> String {
        serde_json::json!({
            "PackageFullName": "OpenAI.Codex_26.727.4816.0_x64__2p2nqsd0c76g0",
            "PackageFamilyName": "OpenAI.Codex_2p2nqsd0c76g0",
            "Name": "OpenAI.Codex",
            "Publisher": "CN=certificate",
            "Version": "26.727.4816.0",
            "InstallLocation": r"C:\Program Files\WindowsApps\OpenAI.Codex_26.727.4816.0_x64__2p2nqsd0c76g0",
            "Applications": [{ "Id": application_id, "Executable": executable }]
        })
        .to_string()
    }

    #[test]
    fn resolves_packaged_app_executable_from_exact_manifest_application() {
        let mut apps =
            parse_start_apps(r#"{"Name":"Codex","AppID":"OpenAI.Codex_2p2nqsd0c76g0!App"}"#)
                .unwrap();

        enrich_packages(&mut apps, &package_json("App", "app/ChatGPT.exe"));

        assert_eq!(apps[0].source_kind, SourceKind::Msix);
        assert_eq!(apps[0].publisher.as_deref(), Some("OpenAI"));
        assert_eq!(
            apps[0].resolved_path.as_deref(),
            Some(
                r"C:\Program Files\WindowsApps\OpenAI.Codex_26.727.4816.0_x64__2p2nqsd0c76g0\app\ChatGPT.exe"
            )
        );
    }

    #[test]
    fn rejects_unmatched_or_escaping_package_manifest_executables() {
        for (application_id, executable) in [
            ("Other", "app/ChatGPT.exe"),
            ("App", r"..\Outside.exe"),
            ("App", r"C:\Outside.exe"),
            ("App", "app/readme.txt"),
        ] {
            let mut apps =
                parse_start_apps(r#"{"Name":"Codex","AppID":"OpenAI.Codex_2p2nqsd0c76g0!App"}"#)
                    .unwrap();

            enrich_packages(&mut apps, &package_json(application_id, executable));

            assert_eq!(
                apps[0].resolved_path, None,
                "unsafe manifest target was accepted: {executable}"
            );
        }
    }
}
