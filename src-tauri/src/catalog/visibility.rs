use super::{AppInfo, LaunchKind, SourceKind};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum VisibilityClass {
    #[default]
    Primary,
    Auxiliary,
    Rejected,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum VisibilityReason {
    StartMenuRegistration,
    WindowsAppRegistration,
    SteamRegistration,
    PortableCandidate,
    ProductMetadata,
    RegisteredProduct,
    ExecutableProductMatch,
    RuntimeDirectory,
    ProductComponent,
    DocumentationShortcut,
    Installer,
    MaintenanceExecutable,
    CommandEnvironment,
    ConsoleApplication,
    InsufficientLaunchEvidence,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct VisibilityDecision {
    pub class: VisibilityClass,
    pub score: i16,
    pub reasons: Vec<VisibilityReason>,
}

pub(crate) fn classify_visibility(app: &AppInfo) -> VisibilityDecision {
    let name = app.name.to_lowercase();
    let path = app.path.to_lowercase().replace('/', r"\");
    let resolved_path = app
        .resolved_path
        .as_deref()
        .unwrap_or_default()
        .to_lowercase()
        .replace('/', r"\");
    let description = app
        .description
        .as_deref()
        .unwrap_or_default()
        .to_lowercase();
    let product_name = app
        .product_name
        .as_deref()
        .unwrap_or_default()
        .to_lowercase();
    let original_filename = app
        .original_filename
        .as_deref()
        .unwrap_or_default()
        .to_lowercase();
    let value =
        format!("{name} {path} {resolved_path} {original_filename} {product_name} {description}");
    let mut score = 0;
    let mut reasons = Vec::new();

    match app.source_kind {
        SourceKind::Steam => {
            score += 60;
            reasons.push(VisibilityReason::SteamRegistration);
        }
        SourceKind::StartApps | SourceKind::Msix => {
            score += 60;
            reasons.push(VisibilityReason::WindowsAppRegistration);
        }
        SourceKind::StartMenu => {
            score += 45;
            reasons.push(VisibilityReason::StartMenuRegistration);
        }
        SourceKind::Portable => {
            score += 10;
            reasons.push(VisibilityReason::PortableCandidate);
        }
        SourceKind::Registry => {}
    }

    if app.source_kind == SourceKind::Registry && app.can_uninstall {
        score += 35;
        reasons.push(VisibilityReason::RegisteredProduct);
    }

    if is_installer_or_uninstaller(&value) {
        return VisibilityDecision {
            class: VisibilityClass::Rejected,
            score: -100,
            reasons: vec![VisibilityReason::Installer],
        };
    }
    if is_documentation(&value) {
        return VisibilityDecision {
            class: VisibilityClass::Rejected,
            score: -80,
            reasons: vec![VisibilityReason::DocumentationShortcut],
        };
    }
    if is_maintenance_executable(&value) {
        return VisibilityDecision {
            class: VisibilityClass::Rejected,
            score: -70,
            reasons: vec![VisibilityReason::MaintenanceExecutable],
        };
    }

    if app
        .publisher
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty())
        && app
            .description
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
    {
        score += 25;
        reasons.push(VisibilityReason::ProductMetadata);
    }

    if executable_matches_product(app) {
        score += 15;
        reasons.push(VisibilityReason::ExecutableProductMatch);
    }

    if has_runtime_path(&path) || has_runtime_path(&resolved_path) {
        score -= 20;
        reasons.push(VisibilityReason::RuntimeDirectory);
    }
    if is_product_component(&value) || is_bundled_toolchain_path(&path) {
        score -= 20;
        reasons.push(VisibilityReason::ProductComponent);
    }
    if is_command_environment(app) {
        reasons.push(VisibilityReason::CommandEnvironment);
    }

    // A command environment and a product component are auxiliary regardless of score or launch
    // kind — including the AppUserModelId fast-path, which is exactly how the VS developer prompts
    // reach the catalog.
    let class = if reasons.iter().any(|reason| {
        matches!(
            reason,
            VisibilityReason::ProductComponent | VisibilityReason::CommandEnvironment
        )
    }) {
        VisibilityClass::Auxiliary
    } else if score >= 20 || app.launch_kind == LaunchKind::AppUserModelId {
        VisibilityClass::Primary
    } else {
        reasons.push(VisibilityReason::InsufficientLaunchEvidence);
        VisibilityClass::Auxiliary
    };

    VisibilityDecision {
        class,
        score,
        reasons,
    }
}

fn executable_matches_product(app: &AppInfo) -> bool {
    let stem = Path::new(&app.path)
        .file_stem()
        .and_then(|value| value.to_str())
        .map(normalized_identity_text)
        .unwrap_or_default();
    if stem.len() < 3 {
        return false;
    }
    [&app.name, app.product_name.as_deref().unwrap_or_default()]
        .iter()
        .map(|value| normalized_identity_text(value))
        .any(|value| value == stem || value.starts_with(&stem))
}

fn normalized_identity_text(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn is_bundled_toolchain_path(path: &str) -> bool {
    contains_any(
        path,
        &[
            r"\.vscode\extensions\",
            r"\git\usr\bin\",
            r"\git\mingw32\",
            r"\git\mingw64\",
            r"\codex-runtimes\",
            r"\sdk\samples\",
        ],
    )
}

/// A command environment opens a configured interpreter shell (a VS developer prompt, a
/// Node.js prompt) rather than an application. A regular user launching software does not pick
/// these; they belong in Auxiliary tools. Plain `Windows PowerShell` / `Command Prompt` — an
/// interpreter with no arguments — stay primary.
fn is_command_environment(app: &AppInfo) -> bool {
    let name = app.name.to_lowercase();
    const NAME_MARKERS: &[&str] = &[
        "developer command prompt",
        "developer powershell",
        "native tools command prompt",
        "cross tools command prompt",
        "command prompt for vs",
        "command line client",
        // A setup/toolchain action ("Install Additional Tools for Node.js") is not an app.
        "additional tools for node",
        // A diagnostic "safe mode" launcher opens the real app in a recovery mode — not the way
        // a user launches it day to day.
        "safe mode",
        "безопасный режим",
    ];
    if NAME_MARKERS.iter().any(|marker| name.contains(marker)) {
        return true;
    }
    let has_arguments = app
        .launch_arguments
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty());
    has_arguments
        && app
            .resolved_path
            .as_deref()
            .is_some_and(is_interpreter_host_path)
}

fn is_interpreter_host_path(path: &str) -> bool {
    let file = Path::new(path.trim().trim_matches('"'))
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(path)
        .to_ascii_lowercase();
    matches!(
        file.as_str(),
        "cmd.exe"
            | "powershell.exe"
            | "pwsh.exe"
            | "wscript.exe"
            | "cscript.exe"
            | "mshta.exe"
            | "rundll32.exe"
            | "python.exe"
            | "pythonw.exe"
            | "py.exe"
            | "mysql.exe"
            | "node.exe"
            | "wsl.exe"
    )
}

pub(crate) fn apply_visibility(app: &mut AppInfo) {
    let decision = classify_visibility(app);
    app.visibility_class = decision.class;
    app.visibility_score = decision.score;
    app.visibility_reasons = decision.reasons;
}

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
    if !super::dedup::dev_report_enabled() {
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

fn contains_any(value: &str, markers: &[&str]) -> bool {
    markers.iter().any(|marker| value.contains(marker))
}

fn is_installer_or_uninstaller(value: &str) -> bool {
    contains_any(
        value,
        &[
            " uninstall",
            "uninstall ",
            "unins000",
            " setup",
            "setup.exe",
            "installer",
            "tsetup",
            "vcredist",
            "redist",
        ],
    )
}

fn is_documentation(value: &str) -> bool {
    contains_any(
        value,
        &[
            " faq",
            "faqs",
            "documentation",
            "installation notes",
            "release notes",
            "readme",
            "manual",
            "документация",
            "справка",
            "руководство",
        ],
    )
}

fn is_maintenance_executable(value: &str) -> bool {
    contains_any(
        value,
        &[
            "update-service",
            "update_service",
            " updater",
            "update.exe",
            "crashhandler",
            "crash handler",
            "crashpad",
            "uninstall.exe",
            // "Reset preferences and cache files"-style shortcuts run the app in a wipe mode
            // instead of opening it; they belong with maintenance, not the player itself.
            "reset preferences",
            "reset cache",
            "reset config",
            "reset settings",
            "сброс настроек",
            "сброс кэш",
        ],
    )
}

fn has_runtime_path(path: &str) -> bool {
    contains_any(
        path,
        &[
            r"\bin\",
            r"\lib\",
            r"\runtime\",
            r"\jre\",
            r"\sdk\",
            r"\plugins\",
            r"\resources\",
            r"\node_modules\",
        ],
    )
}

fn is_product_component(value: &str) -> bool {
    contains_any(
        value,
        &[
            "iconv.exe",
            "intelliphp.ls",
            "language server",
            "openjdk platform binary",
            // Oracle Java runtime entries ("Java(TM) Platform SE", "Java(TM) SE") are the JRE, a
            // runtime component, not an application a user launches.
            "java(tm)",
            "the curl executable",
            "openssl command",
            "credential manager",
            "gettext",
            "git-lfs",
            "git large file storage",
            "sandbox",
            "compiler",
            " helper",
            "_helper",
            "-helper",
            " service.exe",
            " daemon",
            // A bundled utility executable (`AstUtil.exe`, `*Util.exe`) is a maintenance/config
            // helper of its product, not the application a user launches.
            "util.exe",
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{AppInfo, LaunchKind, SourceKind};
    use serde::Deserialize;

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Fixture {
        name: String,
        path: String,
        source: SourceKind,
        expected: String,
        description: Option<String>,
        publisher: Option<String>,
        original_filename: Option<String>,
        resolved_path: Option<String>,
        can_uninstall: Option<bool>,
    }

    fn candidate(name: &str, path: &str, source_kind: SourceKind) -> AppInfo {
        AppInfo {
            id: name.into(),
            name: name.into(),
            path: path.into(),
            icon_base64: None,
            category: Default::default(),
            launch_kind: if path.ends_with(".lnk") {
                LaunchKind::Shortcut
            } else {
                LaunchKind::Executable
            },
            source_kind,
            description: None,
            version: None,
            publisher: None,
            product_name: None,
            original_filename: None,
            install_location: None,
            can_uninstall: false,
            uninstall: None,
            resolved_path: None,
            shortcut_icon_path: None,
            launch_arguments: None,
            canonical_identity: None,
            visibility_class: Default::default(),
            visibility_score: 0,
            visibility_reasons: Vec::new(),
        }
    }

    #[test]
    fn keeps_explicit_user_launchers_primary() {
        let git_bash = candidate(
            "Git Bash",
            r"C:\ProgramData\Microsoft\Windows\Start Menu\Programs\Git\Git Bash.lnk",
            SourceKind::StartMenu,
        );
        assert_eq!(
            classify_visibility(&git_bash).class,
            VisibilityClass::Primary
        );
    }

    #[test]
    fn shortcut_to_runtime_component_is_auxiliary_despite_start_menu_location() {
        let mut shortcut = candidate(
            "PHP Tools",
            r"C:\ProgramData\Microsoft\Windows\Start Menu\Programs\PHP Tools.lnk",
            SourceKind::StartMenu,
        );
        shortcut.resolved_path = Some(r"C:\Editor\plugins\intelliphp.ls.exe".into());

        let decision = classify_visibility(&shortcut);

        assert_eq!(decision.class, VisibilityClass::Auxiliary);
        assert!(decision
            .reasons
            .contains(&VisibilityReason::ProductComponent));
    }

    #[test]
    fn classifies_known_product_components_as_auxiliary() {
        for (name, path, description) in [
            ("iconv", r"C:\Git\usr\bin\iconv.exe", None),
            (
                "intelliphp.ls",
                r"C:\Editor\plugins\intelliphp.ls.exe",
                Some("PHP language server"),
            ),
            (
                "OpenJDK Platform binary",
                r"C:\Java\runtime\bin\javaw.exe",
                Some("OpenJDK Platform binary"),
            ),
            (
                "The curl executable",
                r"C:\Git\mingw64\bin\curl.exe",
                Some("The curl executable"),
            ),
        ] {
            let mut app = candidate(name, path, SourceKind::Portable);
            app.description = description.map(str::to_string);
            assert_eq!(
                classify_visibility(&app).class,
                VisibilityClass::Auxiliary,
                "{name}"
            );
        }
    }

    #[test]
    fn rejects_installers_uninstallers_and_maintenance_shortcuts() {
        for (name, path) in [
            ("Telegram Desktop Setup", r"C:\Downloads\tsetup.exe"),
            ("Uninstall Git", r"C:\Git\Uninstall Git.lnk"),
            ("Git FAQs", r"C:\Git\Git FAQs.lnk"),
            ("Example Update Service", r"C:\Example\update-service.exe"),
        ] {
            let app = candidate(name, path, SourceKind::Portable);
            assert_eq!(
                classify_visibility(&app).class,
                VisibilityClass::Rejected,
                "{name}"
            );
        }
    }

    #[test]
    fn keeps_unknown_apps_conservative_instead_of_rejecting_them() {
        let app = candidate(
            "Example Studio",
            r"D:\Apps\Example Studio\ExampleStudio.exe",
            SourceKind::Portable,
        );
        assert_ne!(classify_visibility(&app).class, VisibilityClass::Rejected);
        assert!(!classify_visibility(&app).reasons.is_empty());
    }

    #[test]
    fn promotes_unknown_portable_apps_with_coherent_product_metadata() {
        let mut app = candidate(
            "Example Studio",
            r"D:\Apps\Example Studio\ExampleStudio.exe",
            SourceKind::Portable,
        );
        app.publisher = Some("Example Software".into());
        app.description = Some("Example Studio desktop application".into());

        let decision = classify_visibility(&app);

        assert_eq!(decision.class, VisibilityClass::Primary);
        assert!(decision
            .reasons
            .contains(&VisibilityReason::ProductMetadata));
    }

    #[test]
    fn keeps_registered_products_with_an_uninstaller_primary() {
        let mut app = candidate(
            "Example Editor",
            r"C:\Program Files\Example\Editor.exe",
            SourceKind::Registry,
        );
        app.can_uninstall = true;

        let decision = classify_visibility(&app);

        assert_eq!(decision.class, VisibilityClass::Primary);
        assert!(decision
            .reasons
            .contains(&VisibilityReason::RegisteredProduct));
    }

    #[test]
    fn original_filename_exposes_a_renamed_helper() {
        let mut app = candidate(
            "Workspace Agent",
            r"D:\Apps\Workspace\random-name.exe",
            SourceKind::Portable,
        );
        app.original_filename = Some("notification_helper.exe".into());
        app.product_name = Some("Workspace".into());

        let decision = classify_visibility(&app);

        assert_eq!(decision.class, VisibilityClass::Auxiliary);
        assert!(decision
            .reasons
            .contains(&VisibilityReason::ProductComponent));
    }

    #[test]
    fn neutral_original_filename_does_not_override_registered_product_evidence() {
        let mut app = candidate(
            "Example Editor",
            r"C:\Program Files\Example\Editor.exe",
            SourceKind::Registry,
        );
        app.can_uninstall = true;
        app.original_filename = Some("editor.exe".into());
        app.product_name = Some("Example Editor".into());

        assert_eq!(classify_visibility(&app).class, VisibilityClass::Primary);
    }

    #[test]
    fn original_filename_exposes_a_renamed_installer() {
        let mut app = candidate(
            "Workspace Download",
            r"D:\Apps\Workspace\payload.exe",
            SourceKind::Portable,
        );
        app.original_filename = Some("product-setup.exe".into());

        let decision = classify_visibility(&app);

        assert_eq!(decision.class, VisibilityClass::Rejected);
        assert_eq!(decision.reasons, vec![VisibilityReason::Installer]);
    }

    #[test]
    fn product_matched_cli_in_user_bin_remains_primary() {
        let mut app = candidate(
            "Claude Code",
            r"C:\Users\Maks\.local\bin\claude.exe",
            SourceKind::Portable,
        );
        app.product_name = Some("Claude Code".into());
        app.publisher = Some("Anthropic PBC".into());
        app.description = Some("Claude Code".into());

        let decision = classify_visibility(&app);

        assert_eq!(decision.class, VisibilityClass::Primary);
        assert!(decision
            .reasons
            .contains(&VisibilityReason::ExecutableProductMatch));
    }

    #[test]
    fn bundled_toolchain_binary_remains_auxiliary_despite_product_metadata() {
        let mut app = candidate(
            "The OpenSSL Toolkit",
            r"D:\Git\mingw64\bin\openssl.exe",
            SourceKind::Portable,
        );
        app.product_name = Some("The OpenSSL Toolkit".into());
        app.publisher = Some("The OpenSSL Project".into());
        app.description = Some("OpenSSL application".into());

        assert_eq!(classify_visibility(&app).class, VisibilityClass::Auxiliary);
    }

    #[test]
    fn debug_report_redacts_the_user_profile_prefix() {
        let profile = r"C:\Users\Maks";
        assert_eq!(
            redact_path(r"C:\Users\Maks\Downloads\setup.exe", Some(profile)),
            r"<USERPROFILE>\Downloads\setup.exe"
        );
    }

    // "VLC media player - reset preferences and cache files" runs vlc.exe in a wipe mode; it must
    // not be shown as the player.
    #[test]
    fn vlc_reset_maintenance_shortcut_is_rejected() {
        let mut app = candidate(
            "VLC media player - reset preferences and cache files",
            r"C:\ProgramData\Microsoft\Windows\Start Menu\Programs\VideoLAN\VLC media player - reset preferences and cache files.lnk",
            SourceKind::StartMenu,
        );
        app.resolved_path = Some(r"C:\Program Files\VideoLAN\VLC\vlc.exe".into());

        assert_eq!(classify_visibility(&app).class, VisibilityClass::Rejected);
    }

    // A VS developer prompt reaches the catalog as an AUMID Start-App, which normally forces
    // Primary. As a command environment it must still be Auxiliary.
    #[test]
    fn visual_studio_command_prompt_is_auxiliary_despite_aumid() {
        let mut native = candidate(
            "x64 Native Tools Command Prompt for VS 2019",
            "Microsoft.AutoGenerated.{FF70D809-A022-5972-850F-19E0AA8C07C2}",
            SourceKind::StartApps,
        );
        native.launch_kind = LaunchKind::AppUserModelId;

        let decision = classify_visibility(&native);
        assert_eq!(decision.class, VisibilityClass::Auxiliary);
        assert!(decision
            .reasons
            .contains(&VisibilityReason::CommandEnvironment));
    }

    // A `cmd.exe /k <bat>` wrapper is a command environment; a plain interpreter with no
    // arguments is a normal launcher entry and stays Primary.
    #[test]
    fn interpreter_wrapper_is_auxiliary_but_plain_shell_stays_primary() {
        let mut node = candidate(
            "Node.js command prompt",
            r"C:\ProgramData\Microsoft\Windows\Start Menu\Programs\Node.js\Node.js command prompt.lnk",
            SourceKind::StartMenu,
        );
        node.resolved_path = Some(r"C:\Windows\System32\cmd.exe".into());
        node.launch_arguments = Some(r#"/k "C:\Program Files\nodejs\nodevars.bat""#.into());
        assert_eq!(classify_visibility(&node).class, VisibilityClass::Auxiliary);

        let mut shell = candidate(
            "Windows PowerShell",
            r"C:\Menu\Windows PowerShell\Windows PowerShell.lnk",
            SourceKind::StartMenu,
        );
        shell.resolved_path =
            Some(r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe".into());
        assert_eq!(classify_visibility(&shell).class, VisibilityClass::Primary);
    }

    // The command-environment rule requires an interpreter host: a normal application launched
    // with arguments (but resolving to its own exe) must not be demoted to auxiliary.
    #[test]
    fn app_with_arguments_but_non_host_target_stays_primary() {
        let mut app = candidate("Some App", r"C:\Menu\Some App.lnk", SourceKind::StartMenu);
        app.resolved_path = Some(r"C:\Program Files\SomeApp\app.exe".into());
        app.launch_arguments = Some("--flag value".into());
        assert_eq!(classify_visibility(&app).class, VisibilityClass::Primary);
    }

    // Real corpus: interpreter-hosted dev tools and a diagnostic launcher are auxiliary, not
    // applications a launcher user picks to open.
    #[test]
    fn python_idle_mysql_client_and_safe_mode_are_auxiliary() {
        let mut idle = candidate(
            "IDLE (Python 3.14 64-bit)",
            r"C:\Menu\Python 3.14\IDLE (Python 3.14 64-bit).lnk",
            SourceKind::StartMenu,
        );
        idle.resolved_path = Some(r"C:\Python314\pythonw.exe".into());
        idle.launch_arguments = Some(r#""C:\Python314\Lib\idlelib\idle.pyw""#.into());
        assert_eq!(classify_visibility(&idle).class, VisibilityClass::Auxiliary);

        let mut mysql = candidate(
            "MySQL 8.0 Command Line Client",
            r"C:\Menu\MySQL\MySQL 8.0 Command Line Client.lnk",
            SourceKind::StartMenu,
        );
        mysql.resolved_path = Some(r"C:\Program Files\MySQL\MySQL Server 8.0\bin\mysql.exe".into());
        mysql.launch_arguments = Some(r#""-uroot" "-p""#.into());
        assert_eq!(
            classify_visibility(&mysql).class,
            VisibilityClass::Auxiliary
        );

        // Name-based: soffice.exe is the app itself, so only the "safe mode" name marks it.
        let mut safe = candidate(
            "LibreOffice (Безопасный режим)",
            r"C:\Menu\LibreOffice\LibreOffice (Безопасный режим).lnk",
            SourceKind::StartMenu,
        );
        safe.resolved_path = Some(r"C:\Program Files\LibreOffice\program\soffice.exe".into());
        safe.launch_arguments = Some("--safe-mode".into());
        assert_eq!(classify_visibility(&safe).class, VisibilityClass::Auxiliary);
    }

    // A `rundll32.exe`-hosted config shortcut (Configure x264vfw) and a setup Start-App (Install
    // Additional Tools for Node.js) are not applications.
    #[test]
    fn rundll32_config_and_node_setup_are_auxiliary() {
        let mut x264 = candidate(
            "Configure x264vfw",
            r"C:\Menu\x264vfw\Configure x264vfw.lnk",
            SourceKind::StartMenu,
        );
        x264.resolved_path = Some(r"C:\Windows\SysWOW64\rundll32.exe".into());
        x264.launch_arguments = Some("x264vfw.dll,Configure".into());
        assert_eq!(classify_visibility(&x264).class, VisibilityClass::Auxiliary);

        let mut node = candidate(
            "Install Additional Tools for Node.js",
            "Microsoft.AutoGenerated.{04770C2D}",
            SourceKind::StartApps,
        );
        node.launch_kind = LaunchKind::AppUserModelId;
        assert_eq!(classify_visibility(&node).class, VisibilityClass::Auxiliary);
    }

    // A bundled `*Util.exe` (e.g. AstUtil.exe of "Ассистент") is a utility component, not the app.
    #[test]
    fn bundled_utility_executable_is_auxiliary() {
        let app = candidate(
            "Ассистент",
            r"D:\разный хлам\Ассистент\AstUtil.exe",
            SourceKind::Portable,
        );
        assert_eq!(classify_visibility(&app).class, VisibilityClass::Auxiliary);
    }

    #[test]
    fn oracle_java_runtime_entries_are_auxiliary() {
        for name in ["Java(TM) Platform SE", "Java(TM) SE Development Kit"] {
            let app = candidate(
                name,
                r"C:\Program Files\Java\jre\bin\javaw.exe",
                SourceKind::StartMenu,
            );
            assert_eq!(
                classify_visibility(&app).class,
                VisibilityClass::Auxiliary,
                "{name}"
            );
        }
    }

    #[test]
    fn synthetic_fixture_corpus_matches_manual_labels() {
        let fixtures: Vec<Fixture> =
            serde_json::from_str(include_str!("../../tests/fixtures/catalog_visibility.json"))
                .unwrap();
        assert!(
            fixtures.len() < 100,
            "synthetic corpus must not be presented as real-world validation"
        );
        for fixture in fixtures {
            let mut app = candidate(&fixture.name, &fixture.path, fixture.source);
            app.description = fixture.description;
            app.publisher = fixture.publisher;
            app.original_filename = fixture.original_filename;
            app.resolved_path = fixture.resolved_path;
            app.can_uninstall = fixture.can_uninstall.unwrap_or(false);
            let actual = classify_visibility(&app);
            let matches = match fixture.expected.as_str() {
                "primary" => actual.class == VisibilityClass::Primary,
                "auxiliary" | "uncertain" => actual.class == VisibilityClass::Auxiliary,
                "rejected" => actual.class == VisibilityClass::Rejected,
                label => panic!("unknown fixture label: {label}"),
            };
            assert!(
                matches,
                "{}: expected {}, got {:?} ({:?})",
                fixture.name, fixture.expected, actual.class, actual.reasons
            );
        }
    }

    #[test]
    fn applies_explainable_decision_to_catalog_entry() {
        let mut app = candidate("iconv", r"C:\Git\usr\bin\iconv.exe", SourceKind::Portable);

        apply_visibility(&mut app);

        assert_eq!(app.visibility_class, VisibilityClass::Auxiliary);
        assert!(app.visibility_score < 0);
        assert!(app
            .visibility_reasons
            .contains(&VisibilityReason::ProductComponent));
    }
}
