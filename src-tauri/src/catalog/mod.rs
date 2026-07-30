use sha2::{Digest, Sha256};
use std::env;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

mod classify;
mod dedup;
mod filters;
mod model;
mod naming;
mod scan;
mod sources;
mod storage;
pub(crate) mod sync;
mod visibility;

pub(crate) use model::{
    AppCategory, AppInfo, LaunchKind, ScanProgress, SourceKind, UninstallTarget,
};
pub(crate) use scan::{
    coordinator as scan_coordinator, hydration, incremental, settings as scan_settings,
};
pub(crate) use sources::source;
use sources::{portable, registry, start_apps, steam};
pub(crate) use storage::{cache, icon_cache};
pub(crate) use visibility::{VisibilityClass, VisibilityReason};

fn steam_app(game: steam::SteamGame) -> AppInfo {
    let path = format!("steam://rungameid/{}", game.app_id);
    let product_name = game.name.clone();
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
        product_name: Some(product_name),
        original_filename: None,
        install_location: Some(game.install_dir.to_string_lossy().into_owned()),
        can_uninstall: false,
        uninstall: None,
        resolved_path: find_executable(&game.install_dir.to_string_lossy())
            .map(|path| path.to_string_lossy().into_owned()),
        shortcut_icon_path: None,
        launch_arguments: None,
        canonical_identity: None,
        preference_identity: None,
        visibility_class: Default::default(),
        visibility_score: 0,
        visibility_reasons: Vec::new(),
    }
}

pub(super) fn portable_app(path: PathBuf) -> Option<AppInfo> {
    let metadata = crate::platform::windows::executable_metadata::read(&path);
    let has_metadata = metadata.product_name.is_some()
        || metadata.description.is_some()
        || metadata.publisher.is_some()
        || metadata.original_filename.is_some();
    let stem = path.file_stem()?.to_string_lossy().trim().to_string();
    let parent_matches = path
        .parent()
        .and_then(Path::file_name)
        .is_some_and(|parent| {
            naming::normalized_portable_name(&parent.to_string_lossy())
                == naming::normalized_portable_name(&stem)
        });
    if !has_metadata && !parent_matches && !is_known_standalone_portable(&stem) {
        return None;
    }
    let parent_name = path
        .parent()
        .and_then(Path::file_name)
        .map(|parent| parent.to_string_lossy().into_owned());
    let name = naming::portable_display_name(
        &stem,
        parent_name.as_deref(),
        metadata.product_name.as_deref(),
    );
    if filters::is_maintenance_entry(&name, &path.to_string_lossy(), None) {
        return None;
    }
    let mut app = make_app(name, path.clone());
    app.source_kind = SourceKind::Portable;
    app.description = metadata.description;
    app.version = metadata
        .version
        .or_else(|| naming::portable_version_from_stem(&stem));
    app.publisher = metadata.publisher;
    app.product_name = metadata.product_name;
    app.original_filename = metadata.original_filename;
    app.install_location = path
        .parent()
        .map(|value| value.to_string_lossy().into_owned());
    Some(app)
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

pub(crate) fn watcher_paths(settings: &scan_settings::ScanSettings) -> Vec<PathBuf> {
    let mut paths = vec![PathBuf::from(
        r"C:\ProgramData\Microsoft\Windows\Start Menu\Programs",
    )];
    if let Some(appdata) = env::var_os("APPDATA") {
        paths.push(PathBuf::from(appdata).join(r"Microsoft\Windows\Start Menu\Programs"));
    }
    paths.extend(settings.included_paths.iter().map(PathBuf::from));
    // Intentionally NOT watching Steam `steamapps`: Steam writes there constantly
    // (shadercache, manifests, logs), which triggered a full resync every few seconds.
    // Newly installed Steam games are picked up on manual Refresh / next startup.
    paths.retain(|path| path.is_dir());
    paths.sort_by_cached_key(|path| path.to_string_lossy().to_lowercase());
    paths.dedup_by(|left, right| {
        left.to_string_lossy()
            .eq_ignore_ascii_case(&right.to_string_lossy())
    });
    paths
}

fn scan_registry() -> (Vec<AppInfo>, Vec<registry::RegistryMetadata>) {
    let scan = registry::scan();
    (scan.apps, scan.metadata)
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
    if dedup::normalized_product_family(&app.name) != dedup::normalized_product_family(&record.name)
    {
        return false;
    }
    match (&app.publisher, &record.publisher) {
        (Some(app_publisher), Some(record_publisher)) => {
            app_publisher.eq_ignore_ascii_case(record_publisher)
        }
        _ => true,
    }
}

fn filter_maintenance(apps: Vec<AppInfo>) -> Vec<AppInfo> {
    let classified = apps
        .into_iter()
        .filter(|app| !filters::is_invalid_display_name(&app.name))
        .map(|mut app| {
            visibility::apply_visibility(&mut app);
            app
        })
        .collect::<Vec<_>>();
    visibility::write_dev_report(&classified);
    classified
        .into_iter()
        .filter(|app| app.visibility_class != VisibilityClass::Rejected)
        .collect()
}

pub(crate) fn sanitize(apps: Vec<AppInfo>) -> Vec<AppInfo> {
    dedup::deduplicate(
        filter_maintenance(apps),
        classify::classify_app,
        crate::platform::windows::os_ui_script(),
    )
}

/// Like `sanitize`, but also refreshes the dev-only dedup report. Call this from the full
/// catalog assembly (`sync::synchronize`, after registry metadata is attached) so the report
/// reflects every app — never a partial sub-list — and is overwritten once per scan (no
/// accumulation).
pub(crate) fn sanitize_reported(apps: Vec<AppInfo>) -> Vec<AppInfo> {
    let filtered = filter_maintenance(apps);
    if dedup::dev_report_enabled() {
        dedup::write_dev_report(&filtered);
    }
    dedup::deduplicate(
        filtered,
        classify::classify_app,
        crate::platform::windows::os_ui_script(),
    )
}

fn is_known_standalone_portable(stem: &str) -> bool {
    let normalized = naming::normalized_portable_name(stem);
    [
        "rufus",
        "putty",
        "winscp",
        "ventoy",
        "crystaldiskinfo",
        "crystaldiskmark",
        "cpu z",
        "gpu z",
        "hwinfo",
        "memtest",
        "processhacker",
        "processexplorer",
    ]
    .iter()
    .any(|known| normalized == naming::normalized_portable_name(known))
        || normalized.starts_with("rufus")
        || normalized.starts_with("putty")
        || normalized.starts_with("winscp")
        || normalized.starts_with("ventoy")
}

/// Ordered icon-source candidates: the shortcut's declared icon file first, then the
/// resolved launch target, then the catalog path itself. Hydration walks the list until
/// one source actually yields an icon (a single source is not enough — e.g. PostgreSQL
/// shortcuts point at an `.ico` the shell can't rasterize, while the target exe can).
pub(super) fn icon_source_candidates(app: &AppInfo) -> Vec<String> {
    let mut candidates: Vec<String> = Vec::new();
    let mut push = |value: Option<&String>| {
        if let Some(path) = value {
            if Path::new(path).is_file() && !candidates.contains(path) {
                candidates.push(path.clone());
            }
        }
    };
    push(app.shortcut_icon_path.as_ref());
    push(app.resolved_path.as_ref());
    if app.launch_kind != LaunchKind::AppUserModelId && !candidates.contains(&app.path) {
        candidates.push(app.path.clone());
    }
    candidates
}

#[cfg(test)]
pub(super) fn icon_source(app: &AppInfo) -> Option<String> {
    icon_source_candidates(app).into_iter().next()
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
                && !filters::is_maintenance_entry(
                    &name,
                    &path.to_string_lossy(),
                    target.as_deref(),
                ))
            .then(|| {
                let mut app = make_app(name, path);
                app.source_kind = SourceKind::StartMenu;
                app.resolved_path = target;
                app.shortcut_icon_path = details
                    .icon_location
                    .map(|value| value.to_string_lossy().into_owned());
                app.launch_arguments = details.arguments;
                if let Some(target) = app.resolved_path.as_deref() {
                    let metadata =
                        crate::platform::windows::executable_metadata::read(Path::new(target));
                    app.product_name = metadata.product_name;
                    app.original_filename = metadata.original_filename;
                    app.description = metadata.description;
                    app.version = metadata.version;
                    app.publisher = metadata.publisher;
                }
                app
            })
        })
        .collect()
}

