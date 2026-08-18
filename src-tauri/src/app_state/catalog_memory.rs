use crate::app_state::{AppState, CloseTarget, LaunchTarget, UninstallRecord};
use crate::catalog::cache::CachedAppDetails;
use crate::catalog::{self, AppDetailsTarget, AppInfo};
use std::collections::{BTreeMap, HashSet};
use std::path::Path;

pub(crate) fn remember_catalog(state: &AppState, apps: &[AppInfo]) {
    remember_catalog_ids(state, apps);
    remember_uninstall_targets(state, apps);
    remember_launch_targets(state, apps);
    remember_close_targets(state, apps);
    remember_app_details_targets(state, apps);
    retain_app_details_cache(state, apps);
}

fn remember_catalog_ids(state: &AppState, apps: &[AppInfo]) {
    let ids = apps.iter().map(|app| app.id.clone()).collect();
    if let Ok(mut stored) = state.catalog_ids.lock() {
        *stored = ids;
    }
}

pub(crate) fn known_catalog_ids(state: &AppState, ids: Vec<String>) -> Vec<String> {
    let Ok(known) = state.catalog_ids.lock() else {
        return Vec::new();
    };
    let mut seen = HashSet::with_capacity(ids.len());
    ids.into_iter()
        .filter(|id| known.contains(id) && seen.insert(id.clone()))
        .collect()
}

fn remember_uninstall_targets(state: &AppState, apps: &[AppInfo]) {
    let targets = apps
        .iter()
        .filter_map(|app| {
            app.uninstall.clone().map(|target| {
                (
                    app.id.clone(),
                    UninstallRecord {
                        app_name: app.name.clone(),
                        publisher: app.publisher.clone(),
                        source_kind: app.source_kind,
                        target,
                    },
                )
            })
        })
        .collect();
    if let Ok(mut stored) = state.uninstall_targets.lock() {
        *stored = targets;
    }
}

fn remember_launch_targets(state: &AppState, apps: &[AppInfo]) {
    let targets = apps
        .iter()
        .map(|app| {
            (
                app.id.clone(),
                LaunchTarget {
                    kind: app.launch_kind,
                    path: app.path.clone(),
                    arguments: app.launch_arguments.clone(),
                },
            )
        })
        .collect();
    if let Ok(mut stored) = state.launch_targets.lock() {
        *stored = targets;
    }
}

fn remember_close_targets(state: &AppState, apps: &[AppInfo]) {
    let targets = apps
        .iter()
        .filter_map(|app| {
            let path = catalog::close_target_of(app)?;
            let blocked = crate::platform::windows::close_risk(Path::new(&path))
                != crate::platform::windows::CloseRisk::Safe;
            Some((
                app.id.clone(),
                CloseTarget {
                    install_root: catalog::close_scope_of(app),
                    path,
                    blocked,
                },
            ))
        })
        .collect();
    if let Ok(mut stored) = state.close_targets.lock() {
        *stored = targets;
    }
}

fn remember_app_details_targets(state: &AppState, apps: &[AppInfo]) {
    let targets = apps
        .iter()
        .map(|app| (app.id.clone(), AppDetailsTarget::from_app(app)))
        .collect();
    if let Ok(mut stored) = state.app_details_targets.lock() {
        *stored = targets;
    }
}

fn retain_app_details_cache(state: &AppState, apps: &[AppInfo]) {
    let live_ids = apps
        .iter()
        .map(|app| app.id.as_str())
        .collect::<HashSet<_>>();
    if let Ok(mut cached) = state.app_details_cache.lock() {
        cached.retain(|id, _| live_ids.contains(id.as_str()));
    }
}

pub(crate) fn app_details_target_for(state: &AppState, id: &str) -> Option<AppDetailsTarget> {
    state.app_details_targets.lock().ok()?.get(id).cloned()
}

