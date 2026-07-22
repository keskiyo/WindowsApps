//! Process-wide catalog state owned by the Tauri app instance, plus the trusted
//! id-keyed target maps that keep launch/uninstall resolution server-side.

use crate::catalog::scan_coordinator::ScanCoordinator;
use crate::catalog::{self, AppInfo, LaunchKind, SourceKind, UninstallTarget};
use crate::platform::windows::{uninstall_history, uninstaller};
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

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
    pub(crate) command: String,
}

/// All process-wide mutable catalog state, owned by the Tauri app instance through
/// `manage`/`State` instead of module-level statics. Commands and app-scoped helpers
/// reach it via `app.state::<AppState>()`; tests construct `AppState::default()`
/// directly, so shared state never leaks across test cases.
#[derive(Default)]
pub(crate) struct AppState {
    /// Trusted uninstall records keyed by catalog id (resolved server-side, never from IPC).
    pub(crate) uninstall_targets: Mutex<HashMap<String, UninstallRecord>>,
    /// Trusted launch targets (kind + path) keyed by catalog id.
    pub(crate) launch_targets: Mutex<HashMap<String, (LaunchKind, String)>>,
    /// Serializes catalog synchronization so scans never write the cache concurrently.
    pub(crate) sync_lock: Mutex<()>,
    /// Coalesces overlapping scan requests into a single in-flight job.
    pub(crate) scan_coordinator: ScanCoordinator<Vec<AppInfo>>,
    /// Pending icon/metadata hydration work.
    pub(crate) hydration_queue: Mutex<catalog::hydration::HydrationQueue>,
    /// Active filesystem-change watcher guard (dropped to stop watching).
    pub(crate) change_watcher:
        Mutex<Option<crate::platform::windows::change_watcher::WatcherGuard>>,
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

pub(crate) fn preview_for(record: &UninstallRecord) -> UninstallPreview {
    let target = uninstaller::preview(&record.target);
    UninstallPreview {
        app_name: record.app_name.clone(),
        publisher: record.publisher.clone(),
        source: record.source_kind,
        mechanism: target.mechanism,
        command: target.command,
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