fn make_app(name: String, path: PathBuf) -> AppInfo {
    let path = path.to_string_lossy().to_string();
    let normalized = path.to_lowercase();
    let id = format!("{:x}", Sha256::digest(normalized.as_bytes()));
    let category = classify::classify(&name, &path);
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
        product_name: None,
        original_filename: None,
        install_location: None,
        can_uninstall: false,
        uninstall: None,
        resolved_path: None,
        shortcut_icon_path: None,
        launch_arguments: None,
        canonical_identity: None,
        preference_identity: None,
        visibility_class: Default::default(),
        visibility_score: 0,
        visibility_reasons: Vec::new(),
    }
}

fn stable_id(identity: &str) -> String {
    format!(
        "{:x}",
        Sha256::digest(identity.trim().to_lowercase().as_bytes())
    )
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
        .map(naming::normalized_portable_name)
        .filter(|key| !key.is_empty());
    WalkDir::new(&root)
        .max_depth(2)
        .into_iter()
        .filter_map(Result::ok)
        .map(|entry| entry.into_path())
        .filter(|path| {
            is_launchable(path) && !filters::is_maintenance_path(&path.to_string_lossy())
        })
        .min_by_key(|path| {
            let stem = path
                .file_stem()
                .map(|value| naming::normalized_portable_name(&value.to_string_lossy()))
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

/// Directory containment for already-lowercased paths, stopping at a component boundary.
///
/// A plain `starts_with` also matches partial component names, which silently changes meaning
/// in both directions: an excluded `d:\games` would also swallow `d:\gamesbackup`, and an
/// install root of `c:\prog` would "contain" `c:\program files\other.exe`. Both separators are
/// accepted because scan settings are typed by hand.
pub(super) fn path_is_within(path: &str, root: &str) -> bool {
    let root = root.trim_end_matches(['\\', '/']);
    if root.is_empty() {
        return false;
    }
    let Some(rest) = path.strip_prefix(root) else {
        return false;
    };
    rest.is_empty() || rest.starts_with('\\') || rest.starts_with('/')
}

fn is_launchable(path: &Path) -> bool {
    path.is_file()
        && path.extension().is_some_and(|extension| {
            extension.eq_ignore_ascii_case("exe") || extension.eq_ignore_ascii_case("lnk")
        })
}

/// Whether an app's launch target still exists. A Start-Menu shortcut or registry entry left
/// behind by an uninstalled application points at a file that is gone; such phantoms should not
/// appear. Non-file targets — Store AUMIDs, `steam://` URIs, UNC paths — always pass. A target on
/// a drive that is not currently mounted is kept, so unplugging a removable or second disk does
/// not erase its apps until it is genuinely known to be gone.
pub(crate) fn target_is_present(app: &AppInfo) -> bool {
    if app.launch_kind == LaunchKind::AppUserModelId {
        return true;
    }
    let target = app.resolved_path.as_deref().unwrap_or(&app.path).trim();
    if target.is_empty() || target.contains("://") || target.starts_with(r"\\") {
        return true;
    }
    let path = Path::new(target);
    if !path.is_absolute() {
        return true;
    }
    let drive_present = path.ancestors().last().is_some_and(|root| root.exists());
    !drive_present || path.exists()
}

/// Move console (CLI) executables — a "7-Zip Console", `git.exe` — out of the primary catalog into
/// Auxiliary. The PE subsystem is a reliable, launch-free signal that generalizes to any
/// executable (including ones nobody can test by hand), so it beats name/description heuristics.
/// Runs on the scan path only (it reads the target file), after classification.
fn is_plain_windows_powershell(app: &AppInfo, target: &Path) -> bool {
    if app.source_kind != SourceKind::StartMenu
        || app.launch_kind != LaunchKind::Shortcut
        || app
            .launch_arguments
            .as_deref()
            .is_some_and(|arguments| !arguments.trim().is_empty())
    {
        return false;
    }
    target
        .file_name()
        .and_then(|file| file.to_str())
        .is_some_and(|file| file.eq_ignore_ascii_case("powershell.exe"))
}

pub(crate) fn demote_console_applications(apps: &mut [AppInfo]) {
    for app in apps.iter_mut() {
        if app.visibility_class != VisibilityClass::Primary
            || app.launch_kind == LaunchKind::AppUserModelId
        {
            continue;
        }
        let target = app.resolved_path.as_deref().unwrap_or(&app.path);
        let path = Path::new(target);
        if is_plain_windows_powershell(app, path) {
            continue;
        }
        if path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
            && crate::platform::windows::executable_metadata::is_console_subsystem(path)
        {
            app.visibility_class = VisibilityClass::Auxiliary;
            if !app
                .visibility_reasons
                .contains(&VisibilityReason::ConsoleApplication)
            {
                app.visibility_reasons
                    .push(VisibilityReason::ConsoleApplication);
            }
        }
    }
}

#[cfg(test)]
fn deduplicate(apps: Vec<AppInfo>) -> Vec<AppInfo> {
    // Tests assert the English-user experience by default; the locale-aware name pick keeps a Latin
    // name over a Cyrillic one. Cases that need the Cyrillic result set the script explicitly.
    dedup::deduplicate(
        apps,
        classify::classify_app,
        crate::platform::windows::NameScript::Latin,
    )
}

#[cfg(test)]
mod tests {
    use super::classify::{classify, classify_app};
    use super::filters::*;
    use super::naming::*;
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
            product_name: None,
            original_filename: None,
            install_location: None,
            can_uninstall: false,
            uninstall: None,
            resolved_path: None,
            shortcut_icon_path: None,
            launch_arguments: None,
            canonical_identity: None,
            preference_identity: None,
            visibility_class: Default::default(),
            visibility_score: 0,
            visibility_reasons: Vec::new(),
        }
    }

    #[test]
    fn portable_generic_exe_name_uses_parent_folder() {
        // 32.exe inside "Крипто 4" reports Yandex metadata — trust the folder instead.
        assert_eq!(
            portable_display_name("32", Some("Крипто 4"), Some("Yandex")),
            "Крипто 4"
        );
        assert_eq!(
            portable_display_name("64", Some("Крипто 5"), Some("Yandex")),
            "Крипто 5"
        );
    }

    #[test]
    fn portable_real_exe_name_keeps_product_name() {
        // A properly named executable keeps its product metadata.
        assert_eq!(
            portable_display_name("Yandex 32bit", Some("Браузер"), Some("Yandex")),
            "Yandex"
        );
        // Generic stem AND generic (arch) folder → fall back to metadata product name.
        assert_eq!(
            portable_display_name("app", Some("x64"), Some("Yandex")),
            "Yandex"
        );
    }

    #[test]
    fn includes_known_standalone_portable_without_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Tools").join("rufus-4.11p.exe");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, []).unwrap();

        let app = portable_app(path.clone()).expect("rufus should be detected");

        assert_eq!(app.name, "rufus");
        assert_eq!(app.path, path.to_string_lossy());
        assert_eq!(app.source_kind, SourceKind::Portable);
    }

    // Shared by scan exclusions and by registry-install containment in deduplication, so its
    // exact boundary behaviour is what keeps both from matching partial component names.
    #[test]
    fn path_containment_stops_at_component_boundaries() {
        assert!(path_is_within(r"c:\prog\app.exe", r"c:\prog"));
        assert!(path_is_within(r"c:\prog\app.exe", r"c:\prog\"));
        assert!(path_is_within(r"c:\prog", r"c:\prog"));
        assert!(path_is_within("c:/prog/app.exe", "c:/prog"));
        assert!(!path_is_within(r"c:\program files\app.exe", r"c:\prog"));
        assert!(!path_is_within(r"c:\progbackup", r"c:\prog"));
        assert!(!path_is_within(r"c:\prog\app.exe", ""));
        assert!(!path_is_within(r"c:\prog\app.exe", r"\"));
    }

    #[test]
    fn phantom_entries_whose_target_is_gone_are_dropped() {
        let dir = tempfile::tempdir().unwrap();
        let real = dir.path().join("app.exe");
        std::fs::write(&real, []).unwrap();

        // A shortcut resolving to an existing exe is kept; one resolving to a deleted exe is not.
        let mut present = app("App", &dir.path().join("App.lnk").to_string_lossy());
        present.launch_kind = LaunchKind::Shortcut;
        present.resolved_path = Some(real.to_string_lossy().into_owned());
        assert!(target_is_present(&present));

        let mut gone = app(
            "uTorrent",
            &dir.path().join("uTorrent.lnk").to_string_lossy(),
        );
        gone.launch_kind = LaunchKind::Shortcut;
        gone.resolved_path = Some(
            dir.path()
                .join("uTorrent.exe")
                .to_string_lossy()
                .into_owned(),
        );
        assert!(!target_is_present(&gone));

        // Non-file targets are never dropped.
        let mut store = app("Store App", "Some.Store_App!App");
        store.launch_kind = LaunchKind::AppUserModelId;
        assert!(target_is_present(&store));
        let mut steam = app("Game", "steam://rungameid/12345");
        steam.launch_kind = LaunchKind::Executable;
        assert!(target_is_present(&steam));
    }

    // Minimal PE with a chosen Subsystem (3 = console, 2 = GUI).
    fn minimal_pe(subsystem: u16) -> Vec<u8> {
        let mut buffer = vec![0_u8; 64];
        buffer[0] = b'M';
        buffer[1] = b'Z';
        buffer[60..64].copy_from_slice(&64_u32.to_le_bytes());
        buffer.extend_from_slice(b"PE\0\0");
        buffer.extend_from_slice(&[0_u8; 20]);
        buffer.extend_from_slice(&[0_u8; 68]);
        buffer.extend_from_slice(&subsystem.to_le_bytes());
        buffer
    }

    #[test]
    fn console_executables_are_demoted_to_auxiliary() {
        let dir = tempfile::tempdir().unwrap();
        let cli = dir.path().join("7z.exe");
        std::fs::write(&cli, minimal_pe(3)).unwrap();
        let gui = dir.path().join("game.exe");
        std::fs::write(&gui, minimal_pe(2)).unwrap();

        let mut console = app("7-Zip Console", &cli.to_string_lossy());
        console.source_kind = SourceKind::Portable;
        let mut window = app("Game", &gui.to_string_lossy());
        window.source_kind = SourceKind::Portable;

        let mut apps = vec![console, window];
        demote_console_applications(&mut apps);

        assert_eq!(apps[0].visibility_class, VisibilityClass::Auxiliary);
        assert!(apps[0]
            .visibility_reasons
            .contains(&VisibilityReason::ConsoleApplication));
        assert_eq!(apps[1].visibility_class, VisibilityClass::Primary);
    }

    #[test]
    fn command_prompt_is_auxiliary_but_plain_powershell_stays_primary() {
        let dir = tempfile::tempdir().unwrap();
        let cmd = dir.path().join("cmd.exe");
        let powershell = dir.path().join("powershell.exe");
        std::fs::write(&cmd, minimal_pe(3)).unwrap();
        std::fs::write(&powershell, minimal_pe(3)).unwrap();

        let mut command_prompt = app("Command Prompt", r"C:\Menu\Command Prompt.lnk");
        command_prompt.source_kind = SourceKind::StartMenu;
        command_prompt.launch_kind = LaunchKind::Shortcut;
        command_prompt.resolved_path = Some(cmd.to_string_lossy().into_owned());
        let mut windows_powershell = app("Windows PowerShell", r"C:\Menu\Windows PowerShell.lnk");
        windows_powershell.source_kind = SourceKind::StartMenu;
        windows_powershell.launch_kind = LaunchKind::Shortcut;
        windows_powershell.resolved_path = Some(powershell.to_string_lossy().into_owned());
        let mut developer_prompt = app("Developer Command Prompt", r"C:\Menu\Developer Prompt.lnk");
        developer_prompt.source_kind = SourceKind::StartMenu;
        developer_prompt.launch_kind = LaunchKind::Shortcut;
        developer_prompt.resolved_path = Some(cmd.to_string_lossy().into_owned());
        developer_prompt.launch_arguments = Some("/k setup.bat".into());

        let mut apps = vec![command_prompt, windows_powershell, developer_prompt];
        demote_console_applications(&mut apps);

        assert_eq!(apps[0].visibility_class, VisibilityClass::Auxiliary);
        assert!(apps[0]
            .visibility_reasons
            .contains(&VisibilityReason::ConsoleApplication));
        assert_eq!(apps[1].visibility_class, VisibilityClass::Primary);
        assert_eq!(apps[2].visibility_class, VisibilityClass::Auxiliary);
        assert!(apps[2]
            .visibility_reasons
            .contains(&VisibilityReason::ConsoleApplication));
    }

    #[test]
    fn sanitize_keeps_auxiliary_entries_but_rejects_installers() {
        let mut helper = app("iconv", r"C:\Git\usr\bin\iconv.exe");
        helper.source_kind = SourceKind::Portable;
        let mut installer = app("Telegram Desktop Setup", r"C:\Downloads\tsetup.exe");
        installer.source_kind = SourceKind::Portable;

        let result = sanitize(vec![helper, installer]);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "iconv");
        assert_eq!(result[0].visibility_class, VisibilityClass::Auxiliary);
    }

    #[test]
    fn rejects_unknown_orphan_executable_without_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Tools").join("helper-tool.exe");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, []).unwrap();

        assert!(portable_app(path).is_none());
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
        assert!(is_installer_file_name("tsetup-x64.7.3.4"));
        assert!(!is_installer_file_name("7zFM"));
        assert!(!is_installer_file_name("notepad"));
        assert!(!is_installer_file_name("setupbox"));
        assert!(!is_installer_file_name("aida64"));
    }

    #[test]
    fn extracts_portable_version_from_file_name() {
        assert_eq!(
            portable_version_from_stem("rufus-4.11p").as_deref(),
            Some("4.11p")
        );
        assert_eq!(
            portable_version_from_stem("tool_3.11").as_deref(),
            Some("3.11")
        );
        assert_eq!(portable_version_from_stem("notepad"), None);
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
    fn merges_side_by_side_shortcut_and_executable_with_same_product_name() {
        let mut shortcut = app("Battle.net", r"D:\Battle.net\Battle.net.lnk");
        shortcut.launch_kind = LaunchKind::Shortcut;
        shortcut.source_kind = SourceKind::StartMenu;
        let mut executable = app("Battle.net", r"D:\Battle.net\Battle.net.exe");
        executable.source_kind = SourceKind::Portable;

        let merged = deduplicate(vec![executable, shortcut]);

        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].path, r"D:\Battle.net\Battle.net.lnk");
    }

    #[test]
    fn merges_tableplus_shortcut_and_executable_in_same_product_folder() {
        let mut shortcut = app("TablePlus", r"D:\Tools\TablePlus\TablePlus.lnk");
        shortcut.launch_kind = LaunchKind::Shortcut;
        shortcut.source_kind = SourceKind::StartMenu;
        let mut executable = app("TablePlus", r"D:\Tools\TablePlus\TablePlus.exe");
        executable.source_kind = SourceKind::Portable;

        let merged = deduplicate(vec![executable, shortcut]);

        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].path, r"D:\Tools\TablePlus\TablePlus.lnk");
    }

    #[test]
    fn merges_game_shortcut_with_launcher_executable_in_same_product_folder() {
        let mut shortcut = app(
            "World of Warcraft",
            r"D:\World of Warcraft\World of Warcraft.lnk",
        );
        shortcut.launch_kind = LaunchKind::Shortcut;
        shortcut.source_kind = SourceKind::StartMenu;
        let mut executable = app(
            "World of Warcraft Launcher",
            r"D:\World of Warcraft\World of Warcraft Launcher.exe",
        );
        executable.source_kind = SourceKind::Portable;

        let merged = deduplicate(vec![executable, shortcut]);

        assert_eq!(merged.len(), 1);
        assert_eq!(
            merged[0].path,
            r"D:\World of Warcraft\World of Warcraft.lnk"
        );
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
    fn sanitizes_runtime_internal_portable_entries() {
        let apps = sanitize(vec![
            app(
                "codex-windows-sandbox",
                r"C:\Users\Maks\.codex\.sandbox-bin\codex-command-runner.exe",
            ),
            app(
                "Git Large File Storage (LFS)",
                r"C:\Users\Maks\.cache\codex-runtimes\codex-primary-runtime\dependencies\native\git\mingw64\libexec\git-core\git-lfs.exe",
            ),
            app(
                "git-credential-manager",
                r"D:\Tools\Git\mingw64\bin\git-credential-manager.exe",
            ),
            app(
                "GNU gettext utilities",
                r"D:\Tools\Git\mingw64\bin\printf_gettext.exe",
            ),
            app(
                "GNU gettext utilities",
                r"D:\Tools\Git\mingw64\bin\gettext.exe",
            ),
            app(
                "GNU gettext utilities",
                r"D:\Tools\Git\mingw64\bin\printf_ngettext.exe",
            ),
            {
                let mut launcher = app("Claude Code", r"C:\Menu\Claude Code.lnk");
                launcher.source_kind = SourceKind::StartMenu;
                launcher.launch_kind = LaunchKind::Shortcut;
                launcher.resolved_path = Some(r"C:\Users\Maks\.local\bin\claude.exe".into());
                launcher
            },
        ]);

        let primary = apps
            .iter()
            .filter(|app| app.visibility_class == VisibilityClass::Primary)
            .map(|app| app.name.as_str())
            .collect::<Vec<_>>();
        assert_eq!(primary, vec!["Claude Code"]);
        assert!(apps
            .iter()
            .filter(|app| app.name != "Claude Code")
            .all(|app| app.visibility_class == VisibilityClass::Auxiliary));
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
            AppCategory::FileCloud
        );
        assert_eq!(
            classify("Character Map", r"C:\charmap.exe"),
            AppCategory::WindowsFeatures
        );
    }

    #[test]
    fn classifies_new_general_categories() {
        // Office & Productivity.
        for (name, path) in [
            (
                "LibreOffice Calc",
                r"C:\Program Files\LibreOffice\scalc.exe",
            ),
            ("Notion", r"C:\Users\Maks\Notion\Notion.exe"),
            ("Obsidian", r"C:\Users\Maks\Obsidian\Obsidian.exe"),
            (
                "Foxit PDF Reader",
                r"C:\Program Files\Foxit\FoxitReader.exe",
            ),
        ] {
            assert_eq!(classify(name, path), AppCategory::Productivity, "{name}");
        }
        // Security & Privacy.
        for (name, path) in [
            ("Bitwarden", r"C:\Users\Maks\Bitwarden\Bitwarden.exe"),
            ("Malwarebytes", r"C:\Program Files\Malwarebytes\mb.exe"),
            ("WireGuard", r"C:\Program Files\WireGuard\wireguard.exe"),
        ] {
            assert_eq!(classify(name, path), AppCategory::Security, "{name}");
        }
        // File & Cloud.
        for (name, path) in [
            ("WinRAR", r"C:\Program Files\WinRAR\WinRAR.exe"),
            ("Total Commander", r"C:\totalcmd\TOTALCMD64.EXE"),
            ("Dropbox", r"C:\Program Files\Dropbox\Dropbox.exe"),
        ] {
            assert_eq!(classify(name, path), AppCategory::FileCloud, "{name}");
        }
    }

    #[test]
    fn bare_git_keyword_no_longer_miscategorizes_logitech() {
        // "Logitech" contains the substring "git"; it must not land in Development.
        assert_ne!(
            classify("Logitech G HUB", r"C:\Program Files\LGHUB\lghub.exe"),
            AppCategory::Development
        );
        // A real Git tool still classifies as Development.
        assert_eq!(
            classify("Git Bash", r"C:\Program Files\Git\git-bash.exe"),
            AppCategory::Development
        );
    }

    #[test]
    fn pdf_reader_beats_adobe_publisher_editors_mapping() {
        let mut acrobat = app(
            "Adobe Acrobat Reader",
            r"C:\Program Files\Adobe\Acrobat\Acrobat.exe",
        );
        acrobat.publisher = Some("Adobe Inc.".into());
        acrobat.resolved_path = Some(r"C:\Program Files\Adobe\Acrobat\AcroRd32.exe".into());
        assert_eq!(classify_app(&acrobat), AppCategory::Productivity);
    }

    #[test]
    fn antivirus_publisher_pins_security() {
        let mut kis = app("Protection", r"C:\Program Files\Kaspersky Lab\avp.exe");
        kis.publisher = Some("Kaspersky Lab".into());
        assert_eq!(classify_app(&kis), AppCategory::Security);
    }

    #[test]
    fn install_path_pins_productivity_and_browsers_for_cryptic_names() {
        // Neutral shortcut name, but the resolved target lives in a known product tree.
        let mut writer = app("Writer", r"C:\Menu\Writer.lnk");
        writer.resolved_path = Some(r"C:\Program Files\LibreOffice\program\swriter.exe".into());
        assert_eq!(classify_app(&writer), AppCategory::Productivity);

        // A launcher exe with no browser keyword, identified purely by its install tree.
        let mut ff = app("Nightly", r"C:\Menu\Nightly.lnk");
        ff.resolved_path = Some(r"C:\Program Files\Mozilla Firefox\launcher.exe".into());
        assert_eq!(classify_app(&ff), AppCategory::Browsers);
    }

    #[test]
    fn classify_app_reads_product_name_signal() {
        // A cryptic display name and exe; the PE product name carries the category.
        let mut note = app("Notes", r"C:\Apps\notes.exe");
        note.product_name = Some("Obsidian".into());
        assert_eq!(classify_app(&note), AppCategory::Productivity);
    }

    #[test]
    fn communication_and_ai_publishers_pin_category() {
        let mut tg = app("Nikogram", r"C:\Apps\ng.exe");
        tg.publisher = Some("Telegram FZ-LLC".into());
        assert_eq!(classify_app(&tg), AppCategory::Communication);

        let mut ai = app("Assistant", r"C:\Apps\assist.exe");
        ai.publisher = Some("OpenAI".into());
        assert_eq!(classify_app(&ai), AppCategory::Ai);
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
    fn classify_app_uses_source_publisher_and_install_path() {
        // Steam source → Games regardless of the name.
        let mut steam = app("Some Steam Title", "steam://rungameid/1");
        steam.source_kind = SourceKind::Steam;
        assert_eq!(classify_app(&steam), AppCategory::Games);

        // A Blizzard game whose name carries no game word.
        let mut hearthstone = app(
            "Hearthstone",
            r"C:\Program Files\Hearthstone\Hearthstone.exe",
        );
        hearthstone.publisher = Some("Blizzard Entertainment".into());
        assert_eq!(classify_app(&hearthstone), AppCategory::Games);

        // A game reached via a store install tree.
        let mut store_game = app("Launcher Title", r"C:\Menu\Launcher Title.lnk");
        store_game.resolved_path = Some(r"D:\Games\Battle.net\Diablo IV\game.exe".into());
        assert_eq!(classify_app(&store_game), AppCategory::Games);

        // Publisher pins the category.
        let mut resolve = app(
            "Untitled Project",
            r"C:\Program Files\Blackmagic Design\Resolve\Resolve.exe",
        );
        resolve.publisher = Some("Blackmagic Design Pty. Ltd.".into());
        assert_eq!(classify_app(&resolve), AppCategory::Editors);

        // A VPN client falls to Security by keyword.
        assert_eq!(
            classify_app(&app("Hiddify", r"C:\Users\Maks\Hiddify\Hiddify.exe")),
            AppCategory::Security
        );

        // A hardware vendor is NOT a game publisher (no bare "ea"/"riot" substring hit).
        let mut driver = app("Realtek Audio Console", r"C:\Program Files\Realtek\rtk.exe");
        driver.publisher = Some("Realtek Semiconductor Corp.".into());
        assert_ne!(classify_app(&driver), AppCategory::Games);
    }

    #[test]
    fn classify_app_reads_the_resolved_executable_name() {
        // Sota Connect: neutral display name and .lnk, but the real target is SotaVPN.exe.
        let mut sota = app("Sota Connect", r"C:\Menu\Sota Connect.lnk");
        sota.launch_kind = LaunchKind::Shortcut;
        sota.source_kind = SourceKind::StartMenu;
        sota.resolved_path = Some(r"D:\Apps\SotaConnect\SotaVPN.exe".into());
        assert_eq!(classify_app(&sota), AppCategory::Security);

        // OpenCode → AI, even though its exe path contains "code.exe" (AI is matched before Dev).
        let mut opencode = app("OpenCode", r"C:\Menu\OpenCode.lnk");
        opencode.launch_kind = LaunchKind::Shortcut;
        opencode.resolved_path = Some(r"D:\Apps\OpenCode\OpenCode.exe".into());
        assert_eq!(classify_app(&opencode), AppCategory::Ai);

        // MongoDB Compass → Development (publisher-pinned).
        let mut mongo = app("MongoDB Compass", r"C:\Menu\MongoDB Compass.lnk");
        mongo.publisher = Some("MongoDB Inc".into());
        mongo.resolved_path =
            Some(r"C:\Users\Maks\AppData\Local\MongoDBCompass\MongoDBCompass.exe".into());
        assert_eq!(classify_app(&mongo), AppCategory::Development);

        // The Hiddify VPN client → Security.
        assert_eq!(
            classify_app(&app("Hiddify", r"C:\Program Files\Hiddify\Hiddify.exe")),
            AppCategory::Security
        );
    }

    #[test]
    fn tokenized_matching_ignores_substrings_of_unrelated_words() {
        // Whole-word matching: a keyword must be a full token, never a substring. These are the
        // false-positive classes the previous substring cascade had to hand-patch around.
        assert_eq!(
            classify("Omega Launcher", r"C:\Omega\omega.exe"),
            AppCategory::Other,
            "'mega' inside 'Omega' must not imply File & Cloud"
        );
        assert_eq!(
            classify("Excellent Notes", r"C:\ExcellentNotes.exe"),
            AppCategory::Other,
            "'excel' inside 'Excellent' must not imply Productivity"
        );
        assert_ne!(
            classify("Realtek Digital Output", r"C:\Realtek\rtk.exe"),
            AppCategory::Development,
            "'git' inside 'Digital' must not imply Development"
        );
    }

    #[test]
    fn file_description_signal_classifies_cryptic_names() {
        // The PE file description often names the category outright even when the display name and
        // executable are opaque.
        let mut browser = app("Nyxbrowse", r"C:\Apps\nyx.exe");
        browser.description = Some("Internet Browser".into());
        assert_eq!(classify_app(&browser), AppCategory::Browsers);

        let mut shield = app("Shield", r"C:\Apps\shield.exe");
        shield.description = Some("Antivirus".into());
        assert_eq!(classify_app(&shield), AppCategory::Security);
    }

    #[test]
    fn original_filename_reveals_a_renamed_launcher() {
        // A neutral shortcut whose PE original filename is the real browser executable.
        let mut renamed = app("Fast Start", r"C:\Menu\Fast Start.lnk");
        renamed.original_filename = Some("chrome.exe".into());
        assert_eq!(classify_app(&renamed), AppCategory::Browsers);
    }

    #[test]
    fn corroborating_signals_accumulate_to_break_a_tie() {
        // "Cursor Code" hits AI ("cursor") and Development ("code") equally by name; the AI executable
        // adds weight so AI wins on the total, not on list order.
        let mut cursor = app("Cursor Code", r"C:\Apps\Cursor.exe");
        cursor.resolved_path = Some(r"C:\Apps\Cursor.exe".into());
        assert_eq!(classify_app(&cursor), AppCategory::Ai);
    }

    #[test]
    fn game_engine_and_games_folder_classify_unknown_indie_titles() {
        // Brotato: portable, non-Steam, its title in no keyword list. The Godot engine in the file
        // description and the dedicated Games folder identify it structurally.
        let mut brotato = app("Brotato", r"D:\Games\Brotato\Brotato.exe");
        brotato.source_kind = SourceKind::Portable;
        brotato.publisher = Some("Blobfish Games".into());
        brotato.product_name = Some("Brotato".into());
        brotato.description = Some("Godot Engine".into());
        assert_eq!(classify_app(&brotato), AppCategory::Games);

        // The engine fingerprint alone suffices, without a games folder.
        let mut indie = app("Unknowable", r"C:\Apps\Unknowable\game.exe");
        indie.description = Some("Unreal Engine".into());
        assert_eq!(classify_app(&indie), AppCategory::Games);

        // A dedicated games folder alone tips an otherwise-unknown title to Games.
        assert_eq!(
            classify_app(&app("Zephyr", r"D:\Games\Zephyr\Zephyr.exe")),
            AppCategory::Games
        );
    }

    #[test]
    fn extended_coverage_classifies_common_apps_not_installed_here() {
        use AppCategory::*;
        // Popular apps a typical user has, identified by distinctive name tokens.
        for (name, path, want) in [
            ("Epic Games Launcher", r"C:\Menu\Epic.lnk", Games),
            ("CurseForge", r"C:\Menu\CurseForge.lnk", Games),
            (
                "PostgreSQL 16",
                r"C:\Program Files\PostgreSQL\16\bin\postgres.exe",
                Development,
            ),
            ("PuTTY", r"C:\Program Files\PuTTY\putty.exe", Development),
            ("Zed", r"C:\Apps\Zed\zed.exe", Development),
            (
                "HandBrake",
                r"C:\Program Files\HandBrake\HandBrake.exe",
                Editors,
            ),
            ("Autodesk AutoCAD 2024", r"C:\Menu\AutoCAD.lnk", Editors),
            ("Todoist", r"C:\Apps\Todoist\Todoist.exe", Productivity),
            (
                "Microsoft PowerPoint",
                r"C:\Menu\PowerPoint.lnk",
                Productivity,
            ),
            ("Tor Browser", r"C:\Apps\Tor Browser\firefox.exe", Browsers),
            (
                "LibreWolf",
                r"C:\Program Files\LibreWolf\librewolf.exe",
                Browsers,
            ),
            (
                "OBS Studio",
                r"C:\Program Files\obs-studio\bin\obs64.exe",
                Media,
            ),
            ("FL Studio", r"C:\Program Files\FL Studio\FL64.exe", Media),
            ("Microsoft Teams", r"C:\Menu\Teams.lnk", Communication),
            (
                "qBittorrent",
                r"C:\Program Files\qBittorrent\qbittorrent.exe",
                FileCloud,
            ),
            ("WinZip", r"C:\Program Files\WinZip\winzip64.exe", FileCloud),
            (
                "Tailscale",
                r"C:\Program Files\Tailscale\tailscale.exe",
                Security,
            ),
            (
                "VeraCrypt",
                r"C:\Program Files\VeraCrypt\VeraCrypt.exe",
                Security,
            ),
            (
                "TeamViewer",
                r"C:\Program Files\TeamViewer\TeamViewer.exe",
                Utilities,
            ),
            ("ShareX", r"C:\Program Files\ShareX\ShareX.exe", Utilities),
            ("NVIDIA App", r"C:\Menu\NVIDIA App.lnk", Utilities),
        ] {
            assert_eq!(classify(name, path), want, "{name}");
        }

        // Publisher rules generalize across every product a vendor ships, even a neutral name.
        for (publisher, want) in [
            ("Autodesk Inc.", Editors),
            ("NVIDIA Corporation", Utilities),
            ("Sophos Ltd.", Security),
            ("PostgreSQL Global Development Group", Development),
        ] {
            let mut a = app("Neutral Tool", r"C:\Apps\tool.exe");
            a.publisher = Some(publisher.into());
            assert_eq!(classify_app(&a), want, "{publisher}");
        }
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
    fn shortcut_icon_source_prefers_target_when_icon_location_is_empty() {
        let dir = tempfile::tempdir().unwrap();
        let shortcut = dir.path().join("Happ.lnk");
        let target = dir.path().join("Happ.exe");
        std::fs::write(&shortcut, []).unwrap();
        std::fs::write(&target, []).unwrap();

        let mut value = app("Happ", &shortcut.to_string_lossy());
        value.launch_kind = LaunchKind::Shortcut;
        value.resolved_path = Some(target.to_string_lossy().into_owned());
        assert_eq!(
            icon_source(&value).as_deref(),
            Some(target.to_string_lossy().as_ref())
        );
    }

    #[test]
    fn icon_candidates_order_icon_location_then_target_then_path() {
        let dir = tempfile::tempdir().unwrap();
        let shortcut = dir.path().join("PgAdmin.lnk");
        let icon = dir.path().join("pgAdmin4.ico");
        let target = dir.path().join("pgAdmin4.exe");
        for file in [&shortcut, &icon, &target] {
            std::fs::write(file, []).unwrap();
        }

        let mut value = app("pgAdmin 4", &shortcut.to_string_lossy());
        value.launch_kind = LaunchKind::Shortcut;
        value.shortcut_icon_path = Some(icon.to_string_lossy().into_owned());
        value.resolved_path = Some(target.to_string_lossy().into_owned());

        let candidates = icon_source_candidates(&value);
        assert_eq!(
            candidates,
            vec![
                icon.to_string_lossy().into_owned(),
                target.to_string_lossy().into_owned(),
                shortcut.to_string_lossy().into_owned(),
            ],
        );
    }

    #[test]
    fn icon_candidates_skip_missing_files_and_aumid_path() {
        let mut value = app("Calc", "Microsoft.WindowsCalculator_8wekyb3d8bbwe!App");
        value.launch_kind = LaunchKind::AppUserModelId;
        value.shortcut_icon_path = Some(r"C:\missing\icon.ico".into());
        assert!(icon_source_candidates(&value).is_empty());
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
    fn different_portable_versions_are_kept_as_separate_apps() {
        // A different version is a different application: two portable Rufus builds must stay two
        // cards rather than collapsing to the newest. (Same-version copies still merge — see
        // dedup::tests::portable_copies_merge_on_equal_version_only.)
        let mut old = app("Rufus", r"E:\Tools\rufus-3.11p.exe");
        old.source_kind = SourceKind::Portable;
        old.version = Some("3.11.0".into());
        let mut current = app("Rufus", r"D:\Tools\rufus-4.11p.exe");
        current.source_kind = SourceKind::Portable;
        current.version = Some("4.11.2285".into());

        assert_eq!(deduplicate(vec![old, current]).len(), 2);
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
