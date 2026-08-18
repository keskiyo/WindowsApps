mod catalog_memory;
mod launch_waits;
mod uninstall;

pub(crate) use catalog_memory::{
    app_details_target_for, cached_app_details_for, cached_details_for_catalog, known_catalog_ids,
    remember_app_details, remember_catalog,
};
pub(crate) use launch_waits::LaunchWaitLimiter;
pub(crate) use uninstall::{execute_and_record, preview_for, UninstallPreview, UninstallRecord};

use crate::catalog::cache::CachedAppDetails;
use crate::catalog::scan_coordinator::ScanCoordinator;
use crate::catalog::{self, AppDetailsTarget, AppInfo, LaunchKind};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CloseTarget {
    pub(crate) path: String,
    pub(crate) install_root: Option<String>,
    pub(crate) blocked: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LaunchTarget {
    pub(crate) kind: LaunchKind,
    pub(crate) path: String,
    pub(crate) arguments: Option<String>,
}

#[derive(Default)]
pub(crate) struct AppState {
    pub(crate) catalog_ids: Mutex<HashSet<String>>,
    pub(crate) uninstall_targets: Mutex<HashMap<String, UninstallRecord>>,
    pub(crate) launch_targets: Mutex<HashMap<String, LaunchTarget>>,
    pub(crate) close_targets: Mutex<HashMap<String, CloseTarget>>,
    pub(crate) app_details_targets: Mutex<HashMap<String, AppDetailsTarget>>,
    pub(crate) app_details_cache: Mutex<HashMap<String, CachedAppDetails>>,
    pub(crate) launch_waits: Arc<LaunchWaitLimiter>,
    pub(crate) sync_lock: Mutex<()>,
    pub(crate) scan_coordinator: ScanCoordinator<Vec<AppInfo>>,
    pub(crate) hydration_queue: Mutex<catalog::hydration::HydrationQueue>,
    pub(crate) change_watcher:
        Mutex<Option<crate::platform::windows::change_watcher::WatcherGuard>>,
    pub(crate) global_shortcut:
        Mutex<Option<crate::platform::windows::global_shortcut::ShortcutGuard>>,
    pub(crate) shortcut_status: Mutex<crate::platform::windows::global_shortcut::Status>,
}

#[cfg(test)]
pub(crate) fn cached_app(name: &str, path: &str) -> AppInfo {
    use crate::catalog::SourceKind;
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
        target_availability: None,
        category_reasons: Vec::new(),
        close_risk: None,
    }
}
