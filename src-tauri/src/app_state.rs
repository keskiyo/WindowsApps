//! Process-wide catalog state owned by the Tauri app instance, plus the trusted
//! id-keyed target maps that keep launch/uninstall resolution server-side.

use crate::catalog::cache::CachedAppDetails;
use crate::catalog::scan_coordinator::ScanCoordinator;
use crate::catalog::{self, AppDetailsTarget, AppInfo, LaunchKind, SourceKind, UninstallTarget};
use crate::platform::windows::{uninstall_history, uninstaller};
use serde::Serialize;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

const MAX_CONCURRENT_LAUNCH_WAITS: usize = 8;

pub(crate) struct LaunchWaitLimiter {
    active: AtomicUsize,
}

impl Default for LaunchWaitLimiter {
    fn default() -> Self {
        Self {
            active: AtomicUsize::new(0),
        }
    }
}

impl LaunchWaitLimiter {
    pub(crate) fn acquire(self: &Arc<Self>) -> Option<LaunchWaitPermit> {
        self.active
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |active| {
                (active < MAX_CONCURRENT_LAUNCH_WAITS).then_some(active + 1)
            })
            .ok()
            .map(|_| LaunchWaitPermit {
                limiter: Arc::clone(self),
            })
    }
}

pub(crate) struct LaunchWaitPermit {
    limiter: Arc<LaunchWaitLimiter>,
}

impl Drop for LaunchWaitPermit {
    fn drop(&mut self) {
        self.limiter.active.fetch_sub(1, Ordering::AcqRel);
    }
}