pub(crate) fn cached_app_details_for(
    state: &AppState,
    id: &str,
    fingerprint: &str,
) -> Option<catalog::AppDetails> {
    state
        .app_details_cache
        .lock()
        .ok()?
        .get(id)
        .filter(|cached| cached.fingerprint == fingerprint)
        .map(|cached| cached.details.clone())
}

pub(crate) fn remember_app_details(
    state: &AppState,
    id: String,
    fingerprint: String,
    details: catalog::AppDetails,
) {
    if let Ok(mut cached) = state.app_details_cache.lock() {
        cached.insert(
            id,
            CachedAppDetails {
                fingerprint,
                details,
            },
        );
    }
}

pub(crate) fn cached_details_for_catalog(
    state: &AppState,
    apps: &[AppInfo],
    persisted: BTreeMap<String, CachedAppDetails>,
) -> BTreeMap<String, CachedAppDetails> {
    let live_ids = apps
        .iter()
        .map(|app| app.id.as_str())
        .collect::<HashSet<_>>();
    let mut current = persisted
        .into_iter()
        .filter(|(id, _)| live_ids.contains(id.as_str()))
        .collect::<BTreeMap<_, _>>();
    let Ok(cached) = state.app_details_cache.lock() else {
        return current;
    };
    for (id, details) in cached
        .iter()
        .filter(|(id, _)| live_ids.contains(id.as_str()))
    {
        current.insert(id.clone(), details.clone());
    }
    current
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_state::cached_app;
    use crate::catalog::LaunchKind;

    #[test]
    fn launch_targets_only_resolve_known_catalog_ids() {
        let mut app = cached_app("Visual Studio Code", r"C:\Code.exe");
        app.id = "code".into();
        app.launch_kind = LaunchKind::Executable;
        let state = AppState::default();
        remember_launch_targets(&state, &[app]);
        let stored = state.launch_targets.lock().unwrap();
        assert_eq!(
            stored.get("code").cloned(),
            Some(LaunchTarget {
                kind: LaunchKind::Executable,
                path: r"C:\Code.exe".to_string(),
                arguments: None,
            })
        );
        assert!(stored.get("unknown-id").is_none());
    }

    #[test]
    fn launch_targets_remember_catalog_arguments() {
        let mut app = cached_app("Editor", r"C:\\Editor\\editor.exe");
        app.id = "editor".into();
        app.launch_arguments = Some("--safe-mode".into());
        let state = AppState::default();

        remember_launch_targets(&state, &[app]);

        assert_eq!(
            state
                .launch_targets
                .lock()
                .unwrap()
                .get("editor")
                .unwrap()
                .arguments,
            Some("--safe-mode".into())
        );
    }

    #[test]
    fn close_targets_cover_only_entries_that_name_a_running_executable() {
        let mut executable = cached_app("Editor", r"C:\Editor\editor.exe");
        executable.id = "editor".into();
        let mut shortcut = cached_app("Game", r"C:\Menu\game.lnk");
        shortcut.id = "game".into();
        shortcut.launch_kind = LaunchKind::Shortcut;
        shortcut.resolved_path = Some(r"C:\Games\game.exe".into());
        let mut package = cached_app("Camera", "Microsoft.WindowsCamera_8wekyb3d8bbwe!App");
        package.id = "camera".into();
        package.launch_kind = LaunchKind::AppUserModelId;
        let mut steam = cached_app("Steam game", "steam://rungameid/1");
        steam.id = "steam-game".into();
        let state = AppState::default();

        remember_close_targets(&state, &[executable, shortcut, package, steam]);

        let stored = state.close_targets.lock().unwrap();
        assert_eq!(
            stored.get("editor").map(|target| target.path.as_str()),
            Some(r"C:\Editor\editor.exe")
        );
        assert_eq!(
            stored.get("game").map(|target| target.path.as_str()),
            Some(r"C:\Games\game.exe")
        );
        assert!(stored.get("camera").is_none());
        assert!(stored.get("steam-game").is_none());
    }

    #[test]
    fn processes_that_would_end_windows_or_the_desktop_session_are_remembered_as_blocked() {
        let mut security = cached_app("Local Security Authority", r"C:\Windows\System32\lsass.exe");
        security.id = "lsass".into();
        let mut explorer = cached_app("Проводник", r"C:\Windows\explorer.exe");
        explorer.id = "explorer".into();
        let mut editor = cached_app("Editor", r"C:\Editor\editor.exe");
        editor.id = "editor".into();
        let state = AppState::default();

        remember_close_targets(&state, &[security, explorer, editor]);

        let stored = state.close_targets.lock().unwrap();
        assert!(stored.get("lsass").unwrap().blocked);
        assert!(stored.get("explorer").unwrap().blocked);
        assert!(!stored.get("editor").unwrap().blocked);
    }

    #[test]
    fn close_targets_include_a_store_app_that_resolved_a_package_executable() {
        let mut calculator = cached_app("Калькулятор", "Microsoft.WindowsCalculator_8wek!App");
        calculator.id = "calculator".into();
        calculator.launch_kind = LaunchKind::AppUserModelId;
        calculator.resolved_path =
            Some(r"C:\Program Files\WindowsApps\Microsoft.WindowsCalculator_11.0_x64__8wek\CalculatorApp.exe".into());
        let mut documentation = cached_app("Node.js website", "https://nodejs.org/");
        documentation.id = "docs".into();
        documentation.launch_kind = LaunchKind::AppUserModelId;
        documentation.resolved_path = Some("https://nodejs.org/".into());
        let state = AppState::default();

        remember_close_targets(&state, &[calculator, documentation]);

        let stored = state.close_targets.lock().unwrap();
        assert_eq!(
            stored.get("calculator").map(|target| target.path.as_str()),
            Some(
                r"C:\Program Files\WindowsApps\Microsoft.WindowsCalculator_11.0_x64__8wek\CalculatorApp.exe"
            )
        );
        assert!(stored.get("docs").is_none());
    }

    #[test]
    fn details_targets_resolve_only_catalog_entries_and_keep_trusted_aumid_executables() {
        let mut executable = cached_app("Editor", r"C:\Editor\editor.exe");
        executable.id = "editor".into();
        let mut aumid = cached_app("Store app", "Contoso.Store_123!App");
        aumid.id = "store".into();
        aumid.launch_kind = LaunchKind::AppUserModelId;
        aumid.resolved_path = Some(r"C:\Program Files\WindowsApps\Contoso\app.exe".into());
        let state = AppState::default();

        remember_catalog(&state, &[executable, aumid]);

        assert!(app_details_target_for(&state, "editor").is_some());
        assert!(app_details_target_for(&state, "unknown").is_none());
        let store = app_details_target_for(&state, "store").unwrap();
        assert!(catalog::details_fingerprint(&store).is_some());
    }

    #[test]
    fn keeps_current_memory_details_for_the_next_catalog_cache_write() {
        let mut app = cached_app("Editor", r"C:\Editor\editor.exe");
        app.id = "editor".into();
        let state = AppState::default();
        remember_catalog(&state, &[app.clone()]);
        remember_app_details(
            &state,
            "editor".into(),
            "current".into(),
            catalog::AppDetails {
                file_size_bytes: Some(42),
                ..Default::default()
            },
        );
        let persisted = BTreeMap::from([
            (
                "editor".into(),
                CachedAppDetails {
                    fingerprint: "stale".into(),
                    details: Default::default(),
                },
            ),
            (
                "removed".into(),
                CachedAppDetails {
                    fingerprint: "old".into(),
                    details: Default::default(),
                },
            ),
        ]);

        let merged = cached_details_for_catalog(&state, &[app], persisted);

        assert_eq!(merged.len(), 1);
        assert_eq!(merged["editor"].fingerprint, "current");
        assert_eq!(merged["editor"].details.file_size_bytes, Some(42));
    }
}
