use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::env;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};

pub mod cache;
mod portable;
mod registry;
pub mod scan_settings;
mod start_apps;
mod steam;

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AppCategory {
    Games,
    Ai,
    Editors,
    Development,
    Browsers,
    Media,
    Communication,
    Utilities,
    System,
    WindowsFeatures,
    #[default]
    Other,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LaunchKind {
    #[default]
    Executable,
    Shortcut,
    AppUserModelId,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    #[default]
    Registry,
    StartMenu,
    StartApps,
    Msix,
    Steam,
    Portable,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UninstallTarget {
    Command {
        executable: String,
        arguments: String,
    },
    Msix {
        package_full_name: String,
    },
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub id: String,
    pub name: String,
    pub path: String,
    pub icon_base64: Option<String>,
    #[serde(default)]
    pub category: AppCategory,
    #[serde(default)]
    pub launch_kind: LaunchKind,
    #[serde(default)]
    pub source_kind: SourceKind,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub publisher: Option<String>,
    #[serde(default)]
    pub install_location: Option<String>,
    #[serde(default)]
    pub can_uninstall: bool,
    #[serde(default)]
    pub uninstall: Option<UninstallTarget>,
    #[serde(skip)]
    pub resolved_path: Option<String>,
    #[serde(skip)]
    pub shortcut_icon_path: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanProgress {
    pub stage: String,
    pub location: Option<String>,
    pub completed_roots: usize,
    pub total_roots: usize,
}

pub fn discover_apps_with(
    settings: &scan_settings::ScanSettings,
    progress: impl Fn(ScanProgress),
    is_cancelled: impl Fn() -> bool + Sync,
) -> Vec<AppInfo> {
    progress(ScanProgress {
        stage: "Windows applications".into(),
        location: None,
        completed_roots: 0,
        total_roots: 0,
    });
    let (mut apps, registry_metadata) = scan_registry();
    apps.extend(scan_start_menu());
    apps.extend(start_apps::scan());
    if is_cancelled() {
        return sanitize(apps);
    }

    let steam_libraries = steam::installed_libraries();
    progress(ScanProgress {
        stage: "Steam libraries".into(),
        location: None,
        completed_roots: 0,
        total_roots: steam_libraries.len(),
    });
    for (index, library) in steam_libraries.iter().enumerate() {
        if is_cancelled() {
            break;
        }
        progress(ScanProgress {
            stage: "Steam libraries".into(),
            location: Some(library.to_string_lossy().into_owned()),
            completed_roots: index,
            total_roots: steam_libraries.len(),
        });
        apps.extend(steam::scan_library(library).into_iter().map(steam_app));
    }

    let mut roots = if settings.auto_scan_fixed_drives {
        crate::platform::windows::drives::fixed_drive_roots()
    } else {
        Vec::new()
    };
    roots.extend(
        settings
            .included_paths
            .iter()
            .map(PathBuf::from)
            .filter(|path| path.is_dir()),
    );
    roots.sort_by_cached_key(|path| path.to_string_lossy().to_lowercase());
    roots.dedup_by(|left, right| {
        left.to_string_lossy()
            .eq_ignore_ascii_case(&right.to_string_lossy())
    });
    let mut excluded = default_portable_exclusions();
    excluded.extend(settings.excluded_paths.iter().map(PathBuf::from));
    excluded.extend(steam_libraries);
    progress(ScanProgress {
        stage: "Portable applications".into(),
        location: None,
        completed_roots: 0,
        total_roots: roots.len(),
    });
    let portable_paths = std::thread::scope(|scope| {
        let (sender, receiver) = std::sync::mpsc::channel();
        for root in &roots {
            let sender = sender.clone();
            let excluded = &excluded;
            let is_cancelled = &is_cancelled;
            scope.spawn(move || {
                let paths = portable::discover_executables(
                    std::slice::from_ref(root),
                    excluded,
                    is_cancelled,
                );
                let _ = sender.send((root.to_path_buf(), paths));
            });
        }
        drop(sender);
        let mut paths = Vec::new();
        for (index, (root, found)) in receiver.into_iter().enumerate() {
            progress(ScanProgress {
                stage: "Portable applications".into(),
                location: Some(root.to_string_lossy().into_owned()),
                completed_roots: index + 1,
                total_roots: roots.len(),
            });
            paths.extend(found);
        }
        paths
    });
    apps.extend(portable_paths.into_iter().filter_map(portable_app));
    attach_registry_metadata(&mut apps, &registry_metadata);
    enrich_local_metadata(&mut apps);
    sanitize(apps)
}

fn steam_app(game: steam::SteamGame) -> AppInfo {
    let path = format!("steam://rungameid/{}", game.app_id);
    AppInfo {
        id: stable_id(&path),
        category: AppCategory::Games,
        name: game.name,
        path,
        icon_base64: None,
        launch_kind: LaunchKind::Executable,
        source_kind: SourceKind::Steam,
        description: None,
        version: None,
        publisher: None,
        install_location: Some(game.install_dir.to_string_lossy().into_owned()),
        can_uninstall: false,
        uninstall: None,
        resolved_path: find_executable(&game.install_dir.to_string_lossy())
            .map(|path| path.to_string_lossy().into_owned()),
        shortcut_icon_path: None,
    }
}

fn portable_app(path: PathBuf) -> Option<AppInfo> {
    let metadata = crate::platform::windows::executable_metadata::read(&path);
    let has_metadata = metadata.product_name.is_some()
        || metadata.description.is_some()
        || metadata.publisher.is_some();
    let stem = path.file_stem()?.to_string_lossy().trim().to_string();
    let parent_matches = path
        .parent()
        .and_then(Path::file_name)
        .is_some_and(|parent| {
            normalized_portable_name(&parent.to_string_lossy()) == normalized_portable_name(&stem)
        });
    if !has_metadata && !parent_matches {
        return None;
    }
    let name = metadata
        .product_name
        .clone()
        .filter(|value| !is_generic_product_name(value))
        .unwrap_or_else(|| clean_portable_name(&stem));
    if is_maintenance_entry(&name, &path.to_string_lossy(), None) {
        return None;
    }
    let mut app = make_app(name, path.clone());
    app.source_kind = SourceKind::Portable;
    app.description = metadata.description;
    app.version = metadata.version;
    app.publisher = metadata.publisher;
    app.install_location = path
        .parent()
        .map(|value| value.to_string_lossy().into_owned());
    Some(app)
}

fn is_generic_product_name(value: &str) -> bool {
    ["godot engine", "electron", "chromium", "application"]
        .iter()
        .any(|generic| value.trim().eq_ignore_ascii_case(generic))
}

fn normalized_portable_name(value: &str) -> String {
    clean_portable_name(value)
        .chars()
        .filter(|character| character.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn clean_portable_name(value: &str) -> String {
    let trimmed = value.trim();
    let version_start = trimmed
        .char_indices()
        .find(|(index, character)| {
            *index > 0 && character.is_ascii_digit() && trimmed[..*index].ends_with(['-', '_', ' '])
        })
        .map(|(index, _)| index.saturating_sub(1));
    version_start
        .map_or(trimmed, |index| &trimmed[..index])
        .replace(['_', '-'], " ")
        .trim()
        .to_string()
}

fn default_portable_exclusions() -> Vec<PathBuf> {
    [
        "WINDIR",
        "ProgramFiles",
        "ProgramFiles(x86)",
        "ProgramData",
        "APPDATA",
        "LOCALAPPDATA",
    ]
    .into_iter()
    .filter_map(env::var_os)
    .map(PathBuf::from)
    .collect()
}

fn scan_registry() -> (Vec<AppInfo>, Vec<registry::RegistryMetadata>) {
    let mut apps = Vec::new();
    let mut metadata = Vec::new();
    let scans = [
        registry::scan(
            HKEY_LOCAL_MACHINE,
            r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
        ),
        registry::scan(
            HKEY_LOCAL_MACHINE,
            r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
        ),
        registry::scan(
            HKEY_CURRENT_USER,
            r"Software\Microsoft\Windows\CurrentVersion\Uninstall",
        ),
    ];
    for scan in scans {
        apps.extend(scan.apps);
        metadata.extend(scan.metadata);
    }
    (apps, metadata)
}

pub fn enrich_registered_uninstall_metadata(apps: &mut [AppInfo]) {
    let (_, metadata) = scan_registry();
    attach_registry_metadata(apps, &metadata);
}

fn attach_registry_metadata(apps: &mut [AppInfo], metadata: &[registry::RegistryMetadata]) {
    for app in apps.iter_mut().filter(|app| app.uninstall.is_none()) {
        let matches = metadata
            .iter()
            .filter(|record| registry_metadata_matches(app, record))
            .collect::<Vec<_>>();
        let Some(first) = matches.first() else {
            continue;
        };
        if !matches
            .iter()
            .all(|record| record.uninstall == first.uninstall)
        {
            continue;
        }
        app.uninstall = Some(first.uninstall.clone());
        app.can_uninstall = true;
        if app.description.is_none() {
            app.description = first.description.clone();
        }
        if app.version.is_none() {
            app.version = first.version.clone();
        }
        if app.publisher.is_none() {
            app.publisher = first.publisher.clone();
        }
        if app.install_location.is_none() {
            app.install_location = first.install_location.clone();
        }
    }
}

fn registry_metadata_matches(app: &AppInfo, record: &registry::RegistryMetadata) -> bool {
    if normalized_product_family(&app.name) != normalized_product_family(&record.name) {
        return false;
    }
    match (&app.publisher, &record.publisher) {
        (Some(app_publisher), Some(record_publisher)) => {
            app_publisher.eq_ignore_ascii_case(record_publisher)
        }
        _ => true,
    }
}

pub fn sanitize(apps: Vec<AppInfo>) -> Vec<AppInfo> {
    deduplicate(
        apps.into_iter()
            .filter(|app| !is_maintenance_entry(&app.name, &app.path, app.resolved_path.as_deref()))
            .collect(),
    )
}

fn enrich_local_metadata(apps: &mut [AppInfo]) {
    for app in apps {
        let target = app.resolved_path.as_deref().unwrap_or(&app.path);
        let path = Path::new(target);
        if !path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
            || !path.is_file()
        {
            continue;
        }
        let metadata = crate::platform::windows::executable_metadata::read(path);
        crate::platform::windows::executable_metadata::fill_missing(
            &mut app.description,
            &mut app.version,
            &mut app.publisher,
            &metadata,
        );
        if app.install_location.is_none() {
            app.install_location = path
                .parent()
                .map(|parent| parent.to_string_lossy().into_owned());
        }
    }
}

pub fn reuse_cached_icons(apps: &mut [AppInfo], cached: &[AppInfo]) {
    for app in apps {
        if let Some(icon) = cached
            .iter()
            .find(|cached| cached.id == app.id)
            .and_then(|cached| cached.icon_base64.clone())
        {
            app.icon_base64 = Some(icon);
        }
    }
}

fn icon_source(app: &AppInfo) -> Option<String> {
    app.shortcut_icon_path
        .clone()
        .filter(|path| Path::new(path).is_file())
        .or_else(|| {
            app.resolved_path
                .clone()
                .filter(|path| Path::new(path).is_file())
        })
        .or_else(|| (app.launch_kind != LaunchKind::AppUserModelId).then(|| app.path.clone()))
}

pub fn hydrate_missing_icons(apps: &mut [AppInfo]) {
    for app in apps.iter_mut().filter(|app| app.icon_base64.is_none()) {
        app.icon_base64 = if app.launch_kind == LaunchKind::AppUserModelId {
            crate::platform::windows::icon_extractor::extract_app_id_icon(&app.path)
        } else {
            icon_source(app).and_then(|path| {
                crate::platform::windows::icon_extractor::extract_icon(Path::new(&path))
            })
        };
    }
}

fn scan_start_menu() -> Vec<AppInfo> {
    let mut roots = vec![PathBuf::from(
        r"C:\ProgramData\Microsoft\Windows\Start Menu\Programs",
    )];
    if let Some(appdata) = env::var_os("APPDATA") {
        roots.push(PathBuf::from(appdata).join(r"Microsoft\Windows\Start Menu\Programs"));
    }

    roots
        .into_iter()
        .flat_map(|root| WalkDir::new(root).follow_links(false).into_iter())
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter(|entry| {
            entry
                .path()
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("lnk"))
        })
        .filter_map(|entry| {
            let path = entry.into_path();
            let name = path.file_stem()?.to_string_lossy().trim().to_string();
            let details = crate::platform::windows::shortcut::resolve(&path);
            let target = details
                .target
                .as_ref()
                .map(|value| value.to_string_lossy().into_owned());
            (!name.is_empty()
                && !is_maintenance_entry(&name, &path.to_string_lossy(), target.as_deref()))
            .then(|| {
                let mut app = make_app(name, path);
                app.source_kind = SourceKind::StartMenu;
                app.resolved_path = target;
                app.shortcut_icon_path = details
                    .icon_location
                    .map(|value| value.to_string_lossy().into_owned());
                app
            })
        })
        .collect()
}

fn make_app(name: String, path: PathBuf) -> AppInfo {
    let path = path.to_string_lossy().to_string();
    let normalized = path.to_lowercase();
    let id = format!("{:x}", Sha256::digest(normalized.as_bytes()));
    let category = classify(&name, &path);
    let launch_kind = if Path::new(&path)
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("lnk"))
    {
        LaunchKind::Shortcut
    } else {
        LaunchKind::Executable
    };
    AppInfo {
        id,
        name,
        path,
        icon_base64: None,
        category,
        launch_kind,
        source_kind: SourceKind::Registry,
        description: None,
        version: None,
        publisher: None,
        install_location: None,
        can_uninstall: false,
        uninstall: None,
        resolved_path: None,
        shortcut_icon_path: None,
    }
}

fn stable_id(identity: &str) -> String {
    format!(
        "{:x}",
        Sha256::digest(identity.trim().to_lowercase().as_bytes())
    )
}

fn classify(name: &str, path: &str) -> AppCategory {
    let value = format!("{name} {path}").to_lowercase();
    let contains = |keywords: &[&str]| keywords.iter().any(|keyword| value.contains(keyword));

    if is_windows_feature(&value) {
        AppCategory::WindowsFeatures
    } else if contains(&[
        "steam",
        "battle.net",
        "epic games",
        "gog",
        "game",
        "minecraft",
        "roblox",
        "backpack battles",
        "warcraft",
    ]) {
        AppCategory::Games
    } else if contains(&[
        "claude",
        "chatgpt",
        "openai",
        "codex",
        "ollama",
        "lm studio",
        "gemini",
        "copilot",
        "cursor",
        "ai agent",
    ]) {
        AppCategory::Ai
    } else if contains(&[
        "visual studio",
        "vscode",
        "code.exe",
        "rustrover",
        "pycharm",
        "webstorm",
        "intellij",
        "android studio",
        "git",
        "docker",
        "postman",
        "terminal",
    ]) {
        AppCategory::Development
    } else if contains(&[
        "photoshop",
        "illustrator",
        "lightroom",
        "figma",
        "gimp",
        "inkscape",
        "blender",
        "paint",
        "canva",
    ]) {
        AppCategory::Editors
    } else if contains(&[
        "chrome", "firefox", "edge", "opera", "brave", "vivaldi", "browser",
    ]) {
        AppCategory::Browsers
    } else if contains(&[
        "vlc",
        "spotify",
        "foobar",
        "audacity",
        "obs",
        "media player",
        "music",
        "video",
    ]) {
        AppCategory::Media
    } else if contains(&[
        "discord", "telegram", "whatsapp", "slack", "teams", "zoom", "skype", "signal",
    ]) {
        AppCategory::Communication
    } else if contains(&[
        "character map",
        "command prompt",
        "powershell",
        "windows terminal",
        "task manager",
        "таблица символов",
        "командная строка",
        "диспетчер задач",
    ]) {
        AppCategory::System
    } else if contains(&[
        "7-zip",
        "winrar",
        "everything",
        "powertoys",
        "rufus",
        "verifier",
        "configure",
        "utility",
        "uninstaller",
    ]) {
        AppCategory::Utilities
    } else {
        AppCategory::Other
    }
}

fn is_windows_feature(value: &str) -> bool {
    const FEATURES: &[&str] = &[
        // Windows shell, inbox applications and accessibility tools.
        "file explorer",
        "explorer.exe",
        "проводник",
        "snipping tool",
        "snippingtool.exe",
        "microsoft.screensketch",
        "ножницы",
        "get help",
        "microsoft.gethelp",
        "техническая поддержка",
        "remote desktop connection",
        "mstsc.exe",
        "подключение к удаленному рабочему столу",
        "calculator",
        "microsoft.windowscalculator",
        "калькулятор",
        "notepad",
        "microsoft.windowsnotepad",
        "блокнот",
        "microsoft paint",
        "mspaint.exe",
        "microsoft.paint",
        "камера",
        "microsoft.windowscamera",
        "clock",
        "microsoft.windowsalarms",
        "часы",
        "voice recorder",
        "sound recorder",
        "microsoft.windowssoundrecorder",
        "запись голоса",
        "звукозапись",
        "magnifier",
        "magnify.exe",
        "экранная лупа",
        "on-screen keyboard",
        "osk.exe",
        "экранная клавиатура",
        "narrator",
        "narrator.exe",
        "экранный диктор",
        "quick assist",
        "microsoftcorporationii.quickassist",
        "быстрая помощь",
        "windows security",
        "microsoft.sechealthui",
        "безопасность windows",
        "windows settings",
        "systemsettings.exe",
        "параметры",
        "microsoft.windows.administrativetools",
        "инструменты windows",
        "windows backup",
        "windowsbackup",
        "архивация windows",
        "microsoft.windowsstore",
        "microsoft store",
        "microsoft.windows.photos",
        "фотографии",
        "microsoft.bingweather",
        "погода",
        "microsoft.bingnews",
        "новости",
        "microsoft.microsoftstickynotes",
        "sticky notes",
        "записки",
        "microsoft.yourphone",
        "phone link",
        "связь с телефоном",
        "microsoft.windowsfeedbackhub",
        "feedback hub",
        "центр отзывов",
        "microsoft.windows.shell.rundialog",
        "выполнить",
        "windows media player legacy",
        "steps recorder",
        "psr.exe",
        "средство записи действий",
        "memory diagnostics tool",
        "mdsched.exe",
        "средство проверки памяти windows",
        "recoverydrive",
        "recoverydrive.exe",
        "диск восстановления",
        "iscsi initiator",
        "iscsicpl.exe",
        "инициатор iscsi",
        // Management consoles and administrative tools.
        "computer management",
        "compmgmt.msc",
        "управление компьютером",
        "print management",
        "printmanagement.msc",
        "управление печатью",
        "device manager",
        "devmgmt.msc",
        "диспетчер устройств",
        "disk management",
        "diskmgmt.msc",
        "управление дисками",
        "event viewer",
        "eventvwr.msc",
        "просмотр событий",
        "task scheduler",
        "taskschd.msc",
        "планировщик задач",
        "планировщик заданий",
        "local security policy",
        "secpol.msc",
        "локальная политика безопасности",
        "group policy",
        "gpedit.msc",
        "групповая политика",
        "system configuration",
        "msconfig.exe",
        "конфигурация системы",
        "resource monitor",
        "resmon.exe",
        "монитор ресурсов",
        "performance monitor",
        "perfmon.msc",
        "системный монитор",
        "component services",
        "comexp.msc",
        "службы компонентов",
        "odbc data sources",
        "odbcad32.exe",
        "источники данных odbc",
        "system information",
        "msinfo32.exe",
        "сведения о системе",
        "control panel",
        "control.exe",
        "панель управления",
        "administrative tools",
        "windows tools",
        "администрирование",
        "средства windows",
        "registry editor",
        "regedit.exe",
        "редактор реестра",
        "disk cleanup",
        "cleanmgr.exe",
        "очистка диска",
        "defragment",
        "dfrgui.exe",
        "дефрагментация",
        "windows defender firewall",
        "wf.msc",
        "брандмауэр",
        "system restore",
        "rstrui.exe",
        "восстановление системы",
        "services.msc",
        "службы",
        "character map",
        "charmap.exe",
        "таблица символов",
        "command prompt",
        "cmd.exe",
        "командная строка",
        "windows powershell",
        "powershell.exe",
        "windows terminal",
        "microsoft.windowsterminal",
        "task manager",
        "taskmgr.exe",
        "диспетчер задач",
    ];

    FEATURES.iter().any(|feature| value.contains(feature))
}

fn find_executable(location: &str) -> Option<PathBuf> {
    find_executable_named(location, None)
}

/// Resolve a launchable file inside an install directory. When `name` is given,
/// prefer the executable whose file name matches the application name (e.g. pick
/// `Docker Desktop.exe`, not the first bundled `courgette64.exe` found in the tree).
fn find_executable_named(location: &str, name: Option<&str>) -> Option<PathBuf> {
    let root = PathBuf::from(location.trim().trim_matches('"'));
    if is_launchable(&root) {
        return Some(root);
    }
    if !root.is_dir() {
        return None;
    }
    let target = name
        .map(normalized_portable_name)
        .filter(|key| !key.is_empty());
    WalkDir::new(&root)
        .max_depth(2)
        .into_iter()
        .filter_map(Result::ok)
        .map(|entry| entry.into_path())
        .filter(|path| is_launchable(path) && !is_maintenance_path(&path.to_string_lossy()))
        .min_by_key(|path| {
            let stem = path
                .file_stem()
                .map(|value| normalized_portable_name(&value.to_string_lossy()))
                .unwrap_or_default();
            let name_score = match &target {
                Some(target) if stem == *target => 0u8,
                Some(target)
                    if !stem.is_empty() && (stem.contains(target) || target.contains(&stem)) =>
                {
                    1
                }
                Some(_) => 3,
                None => 2,
            };
            let depth = path
                .strip_prefix(&root)
                .map(|relative| relative.components().count())
                .unwrap_or(usize::MAX);
            (name_score, depth, path.to_string_lossy().into_owned())
        })
}

fn is_launchable(path: &Path) -> bool {
    path.is_file()
        && path.extension().is_some_and(|extension| {
            extension.eq_ignore_ascii_case("exe") || extension.eq_ignore_ascii_case("lnk")
        })
}

fn clean_display_icon(value: &str) -> Option<PathBuf> {
    let trimmed = value.trim().trim_start_matches('\\').trim();
    let path = if let Some(rest) = trimmed.strip_prefix('"') {
        rest.split('"').next().unwrap_or(rest)
    } else {
        trimmed.split(',').next().unwrap_or(trimmed)
    };
    let path = path.trim().trim_matches('"');
    (!path.is_empty()).then(|| PathBuf::from(path))
}

fn is_invalid_display_name(name: &str) -> bool {
    let name = name.trim();
    name.is_empty() || name.to_lowercase().starts_with("ms-resource:") || name.contains('\u{fffd}')
}

fn is_noise(name: &str, path: &str) -> bool {
    is_maintenance_entry(name, path, None)
}

fn is_maintenance_entry(name: &str, path: &str, resolved_path: Option<&str>) -> bool {
    if is_invalid_display_name(name) {
        return true;
    }
    is_documentation_name(name)
        || is_maintenance_path(path)
        || resolved_path.is_some_and(is_maintenance_path)
        || is_maintenance_text(name)
}

fn is_maintenance_path(path: &str) -> bool {
    let path_buf = Path::new(path);
    if path_buf.extension().is_some_and(|extension| {
        [
            "ico", "dll", "mui", "cpl", "chm", "pdf", "html", "htm", "txt", "rtf", "md", "url",
            "hlp", "xml", "log", "ini",
        ]
        .iter()
        .any(|value| extension.eq_ignore_ascii_case(value))
    }) {
        return true;
    }
    if path_buf
        .file_stem()
        .is_some_and(|stem| is_installer_file_name(&stem.to_string_lossy()))
    {
        return true;
    }
    is_maintenance_text(path)
}

/// Junk-detection for installer/updater executables by file name (stem, no extension).
/// Splits the stem into alphanumeric tokens to catch `setup-app`, `app-installer`,
/// `setup_x64`, and also matches glued names like `AppSetup` via prefix/suffix.
pub(crate) fn is_installer_file_name(stem: &str) -> bool {
    let lower = stem.to_lowercase();
    // 7-Zip installers are named like `7z2501-x64`; the real app is `7zFM`/`7zG`/`7z`.
    if lower.starts_with("7z")
        && lower[2..]
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_digit())
    {
        return true;
    }
    const TOKENS: [&str; 8] = [
        "setup",
        "unins",
        "unins000",
        "updater",
        "bootstrapper",
        "установщик",
        "деинсталляция",
        "удаление",
    ];
    let has_token = lower
        .split(|character: char| !character.is_alphanumeric())
        .filter(|token| !token.is_empty())
        // `contains` catches install/installer/instaler/installation, uninstall, and
        // vcredist2005_x64 / vc_redist / redistributables, including misspellings.
        .any(|token| {
            token.contains("instal") || token.contains("redist") || TOKENS.contains(&token)
        });
    if has_token {
        return true;
    }
    ["setup", "install", "uninstall"]
        .iter()
        .any(|marker| lower.ends_with(marker))
}

/// Junk-detection for documentation / website shortcut display names.
/// Matches whole words at the start or end (so "HelpDesk Pro" survives) plus a few
/// multi-word phrases anywhere in the normalized name.
fn is_documentation_name(name: &str) -> bool {
    const WORDS: [&str; 25] = [
        "documentation",
        "docs",
        "readme",
        "manual",
        "help",
        "faq",
        "license",
        "licence",
        "eula",
        "changelog",
        "tutorial",
        "website",
        "homepage",
        "support",
        "samples",
        "sample",
        "sdk",
        "example",
        "examples",
        "demo",
        "документация",
        "справка",
        "руководство",
        "лицензия",
        "сайт",
    ];
    const PHRASES: [&str; 9] = [
        "release notes",
        "what's new",
        "home page",
        "getting started",
        "visit website",
        "support center",
        "заметки о выпуске",
        "что нового",
        "веб сайт",
    ];
    let normalized = normalize_name(name);
    if PHRASES.iter().any(|phrase| normalized.contains(phrase)) {
        return true;
    }
    let words = normalized.split_whitespace().collect::<Vec<_>>();
    match (words.first(), words.last()) {
        (Some(first), Some(last)) => WORDS.contains(first) || WORDS.contains(last),
        _ => false,
    }
}

fn is_maintenance_text(value: &str) -> bool {
    let value = value.to_lowercase();
    [
        "uninstall",
        "unins000",
        "installer",
        "installation notes",
        "setup.exe",
        "updater",
        "update.exe",
        "repair.exe",
        "bootstrap",
        "remove ",
        "delete ",
        "удалить",
        "деинстал",
        "microsoft visual c++ update",
        "hotfix",
        "security update",
        "redistributable",
        "subprocess",
        "kb[",
        "file://",
    ]
    .iter()
    .any(|needle| value.contains(needle))
}

fn deduplicate(apps: Vec<AppInfo>) -> Vec<AppInfo> {
    let mut unique = Vec::<AppInfo>::new();
    for app in apps {
        if let Some(index) = unique
            .iter()
            .position(|existing| same_application(existing, &app))
        {
            let existing = unique.remove(index);
            unique.insert(index, merge_app(existing, app));
        } else {
            unique.push(app);
        }
    }
    let mut apps = unique;
    for app in &mut apps {
        app.name = app.name.split_whitespace().collect::<Vec<_>>().join(" ");
        app.id = format!("{:x}", Sha256::digest(app.path.to_lowercase().as_bytes()));
        app.category = classify(&app.name, &app.path);
    }
    apps.sort_by_cached_key(|app| (category_rank(app.category), app.name.to_lowercase()));
    apps
}

fn same_application(left: &AppInfo, right: &AppInfo) -> bool {
    if left.path.eq_ignore_ascii_case(&right.path) {
        return true;
    }
    let left_identity = left.resolved_path.as_deref().unwrap_or(&left.path);
    let right_identity = right.resolved_path.as_deref().unwrap_or(&right.path);
    if left_identity.eq_ignore_ascii_case(right_identity) {
        return true;
    }
    let left_name = normalized_product_family(&left.name);
    let right_name = normalized_product_family(&right.name);
    if left_name != right_name {
        return false;
    }
    if left.launch_kind == LaunchKind::AppUserModelId
        || right.launch_kind == LaunchKind::AppUserModelId
    {
        return true;
    }
    // A Start-Menu shortcut and a loose executable that share a product family are the
    // same app (e.g. Firefox.lnk + firefox.exe). The shortcut wins via `candidate_score`,
    // so the bare .exe duplicate is dropped. Publisher mismatch between a registry/shortcut
    // label and the exe's signing metadata must not split them.
    if left.launch_kind == LaunchKind::Shortcut || right.launch_kind == LaunchKind::Shortcut {
        return true;
    }
    match (&left.publisher, &right.publisher) {
        (Some(left), Some(right)) => left.eq_ignore_ascii_case(right),
        _ => true,
    }
}

fn merge_app(left: AppInfo, right: AppInfo) -> AppInfo {
    let prefer_right = candidate_score(&right) > candidate_score(&left)
        || (candidate_score(&right) == candidate_score(&left)
            && left.source_kind == SourceKind::Portable
            && right.source_kind == SourceKind::Portable
            && version_key(right.version.as_deref()) > version_key(left.version.as_deref()));
    let (mut primary, secondary) = if prefer_right {
        (right, left)
    } else {
        (left, right)
    };
    if primary.description.is_none() {
        primary.description = secondary.description;
    }
    if primary.version.is_none() {
        primary.version = secondary.version;
    }
    if primary.publisher.is_none()
        || primary
            .publisher
            .as_deref()
            .is_some_and(|value| value.starts_with("CN="))
    {
        if secondary
            .publisher
            .as_deref()
            .is_some_and(|value| !value.starts_with("CN="))
        {
            primary.publisher = secondary.publisher;
        }
    }
    if primary.install_location.is_none() {
        primary.install_location = secondary.install_location;
    }
    if primary.icon_base64.is_none() {
        primary.icon_base64 = secondary.icon_base64;
    }
    if primary.uninstall.is_none() {
        primary.uninstall = secondary.uninstall;
    }
    primary.can_uninstall |= secondary.can_uninstall || primary.uninstall.is_some();
    primary
}

fn version_key(version: Option<&str>) -> Vec<u64> {
    version
        .unwrap_or_default()
        .split(|character: char| !character.is_ascii_digit())
        .filter_map(|segment| segment.parse().ok())
        .collect()
}

fn normalize_name(name: &str) -> String {
    name.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn normalized_product_family(name: &str) -> String {
    let mut value = normalize_name(name);
    for marker in [
        " (64bit)",
        " (32bit)",
        " (64-bit)",
        " (32-bit)",
        " x64",
        " x86",
    ] {
        if value.ends_with(marker) {
            value.truncate(value.len() - marker.len());
        }
    }
    if let Some((family, suffix)) = value.split_once(" - ") {
        let generic_suffix = ["proxy utility", "desktop app", "application"]
            .iter()
            .any(|marker| suffix.starts_with(marker));
        let has_version = suffix.chars().any(|character| character.is_ascii_digit());
        if generic_suffix && has_version {
            return family.trim().to_string();
        }
    }
    let mut family = version_family(&value).trim().to_string();
    if family.ends_with(" version") {
        family.truncate(family.len() - " version".len());
    }
    family
}

fn version_family(name: &str) -> &str {
    let Some((family, suffix)) = name.rsplit_once(' ') else {
        return name;
    };
    if !suffix.starts_with(|character: char| character.is_ascii_digit()) {
        return name;
    }
    let numeric_segments = suffix
        .split(|character: char| !character.is_ascii_digit())
        .filter(|segment| !segment.is_empty())
        .count();
    if numeric_segments >= 2 {
        family
    } else {
        name
    }
}

fn candidate_score(app: &AppInfo) -> u8 {
    if app.source_kind == SourceKind::Steam {
        return 5;
    }
    match Path::new(&app.path)
        .extension()
        .and_then(|value| value.to_str())
    {
        Some(extension) if extension.eq_ignore_ascii_case("lnk") => return 4,
        Some(extension) if extension.eq_ignore_ascii_case("exe") => return 3,
        _ => {}
    }
    if app.launch_kind == LaunchKind::AppUserModelId {
        2
    } else {
        0
    }
}

fn category_rank(category: AppCategory) -> u8 {
    match category {
        AppCategory::Games => 0,
        AppCategory::Ai => 1,
        AppCategory::Editors => 2,
        AppCategory::Development => 3,
        AppCategory::Browsers => 4,
        AppCategory::Media => 5,
        AppCategory::Communication => 6,
        AppCategory::Utilities => 7,
        AppCategory::System => 8,
        AppCategory::WindowsFeatures => 9,
        AppCategory::Other => 10,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn app(name: &str, path: &str) -> AppInfo {
        AppInfo {
            id: String::new(),
            name: name.to_string(),
            path: path.to_string(),
            icon_base64: None,
            category: AppCategory::Other,
            launch_kind: LaunchKind::Executable,
            source_kind: SourceKind::Registry,
            description: None,
            version: None,
            publisher: None,
            install_location: None,
            can_uninstall: false,
            uninstall: None,
            resolved_path: None,
            shortcut_icon_path: None,
        }
    }

    fn registry_metadata(
        name: &str,
        publisher: Option<&str>,
        executable: &str,
    ) -> registry::RegistryMetadata {
        registry::RegistryMetadata {
            name: name.into(),
            description: None,
            version: None,
            publisher: publisher.map(String::from),
            install_location: None,
            uninstall: UninstallTarget::Command {
                executable: executable.into(),
                arguments: String::new(),
            },
        }
    }

    #[test]
    fn attaches_registered_uninstall_to_matching_shortcut() {
        let mut apps = vec![app("Steam", r"C:\Menu\Steam.lnk")];
        apps[0].launch_kind = LaunchKind::Shortcut;
        attach_registry_metadata(
            &mut apps,
            &[registry_metadata(
                "Steam",
                Some("Valve"),
                r"C:\Steam\uninstall.exe",
            )],
        );
        assert!(apps[0].can_uninstall);
        assert_eq!(apps[0].publisher.as_deref(), Some("Valve"));
        assert_eq!(
            apps[0].uninstall,
            Some(UninstallTarget::Command {
                executable: r"C:\Steam\uninstall.exe".into(),
                arguments: String::new(),
            })
        );
    }

    #[test]
    fn attaches_version_labelled_registry_entry_to_plain_shortcut_name() {
        let mut apps = vec![app("Ollama", r"C:\Menu\Ollama.lnk")];
        attach_registry_metadata(
            &mut apps,
            &[registry_metadata(
                "Ollama version 0.24.0",
                Some("Ollama"),
                r"C:\Ollama\unins000.exe",
            )],
        );
        assert!(apps[0].can_uninstall);
    }

    #[test]
    fn does_not_attach_ambiguous_uninstall_commands() {
        let mut apps = vec![app("Studio", r"C:\Menu\Studio.lnk")];
        attach_registry_metadata(
            &mut apps,
            &[
                registry_metadata("Studio", None, r"C:\Alpha\uninstall.exe"),
                registry_metadata("Studio", None, r"C:\Beta\uninstall.exe"),
            ],
        );
        assert!(!apps[0].can_uninstall);
        assert!(apps[0].uninstall.is_none());
    }

    #[test]
    fn does_not_attach_metadata_from_a_conflicting_publisher() {
        let mut apps = vec![app("Studio", r"C:\Menu\Studio.lnk")];
        apps[0].publisher = Some("Alpha".into());
        attach_registry_metadata(
            &mut apps,
            &[registry_metadata(
                "Studio",
                Some("Beta"),
                r"C:\Beta\uninstall.exe",
            )],
        );
        assert!(!apps[0].can_uninstall);
    }

    #[test]
    fn cleans_display_icon_resource_suffix() {
        assert_eq!(
            clean_display_icon(r#"\"C:\Apps\Editor.exe\",0"#),
            Some(PathBuf::from(r"C:\Apps\Editor.exe"))
        );
    }

    #[test]
    fn deduplicates_paths_case_insensitively() {
        let apps = deduplicate(vec![
            app("Editor", r"C:\Apps\Editor.exe"),
            app("Editor", r"c:\apps\EDITOR.exe"),
        ]);
        assert_eq!(apps.len(), 1);
    }

    #[test]
    fn deduplicates_equal_normalized_names() {
        let apps = deduplicate(vec![
            app("Claude", r"C:\Registry\Claude.exe"),
            app("  claude  ", r"C:\Start Menu\Claude.lnk"),
            app("CLAUDE", r"C:\Desktop\Claude.lnk"),
        ]);
        assert_eq!(apps.len(), 1);
    }

    #[test]
    fn prefers_existing_shortcut_over_executable() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("Claude.exe");
        let shortcut = dir.path().join("Claude.lnk");
        std::fs::write(&exe, []).unwrap();
        std::fs::write(&shortcut, []).unwrap();
        let apps = deduplicate(vec![
            app("Claude", &exe.to_string_lossy()),
            app("Claude", &shortcut.to_string_lossy()),
        ]);
        assert_eq!(apps[0].path, shortcut.to_string_lossy());
    }

    #[test]
    fn prefers_existing_shortcut_over_packaged_duplicate() {
        let dir = tempfile::tempdir().unwrap();
        let shortcut = dir.path().join("Claude.lnk");
        std::fs::write(&shortcut, []).unwrap();
        let desktop = app("Claude", &shortcut.to_string_lossy());
        let mut packaged = app("Claude", "Claude.Package!App");
        packaged.launch_kind = LaunchKind::AppUserModelId;
        let apps = deduplicate(vec![packaged, desktop]);
        assert_eq!(apps[0].path, shortcut.to_string_lossy());
    }

    #[test]
    fn keeps_different_version_names() {
        let apps = deduplicate(vec![
            app("Editor 1", r"C:\Editor1.exe"),
            app("Editor 2", r"C:\Editor2.exe"),
        ]);
        assert_eq!(apps.len(), 2);
    }

    #[test]
    fn sorts_apps_by_name_within_category_case_insensitively() {
        let apps = deduplicate(vec![
            app("Zeta Workspace", r"C:\z.exe"),
            app("Alpha Workspace", r"C:\a.exe"),
        ]);
        assert_eq!(apps[0].name, "Alpha Workspace");
    }

    #[test]
    fn detects_installer_file_names_by_token() {
        assert!(is_installer_file_name("setup-app"));
        assert!(is_installer_file_name("app-installer"));
        assert!(is_installer_file_name("setup_x64"));
        assert!(is_installer_file_name("appsetup"));
        assert!(is_installer_file_name("unins000"));
        assert!(is_installer_file_name("vcredist_x64"));
        assert!(is_installer_file_name("vcredist2005_x64"));
        assert!(is_installer_file_name("vc_redist.x64"));
        assert!(is_installer_file_name("7z2501-x64"));
        assert!(is_installer_file_name("app_instaler"));
        assert!(!is_installer_file_name("7zFM"));
        assert!(!is_installer_file_name("notepad"));
        assert!(!is_installer_file_name("setupbox"));
        assert!(!is_installer_file_name("aida64"));
    }

    #[test]
    fn detects_documentation_display_names() {
        assert!(is_documentation_name("Документация AIDA64 Extreme"));
        assert!(is_documentation_name("AIDA64 Documentation"));
        assert!(is_documentation_name("Release Notes"));
        assert!(is_documentation_name("What's New"));
        assert!(is_documentation_name("Samples"));
        assert!(is_documentation_name("MSI Afterburner SDK"));
        assert!(is_documentation_name("Steam Support Center"));
        assert!(!is_documentation_name("HelpDesk Pro"));
        assert!(!is_documentation_name("AIDA64 Extreme"));
        assert!(!is_documentation_name("Visual Studio Code"));
    }

    #[test]
    fn maintenance_entry_filters_installers_and_doc_shortcuts() {
        assert!(is_maintenance_entry(
            "Документация AIDA64 Extreme",
            r"C:\Menu\Документация AIDA64 Extreme.lnk",
            Some(r"C:\Program Files\AIDA64\aida64.chm"),
        ));
        assert!(is_maintenance_entry(
            "AIDA64 Setup",
            r"C:\Apps\setup-app.exe",
            None,
        ));
        assert!(!is_maintenance_entry(
            "AIDA64 Extreme",
            r"C:\Program Files\AIDA64\aida64.exe",
            None,
        ));
    }

    #[test]
    fn identifies_uninstaller_noise() {
        assert!(is_noise("Microsoft Visual C++ Update", r"C:\update.exe"));
        assert!(is_noise("Editor Uninstall", r"C:\uninstall.exe"));
        assert!(!is_noise("Visual Studio Code", r"C:\Code.exe"));
    }

    #[test]
    fn finds_executable_inside_registered_install_location() {
        let dir = tempfile::tempdir().unwrap();
        let executable = dir.path().join("Warhammer 40000 Space Marine 2.exe");
        std::fs::write(&executable, []).unwrap();

        assert_eq!(
            find_executable(&dir.path().to_string_lossy()),
            Some(executable)
        );
    }

    #[test]
    fn prefers_named_executable_over_bundled_helpers() {
        let dir = tempfile::tempdir().unwrap();
        let main = dir.path().join("Docker Desktop.exe");
        let bundled = dir.path().join("courgette64.exe");
        std::fs::write(&bundled, []).unwrap();
        std::fs::write(&main, []).unwrap();

        assert_eq!(
            find_executable_named(&dir.path().to_string_lossy(), Some("Docker Desktop")),
            Some(main)
        );
    }

    #[test]
    fn merges_shortcut_and_executable_despite_publisher_mismatch() {
        let mut shortcut = app("Firefox", r"C:\Menu\Firefox.lnk");
        shortcut.launch_kind = LaunchKind::Shortcut;
        shortcut.publisher = Some("Mozilla".into());
        let mut executable = app("Firefox", r"D:\Apps\Firefox\firefox.exe");
        executable.source_kind = SourceKind::Portable;
        executable.publisher = Some("Mozilla Corporation".into());

        let merged = deduplicate(vec![executable, shortcut]);

        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].path, r"C:\Menu\Firefox.lnk");
        assert_eq!(merged[0].launch_kind, LaunchKind::Shortcut);
    }

    #[test]
    fn identifies_maintenance_and_resource_noise() {
        assert!(is_noise(
            "Удалить Ассистент",
            r"C:\Menu\Удалить Ассистент.lnk"
        ));
        assert!(is_noise(
            "Docker Desktop",
            r"C:\Docker\Docker Desktop Installer.exe"
        ));
        assert!(is_noise("Updater", r"C:\App\update.exe"));
        assert!(is_noise("Repair", r"C:\App\repair.exe"));
        assert!(is_noise("Icon", r"C:\App\app.ico"));
        assert!(!is_noise("Docker Desktop", r"C:\Docker\Docker Desktop.exe"));
    }

    #[test]
    fn identifies_maintenance_from_resolved_shortcut_target() {
        assert!(is_maintenance_entry(
            "Visual Studio Installer",
            r"C:\Menu\Visual Studio Installer.lnk",
            Some(r"C:\Program Files (x86)\Microsoft Visual Studio\Installer\setup.exe"),
        ));
    }

    #[test]
    fn merges_shortcut_and_executable_by_resolved_target() {
        let mut shortcut = app("Happ", r"C:\Menu\Happ.lnk");
        shortcut.launch_kind = LaunchKind::Shortcut;
        shortcut.resolved_path = Some(r"C:\Program Files\Happ\Happ.exe".into());
        let executable = app(
            "Happ - Proxy Utility 2.14.0(542)",
            r"C:\Program Files\Happ\Happ.exe",
        );
        let merged = deduplicate(vec![executable, shortcut]);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].path, r"C:\Menu\Happ.lnk");
    }

    #[test]
    fn merges_cached_happ_names_without_a_resolved_target() {
        let mut shortcut = app("Happ", r"C:\Menu\Happ.lnk");
        shortcut.launch_kind = LaunchKind::Shortcut;
        let mut executable = app(
            "Happ - Proxy Utility 2.14.0(542)",
            r"C:\Program Files\Happ\Happ.exe",
        );
        executable.publisher = Some("Happ".into());
        executable.icon_base64 = Some("data:image/png;base64,happ".into());
        let merged = deduplicate(vec![executable, shortcut]);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].launch_kind, LaunchKind::Shortcut);
        assert_eq!(merged[0].publisher.as_deref(), Some("Happ"));
        assert_eq!(
            merged[0].icon_base64.as_deref(),
            Some("data:image/png;base64,happ"),
        );
    }

    #[test]
    fn merges_cached_obs_architecture_suffix() {
        let mut shortcut = app("OBS Studio (64bit)", r"C:\Menu\OBS Studio.lnk");
        shortcut.launch_kind = LaunchKind::Shortcut;
        let mut executable = app("OBS Studio", r"D:\Apps\obs64.exe");
        executable.publisher = Some("OBS Project".into());
        let merged = deduplicate(vec![executable, shortcut]);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].path, r"C:\Menu\OBS Studio.lnk");
    }

    #[test]
    fn keeps_products_that_only_share_a_name_prefix() {
        assert_eq!(
            deduplicate(vec![
                app("Visual Studio", r"C:\Visual Studio\devenv.exe"),
                app("Visual Studio Code", r"C:\VS Code\Code.exe"),
            ])
            .len(),
            2,
        );
    }

    #[test]
    fn sanitizes_stale_maintenance_entries() {
        let apps = sanitize(vec![
            app(
                "Visual Studio Installer",
                "Microsoft.VisualStudio.Installer",
            ),
            app(
                "Installation notes",
                "file://C:/PostgreSQL/installation-notes.html",
            ),
            app(
                "Документация AIDA64 Extreme",
                r"C:\Menu\Документация AIDA64.lnk",
            ),
            app("AIDA64 Setup", r"C:\Apps\setup-app.exe"),
            app("Visual Studio Code", r"C:\Code.exe"),
        ]);
        assert_eq!(
            apps.iter().map(|app| app.name.as_str()).collect::<Vec<_>>(),
            vec!["Visual Studio Code"],
        );
    }

    #[test]
    fn merges_version_suffixed_duplicate_but_not_simple_numbered_names() {
        let merged = deduplicate(vec![
            app("CurseForge", r"C:\Menu\CurseForge.lnk"),
            app("CurseForge 1.302.1-33120", r"C:\Apps\CurseForge.exe"),
        ]);
        assert_eq!(merged.len(), 1);
        assert_eq!(
            deduplicate(vec![
                app("Editor 1", r"C:\Editor1.exe"),
                app("Editor 2", r"C:\Editor2.exe"),
            ])
            .len(),
            2,
        );
    }

    #[test]
    fn classifies_world_of_warcraft_as_games() {
        assert_eq!(
            classify("World of Warcraft", r"C:\Blizzard\Wow.lnk"),
            AppCategory::Games,
        );
    }

    #[test]
    fn classifies_known_application_categories() {
        assert_eq!(
            classify("Battle.net", r"C:\Games\Battle.net.exe"),
            AppCategory::Games
        );
        assert_eq!(classify("Claude", r"C:\Claude.exe"), AppCategory::Ai);
        assert_eq!(classify("Codex", r"C:\Codex.exe"), AppCategory::Ai);
        assert_eq!(
            classify("Adobe Photoshop", r"C:\Photoshop.exe"),
            AppCategory::Editors
        );
        assert_eq!(classify("Figma", r"C:\Figma.exe"), AppCategory::Editors);
        assert_eq!(
            classify("Visual Studio Code", r"C:\Code.exe"),
            AppCategory::Development
        );
        assert_eq!(
            classify("RustRover", r"C:\RustRover.exe"),
            AppCategory::Development
        );
        assert_eq!(
            classify("Google Chrome", r"C:\Chrome.exe"),
            AppCategory::Browsers
        );
        assert_eq!(
            classify("VLC media player", r"C:\vlc.exe"),
            AppCategory::Media
        );
        assert_eq!(
            classify("Discord", r"C:\Discord.exe"),
            AppCategory::Communication
        );
        assert_eq!(
            classify("7-Zip File Manager", r"C:\7zFM.exe"),
            AppCategory::Utilities
        );
        assert_eq!(
            classify("Character Map", r"C:\charmap.exe"),
            AppCategory::WindowsFeatures
        );
    }

    #[test]
    fn classifies_windows_management_tools_as_windows_features() {
        assert_eq!(
            classify("Computer Management", r"C:\Windows\System32\compmgmt.msc"),
            AppCategory::WindowsFeatures
        );
        assert_eq!(
            classify("Управление печатью", r"C:\Menu\Управление печатью.lnk"),
            AppCategory::WindowsFeatures
        );
        assert_eq!(
            classify(
                "Управление компьютером",
                r"C:\Menu\Управление компьютером.lnk"
            ),
            AppCategory::WindowsFeatures
        );
        assert_eq!(
            classify("Event Viewer", r"C:\Windows\System32\eventvwr.msc"),
            AppCategory::WindowsFeatures
        );
        for (name, path) in [
            ("Snipping Tool", r"C:\Windows\System32\SnippingTool.exe"),
            ("Ножницы", r"C:\Menu\Ножницы.lnk"),
            ("Task Scheduler", r"C:\Windows\System32\taskschd.msc"),
            ("Планировщик задач", r"C:\Menu\Планировщик задач.lnk"),
            ("Get Help", "Microsoft.GetHelp_8wekyb3d8bbwe!App"),
            (
                "Техническая поддержка",
                "Microsoft.GetHelp_8wekyb3d8bbwe!App",
            ),
            ("File Explorer", r"C:\Windows\explorer.exe"),
            ("Проводник", r"C:\Windows\explorer.exe"),
            (
                "Remote Desktop Connection",
                r"C:\Windows\System32\mstsc.exe",
            ),
            (
                "Подключение к удаленному рабочему столу",
                r"C:\Windows\System32\mstsc.exe",
            ),
            (
                "Инструменты Windows",
                "Microsoft.Windows.AdministrativeTools",
            ),
            (
                "Безопасность Windows",
                "Microsoft.SecHealthUI_8wekyb3d8bbwe!SecHealthUI",
            ),
            (
                "Средство проверки памяти Windows",
                r"C:\Windows\System32\MdSched.exe",
            ),
            (
                "Архивация Windows",
                "MicrosoftWindows.Client.CBS_cw5n1h2txyewy!WindowsBackup",
            ),
        ] {
            assert_eq!(
                classify(name, path),
                AppCategory::WindowsFeatures,
                "{name} should be a Windows feature"
            );
        }
    }

    #[test]
    fn microsoft_product_names_do_not_imply_windows_features() {
        assert_eq!(
            classify(
                "Microsoft Edge",
                r"C:\Program Files\Microsoft\Edge\msedge.exe"
            ),
            AppCategory::Browsers
        );
        assert_eq!(
            classify(
                "Microsoft Visual Studio",
                r"C:\Program Files\Microsoft Visual Studio\devenv.exe"
            ),
            AppCategory::Development
        );
        assert_ne!(
            classify(
                "Microsoft 365",
                "Microsoft.MicrosoftOfficeHub_8wekyb3d8bbwe!Microsoft.MicrosoftOfficeHub"
            ),
            AppCategory::WindowsFeatures
        );
    }

    #[test]
    fn classifies_unknown_app_as_other() {
        assert_eq!(
            classify("Acme Workspace", r"C:\Acme.exe"),
            AppCategory::Other
        );
    }

    #[test]
    fn stable_ids_ignore_windows_path_case() {
        assert_eq!(
            stable_id(r"C:\Apps\Codex.exe"),
            stable_id(r"c:\apps\CODEX.exe")
        );
    }

    #[test]
    fn expanded_app_model_preserves_metadata() {
        let mut value = app("Codex", r"C:\Apps\Codex.exe");
        value.version = Some("1.2.3".into());
        value.publisher = Some("OpenAI".into());
        value.description = Some("Coding agent".into());
        value.launch_kind = LaunchKind::Executable;
        value.source_kind = SourceKind::Registry;
        value.can_uninstall = true;
        assert_eq!(value.version.as_deref(), Some("1.2.3"));
        assert_eq!(value.publisher.as_deref(), Some("OpenAI"));
        assert!(value.can_uninstall);
    }

    #[test]
    fn reuses_cached_icon_by_stable_id() {
        let mut discovered = vec![app("Happ", r"C:\Menu\Happ.lnk")];
        discovered[0].id = "happ".into();
        let mut cached = discovered[0].clone();
        cached.icon_base64 = Some("data:image/png;base64,cached".into());
        reuse_cached_icons(&mut discovered, &[cached]);
        assert_eq!(
            discovered[0].icon_base64.as_deref(),
            Some("data:image/png;base64,cached")
        );
    }

    #[test]
    fn shortcut_icon_source_prefers_target_when_icon_location_is_empty() {
        let mut value = app("Happ", r"C:\Menu\Happ.lnk");
        value.launch_kind = LaunchKind::Shortcut;
        value.resolved_path = Some(r"C:\Program Files\Happ\Happ.exe".into());
        assert_eq!(
            icon_source(&value).as_deref(),
            Some(r"C:\Program Files\Happ\Happ.exe")
        );
    }

    #[test]
    fn prefers_executable_over_packaged_target_and_merges_metadata() {
        let mut registry = app("Codex", r"C:\Apps\Codex.exe");
        registry.publisher = Some("OpenAI".into());
        registry.version = Some("1.2.3".into());
        let mut packaged = app("Codex", "OpenAI.Codex_abc!App");
        packaged.launch_kind = LaunchKind::AppUserModelId;
        packaged.source_kind = SourceKind::StartApps;
        let apps = deduplicate(vec![registry, packaged]);
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].launch_kind, LaunchKind::Executable);
        assert_eq!(apps[0].publisher.as_deref(), Some("OpenAI"));
        assert_eq!(apps[0].version.as_deref(), Some("1.2.3"));
    }

    #[test]
    fn prefers_newer_portable_version_when_duplicate_copies_exist() {
        let mut old = app("Rufus", r"E:\Tools\rufus-3.11p.exe");
        old.source_kind = SourceKind::Portable;
        old.version = Some("3.11.0".into());
        let mut current = app("Rufus", r"D:\Tools\rufus-4.11p.exe");
        current.source_kind = SourceKind::Portable;
        current.version = Some("4.11.2285".into());

        let merged = deduplicate(vec![old, current]);

        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].path, r"D:\Tools\rufus-4.11p.exe");
    }

    #[test]
    fn keeps_same_names_with_conflicting_publishers() {
        let mut first = app("Studio", r"C:\Alpha\Studio.exe");
        first.publisher = Some("Alpha".into());
        let mut second = app("Studio", r"C:\Beta\Studio.exe");
        second.publisher = Some("Beta".into());
        assert_eq!(deduplicate(vec![first, second]).len(), 2);
    }

    #[test]
    fn merges_packaged_and_desktop_entries_with_the_same_name() {
        let mut desktop = app("AMD Software", r"C:\AMD\AMDSoftware.exe");
        desktop.publisher = Some("Advanced Micro Devices, Inc.".into());
        let mut packaged = app("AMD Software", "AMD.Package!App");
        packaged.publisher = Some("CN=AMD".into());
        packaged.launch_kind = LaunchKind::AppUserModelId;
        let merged = deduplicate(vec![desktop, packaged]);
        assert_eq!(merged.len(), 1);
        assert_eq!(
            merged[0].publisher.as_deref(),
            Some("Advanced Micro Devices, Inc.")
        );
    }
}