#[derive(Clone, Debug)]
pub(crate) struct UninstallRecord {
    pub(crate) app_name: String,
    pub(crate) publisher: Option<String>,
    pub(crate) source_kind: SourceKind,
    pub(crate) target: UninstallTarget,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UninstallPreview {
    pub(crate) app_name: String,
    pub(crate) publisher: Option<String>,
    pub(crate) source: SourceKind,
    pub(crate) mechanism: uninstaller::UninstallMechanism,
}

/// All process-wide mutable catalog state, owned by the Tauri app instance through
/// `manage`/`State` instead of module-level statics. Commands and app-scoped helpers
/// reach it via `app.state::<AppState>()`; tests construct `AppState::default()`
/// directly, so shared state never leaks across test cases.
#[derive(Default)]
pub(crate) struct AppState {
    /// Catalog ids accepted from trusted backend scans/cache reads.
    pub(crate) catalog_ids: Mutex<HashSet<String>>,
    /// Trusted uninstall records keyed by catalog id (resolved server-side, never from IPC).
    pub(crate) uninstall_targets: Mutex<HashMap<String, UninstallRecord>>,
    /// Trusted launch targets (kind + path) keyed by catalog id.
    pub(crate) launch_targets: Mutex<HashMap<String, (LaunchKind, String)>>,
    /// Trusted executable paths for closing, keyed by catalog id. Only entries whose running
    /// image can be identified appear here; see `remember_close_targets`.
    pub(crate) close_targets: Mutex<HashMap<String, String>>,
    /// Trusted local metadata inputs keyed by catalog id.
    pub(crate) app_details_targets: Mutex<HashMap<String, AppDetailsTarget>>,
    /// Details read during this run, merged into the next synchronized cache document.
    pub(crate) app_details_cache: Mutex<HashMap<String, CachedAppDetails>>,
    /// Bounds process readiness waits for this application instance.
    pub(crate) launch_waits: Arc<LaunchWaitLimiter>,
    /// Serializes catalog synchronization so scans never write the cache concurrently.
    pub(crate) sync_lock: Mutex<()>,
    /// Coalesces overlapping scan requests into a single in-flight job.
    pub(crate) scan_coordinator: ScanCoordinator<Vec<AppInfo>>,
    /// Pending icon/metadata hydration work.
    pub(crate) hydration_queue: Mutex<catalog::hydration::HydrationQueue>,
    /// Active filesystem-change watcher guard (dropped to stop watching).
    pub(crate) change_watcher:
        Mutex<Option<crate::platform::windows::change_watcher::WatcherGuard>>,
    /// Global hotkey registration and its message-loop thread (dropped to unregister).
    pub(crate) global_shortcut:
        Mutex<Option<crate::platform::windows::global_shortcut::ShortcutGuard>>,
    /// Last known registration status of the global hotkey, shown on the settings page.
    pub(crate) shortcut_status: Mutex<crate::platform::windows::global_shortcut::Status>,
}

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

pub(crate) fn remember_uninstall_targets(state: &AppState, apps: &[AppInfo]) {
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

/// Record the trusted launch target (kind + path) for every catalog entry, keyed by
/// id. `launch_app` resolves through this map so the webview can only launch apps the
/// scanner actually found — never an arbitrary path supplied over IPC.
pub(crate) fn remember_launch_targets(state: &AppState, apps: &[AppInfo]) {
    let targets = apps
        .iter()
        .map(|app| (app.id.clone(), (app.launch_kind, app.path.clone())))
        .collect();
    if let Ok(mut stored) = state.launch_targets.lock() {
        *stored = targets;
    }
}

/// Record the executable a catalog entry actually runs as, so `close_apps` can match it against
/// the running processes without the webview ever naming a path.
///
/// The question is only ever "which image ends up running": a shortcut contributes its resolved
/// target rather than the `.lnk` that never runs, and a Store entry contributes the executable its
/// package manifest declares — already validated to live inside the package directory by the
/// Start-Apps source. An AppUserModelId or a `steam://` string is not that image, so it is never
/// used as a fallback; entries that resolve nothing stay out of the map and the command reports
/// the close as unavailable instead of guessing at a process.
pub(crate) fn remember_close_targets(state: &AppState, apps: &[AppInfo]) {
    let targets = apps
        .iter()
        .filter_map(|app| {
            let path = app.resolved_path.clone().or_else(|| {
                matches!(
                    app.launch_kind,
                    LaunchKind::Executable | LaunchKind::Shortcut
                )
                .then(|| app.path.clone())
            })?;
            Path::new(&path)
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
                .then(|| (app.id.clone(), path))
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

/// Detail requests update only memory. The next cache synchronization merges that small map
/// before its existing serialized write, so no command mutex spans filesystem work.
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

pub(crate) fn preview_for(record: &UninstallRecord) -> UninstallPreview {
    let target = uninstaller::preview(&record.target);
    UninstallPreview {
        app_name: record.app_name.clone(),
        publisher: record.publisher.clone(),
        source: record.source_kind,
        mechanism: target.mechanism,
    }
}

pub(crate) fn execute_and_record(
    app_data_dir: &Path,
    record: UninstallRecord,
    executor: impl FnOnce(UninstallTarget) -> Result<(), String>,
) -> Result<(), String> {
    let preview = uninstaller::preview(&record.target);
    let result = executor(record.target);
    let history_result = if result.is_ok() {
        uninstall_history::UninstallResult::Succeeded
    } else {
        uninstall_history::UninstallResult::Failed
    };
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs());
    let _ = uninstall_history::append(
        app_data_dir,
        uninstall_history::UninstallHistoryEntry {
            id: String::new(),
            timestamp,
            app_name: record.app_name,
            publisher: record.publisher,
            mechanism: preview.mechanism,
            result: history_result,
        },
    );
    result
}

/// Shared test builder for a minimal cached [`AppInfo`]; used by unit tests across the
/// app-layer modules.
#[cfg(test)]
pub(crate) fn cached_app(name: &str, path: &str) -> AppInfo {
    AppInfo {
        id: path.into(),
        name: name.into(),
        path: path.into(),
        icon_base64: None,
        artifact_kind: Default::default(),
        category: catalog::AppCategory::Other,
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

#[cfg(test)]
mod tests {
    use super::*;

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
            Some((LaunchKind::Executable, r"C:\Code.exe".to_string()))
        );
        assert!(stored.get("unknown-id").is_none());
    }

    // A close matches a running image, so only entries that name one get a target. A `steam://`
    // handler names no image, and closing "whatever steam.exe is running" would be a guess at a
    // process the user never chose.
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
            stored.get("editor").map(String::as_str),
            Some(r"C:\Editor\editor.exe")
        );
        // A shortcut contributes the image it resolves to, not the .lnk that is never running.
        assert_eq!(
            stored.get("game").map(String::as_str),
            Some(r"C:\Games\game.exe")
        );
        // No manifest executable was resolved, so there is nothing to match a process against.
        assert!(stored.get("camera").is_none());
        assert!(stored.get("steam-game").is_none());
    }

    // The reported bug: a scenario could not close Калькулятор. Store entries launch by
    // AppUserModelId, and the filter dropped every one of them — including the ones whose package
    // manifest already resolved a real executable inside the package directory. That resolved
    // image is exactly what a close has to match, so those entries get a target; the AUMID string
    // in `path` never does, because it names no file.
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
            stored.get("calculator").map(String::as_str),
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
        use crate::catalog::cache::CachedAppDetails;
        use std::collections::BTreeMap;

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

    #[test]
    fn launch_wait_capacity_is_app_scoped() {
        let first = AppState::default();
        let second = AppState::default();
        let permits = (0..8)
            .map(|_| first.launch_waits.acquire().unwrap())
            .collect::<Vec<_>>();

        assert!(first.launch_waits.acquire().is_none());
        assert!(second.launch_waits.acquire().is_some());
        drop(permits);
        assert!(first.launch_waits.acquire().is_some());
    }

    fn uninstall_record(name: &str) -> UninstallRecord {
        UninstallRecord {
            app_name: name.into(),
            publisher: Some("Publisher".into()),
            source_kind: SourceKind::Registry,
            target: UninstallTarget::Command {
                executable: "uninstall.exe".into(),
                arguments: "/quiet".into(),
            },
        }
    }

    #[test]
    fn records_successful_uninstall_without_command_details() {
        let dir = tempfile::tempdir().unwrap();
        execute_and_record(dir.path(), uninstall_record("Editor"), |_| Ok(())).unwrap();

        let history = uninstall_history::read(dir.path());
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].app_name, "Editor");
        assert_eq!(
            history[0].result,
            uninstall_history::UninstallResult::Succeeded
        );
        let serialized = serde_json::to_value(&history[0]).unwrap();
        assert!(serialized.get("command").is_none());
        assert!(serialized.get("path").is_none());
        assert!(serialized.get("error").is_none());
    }

    #[test]
    fn uninstall_preview_excludes_command_details() {
        let serialized = serde_json::to_value(preview_for(&uninstall_record("Editor"))).unwrap();

        assert_eq!(serialized.get("command"), None);
        assert_eq!(serialized.get("path"), None);
        assert_eq!(serialized["mechanism"], "registered_command");
    }

    #[test]
    fn records_failed_uninstall_and_returns_original_error() {
        let dir = tempfile::tempdir().unwrap();
        let error = execute_and_record(dir.path(), uninstall_record("Editor"), |_| {
            Err("boom".into())
        })
        .unwrap_err();

        let history = uninstall_history::read(dir.path());
        assert_eq!(error, "boom");
        assert_eq!(
            history[0].result,
            uninstall_history::UninstallResult::Failed
        );
    }
}
