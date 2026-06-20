use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::env;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};

mod registry;
mod start_apps;

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
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UninstallTarget {
    Command { executable: String, arguments: String },
    Msix { package_full_name: String },
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

pub fn discover_apps() -> Vec<AppInfo> {
    let mut apps = Vec::new();
    apps.extend(registry::scan(
        HKEY_LOCAL_MACHINE,
        r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
    ));
    apps.extend(registry::scan(
        HKEY_LOCAL_MACHINE,
        r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
    ));
    apps.extend(registry::scan(
        HKEY_CURRENT_USER,
        r"Software\Microsoft\Windows\CurrentVersion\Uninstall",
    ));
    apps.extend(scan_start_menu());
    apps.extend(start_apps::scan());
    enrich_local_metadata(&mut apps);
    sanitize(apps)
}

pub fn sanitize(apps: Vec<AppInfo>) -> Vec<AppInfo> {
    deduplicate(
        apps.into_iter()
            .filter(|app| {
                !is_maintenance_entry(
                    &app.name,
                    &app.path,
                    app.resolved_path.as_deref(),
                )
            })
            .collect(),
    )
}

fn enrich_local_metadata(apps: &mut [AppInfo]) {
    for app in apps {
        let target = app
            .resolved_path
            .as_deref()
            .unwrap_or(&app.path);
        let path = Path::new(target);
        if !path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
            || !path.is_file()
        {
            continue;
        }
        let metadata = crate::executable_metadata::read(path);
        crate::executable_metadata::fill_missing(
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
        .or_else(|| app.resolved_path.clone().filter(|path| Path::new(path).is_file()))
        .or_else(|| (app.launch_kind != LaunchKind::AppUserModelId).then(|| app.path.clone()))
}

pub fn hydrate_missing_icons(apps: &mut [AppInfo]) {
    for app in apps.iter_mut().filter(|app| app.icon_base64.is_none()) {
        app.icon_base64 = if app.launch_kind == LaunchKind::AppUserModelId {
            crate::icon_extractor::extract_app_id_icon(&app.path)
        } else {
            icon_source(app).and_then(|path| crate::icon_extractor::extract_icon(Path::new(&path)))
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
            let details = crate::shortcut::resolve(&path);
            let target = details.target.as_ref().map(|value| value.to_string_lossy().into_owned());
            (!name.is_empty() && !is_maintenance_entry(&name, &path.to_string_lossy(), target.as_deref()))
                .then(|| {
                    let mut app = make_app(name, path);
                    app.source_kind = SourceKind::StartMenu;
                    app.resolved_path = target;
                    app.shortcut_icon_path = details.icon_location.map(|value| value.to_string_lossy().into_owned());
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
    let launch_kind = if Path::new(&path).extension().is_some_and(|extension| extension.eq_ignore_ascii_case("lnk")) {
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
    format!("{:x}", Sha256::digest(identity.trim().to_lowercase().as_bytes()))
}

fn classify(name: &str, path: &str) -> AppCategory {
    let value = format!("{name} {path}").to_lowercase();
    let contains = |keywords: &[&str]| keywords.iter().any(|keyword| value.contains(keyword));

    if contains(&["steam", "battle.net", "epic games", "gog", "game", "minecraft", "roblox", "backpack battles", "warcraft"]) {
        AppCategory::Games
    } else if contains(&["claude", "chatgpt", "openai", "codex", "ollama", "lm studio", "gemini", "copilot", "cursor", "ai agent"] ) {
        AppCategory::Ai
    } else if contains(&["visual studio", "vscode", "code.exe", "rustrover", "pycharm", "webstorm", "intellij", "android studio", "git", "docker", "postman", "terminal"] ) {
        AppCategory::Development
    } else if contains(&["photoshop", "illustrator", "lightroom", "figma", "gimp", "inkscape", "blender", "paint", "canva"] ) {
        AppCategory::Editors
    } else if contains(&["chrome", "firefox", "edge", "opera", "brave", "vivaldi", "browser"] ) {
        AppCategory::Browsers
    } else if contains(&["vlc", "spotify", "foobar", "audacity", "obs", "media player", "music", "video"] ) {
        AppCategory::Media
    } else if contains(&["discord", "telegram", "whatsapp", "slack", "teams", "zoom", "skype", "signal"] ) {
        AppCategory::Communication
    } else if contains(&["character map", "command prompt", "powershell", "control panel", "computer management", "component services", "administrative tools", "registry editor", "task manager"] ) {
        AppCategory::System
    } else if contains(&["7-zip", "winrar", "everything", "powertoys", "verifier", "configure", "utility", "uninstaller"] ) {
        AppCategory::Utilities
    } else {
        AppCategory::Other
    }
}

fn find_executable(location: &str) -> Option<PathBuf> {
    let root = PathBuf::from(location.trim().trim_matches('"'));
    if is_launchable(&root) {
        return Some(root);
    }
    if !root.is_dir() {
        return None;
    }
    WalkDir::new(root)
        .max_depth(2)
        .into_iter()
        .filter_map(Result::ok)
        .map(|entry| entry.into_path())
        .find(|path| is_launchable(path) && !is_noise("", &path.to_string_lossy()))
}

fn is_launchable(path: &Path) -> bool {
    path.is_file()
        && path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("exe") || extension.eq_ignore_ascii_case("lnk"))
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
    name.is_empty()
        || name.to_lowercase().starts_with("ms-resource:")
        || name.contains('\u{fffd}')
}

fn is_noise(name: &str, path: &str) -> bool {
    is_maintenance_entry(name, path, None)
}

fn is_maintenance_entry(name: &str, path: &str, resolved_path: Option<&str>) -> bool {
    if is_invalid_display_name(name) {
        return true;
    }
    if Path::new(path).extension().is_some_and(|extension| {
        ["ico", "dll", "mui", "cpl"]
            .iter()
            .any(|value| extension.eq_ignore_ascii_case(value))
    }) {
        return true;
    }
    let value = format!("{name} {path} {}", resolved_path.unwrap_or_default()).to_lowercase();
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
        "kb[",
        "file://",
    ]
    .iter()
    .any(|needle| value.contains(needle))
}

fn deduplicate(apps: Vec<AppInfo>) -> Vec<AppInfo> {
    let mut unique = Vec::<AppInfo>::new();
    for app in apps {
        if let Some(index) = unique.iter().position(|existing| same_application(existing, &app)) {
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
    if left.path.eq_ignore_ascii_case(&right.path) { return true }
    let left_identity = left.resolved_path.as_deref().unwrap_or(&left.path);
    let right_identity = right.resolved_path.as_deref().unwrap_or(&right.path);
    if left_identity.eq_ignore_ascii_case(right_identity) { return true }
    let left_name = normalized_product_family(&left.name);
    let right_name = normalized_product_family(&right.name);
    if left_name != right_name { return false }
    if left.launch_kind == LaunchKind::AppUserModelId || right.launch_kind == LaunchKind::AppUserModelId { return true }
    match (&left.publisher, &right.publisher) {
        (Some(left), Some(right)) => left.eq_ignore_ascii_case(right),
        _ => true,
    }
}

fn merge_app(left: AppInfo, right: AppInfo) -> AppInfo {
    let (mut primary, secondary) = if candidate_score(&right) > candidate_score(&left) { (right, left) } else { (left, right) };
    if primary.description.is_none() { primary.description = secondary.description; }
    if primary.version.is_none() { primary.version = secondary.version; }
    if primary.publisher.is_none() || primary.publisher.as_deref().is_some_and(|value| value.starts_with("CN=")) {
        if secondary.publisher.as_deref().is_some_and(|value| !value.starts_with("CN=")) { primary.publisher = secondary.publisher; }
    }
    if primary.install_location.is_none() { primary.install_location = secondary.install_location; }
    if primary.icon_base64.is_none() { primary.icon_base64 = secondary.icon_base64; }
    if primary.uninstall.is_none() { primary.uninstall = secondary.uninstall; }
    primary.can_uninstall |= secondary.can_uninstall || primary.uninstall.is_some();
    primary
}

fn normalize_name(name: &str) -> String {
    name.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()
}

fn normalized_product_family(name: &str) -> String {
    let mut value = normalize_name(name);
    for marker in [" (64bit)", " (32bit)", " (64-bit)", " (32-bit)", " x64", " x86"] {
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
    version_family(&value).trim().to_string()
}

fn version_family(name: &str) -> &str {
    let Some((family, suffix)) = name.rsplit_once(' ') else { return name };
    if !suffix.starts_with(|character: char| character.is_ascii_digit()) {
        return name;
    }
    let numeric_segments = suffix
        .split(|character: char| !character.is_ascii_digit())
        .filter(|segment| !segment.is_empty())
        .count();
    if numeric_segments >= 2 { family } else { name }
}

fn candidate_score(app: &AppInfo) -> u8 {
    match Path::new(&app.path).extension().and_then(|value| value.to_str()) {
        Some(extension) if extension.eq_ignore_ascii_case("lnk") => return 4,
        Some(extension) if extension.eq_ignore_ascii_case("exe") => return 3,
        _ => {}
    }
    if app.launch_kind == LaunchKind::AppUserModelId { 2 } else { 0 }
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
        AppCategory::Other => 9,
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
    fn identifies_uninstaller_noise() {
        assert!(is_noise("Microsoft Visual C++ Update", r"C:\update.exe"));
        assert!(is_noise("Editor Uninstall", r"C:\uninstall.exe"));
        assert!(!is_noise("Visual Studio Code", r"C:\Code.exe"));
    }

    #[test]
    fn identifies_maintenance_and_resource_noise() {
        assert!(is_noise("Удалить Ассистент", r"C:\Menu\Удалить Ассистент.lnk"));
        assert!(is_noise("Docker Desktop", r"C:\Docker\Docker Desktop Installer.exe"));
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
            app("Visual Studio Installer", "Microsoft.VisualStudio.Installer"),
            app("Installation notes", "file://C:/PostgreSQL/installation-notes.html"),
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
            ]).len(),
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
        assert_eq!(classify("Battle.net", r"C:\Games\Battle.net.exe"), AppCategory::Games);
        assert_eq!(classify("Claude", r"C:\Claude.exe"), AppCategory::Ai);
        assert_eq!(classify("Codex", r"C:\Codex.exe"), AppCategory::Ai);
        assert_eq!(classify("Adobe Photoshop", r"C:\Photoshop.exe"), AppCategory::Editors);
        assert_eq!(classify("Figma", r"C:\Figma.exe"), AppCategory::Editors);
        assert_eq!(classify("Visual Studio Code", r"C:\Code.exe"), AppCategory::Development);
        assert_eq!(classify("RustRover", r"C:\RustRover.exe"), AppCategory::Development);
        assert_eq!(classify("Google Chrome", r"C:\Chrome.exe"), AppCategory::Browsers);
        assert_eq!(classify("VLC media player", r"C:\vlc.exe"), AppCategory::Media);
        assert_eq!(classify("Discord", r"C:\Discord.exe"), AppCategory::Communication);
        assert_eq!(classify("7-Zip File Manager", r"C:\7zFM.exe"), AppCategory::Utilities);
        assert_eq!(classify("Character Map", r"C:\charmap.exe"), AppCategory::System);
    }

    #[test]
    fn classifies_unknown_app_as_other() {
        assert_eq!(classify("Acme Workspace", r"C:\Acme.exe"), AppCategory::Other);
    }

    #[test]
    fn stable_ids_ignore_windows_path_case() {
        assert_eq!(stable_id(r"C:\Apps\Codex.exe"), stable_id(r"c:\apps\CODEX.exe"));
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
        assert_eq!(discovered[0].icon_base64.as_deref(), Some("data:image/png;base64,cached"));
    }

    #[test]
    fn shortcut_icon_source_prefers_target_when_icon_location_is_empty() {
        let mut value = app("Happ", r"C:\Menu\Happ.lnk");
        value.launch_kind = LaunchKind::Shortcut;
        value.resolved_path = Some(r"C:\Program Files\Happ\Happ.exe".into());
        assert_eq!(icon_source(&value).as_deref(), Some(r"C:\Program Files\Happ\Happ.exe"));
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
        assert_eq!(merged[0].publisher.as_deref(), Some("Advanced Micro Devices, Inc."));
    }
}
