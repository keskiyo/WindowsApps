mod assemble;
mod delta;
mod document;
mod health;
mod hydration;
mod portable;
mod scan;
pub(crate) mod scan_control;
mod scan_sources;
mod watcher;

pub(crate) use delta::{compute_delta, CatalogDelta, CatalogDeltaDto};
pub(crate) use document::{load_sanitized_cache, load_sanitized_document};
pub(crate) use hydration::enqueue_hydration;
pub(crate) use scan::run_coordinated_scan;
pub(crate) use watcher::restart_change_watcher;

use crate::catalog::cache::CatalogCache;
use crate::catalog::scan_settings::ScanSettings;
use crate::catalog::ScanProgress;
use std::time::Instant;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum SyncRequest {
    Watch,
    Startup,
    Refresh,
    Force,
}

impl SyncRequest {
    pub(crate) fn is_interactive(self) -> bool {
        matches!(self, Self::Refresh | Self::Force)
    }

    fn label(self) -> &'static str {
        match self {
            Self::Watch => "watch",
            Self::Startup => "startup",
            Self::Refresh => "refresh",
            Self::Force => "force",
        }
    }
}

pub(crate) fn synchronize(
    previous: &CatalogCache,
    settings: &ScanSettings,
    request: SyncRequest,
    progress: impl Fn(ScanProgress),
    is_cancelled: impl Fn() -> bool + Sync,
) -> CatalogCache {
    let started_at = Instant::now();
    let attempted_at = health::seconds_since_epoch();
    let scan = scan_sources::scan_all(previous, settings, request, &progress, &is_cancelled);
    assemble::assemble(previous, scan, settings, request, started_at, attempted_at)
}

#[cfg(test)]
fn app(id: &str, name: &str) -> crate::catalog::AppInfo {
    use crate::catalog::{AppCategory, AppInfo, LaunchKind, SourceKind};
    AppInfo {
        id: id.into(),
        name: name.into(),
        path: format!(r"C:\{name}.exe"),
        icon_base64: None,
        artifact_kind: Default::default(),
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
        target_availability: None,
        category_reasons: Vec::new(),
        close_risk: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::source::{merge_sources, SourceKey, SourceSnapshot};
    use crate::catalog::sync::scan_control::{ScanControl, StageStop};
    use crate::catalog::StartMenuScan;
    use crate::catalog::{self, LaunchKind, SourceKind};

    #[test]
    fn a_cancelled_scan_reports_windows_sources_as_incomplete_not_empty() {
        let cancelled = || true;
        let control = ScanControl::new(&cancelled);

        let registry = catalog::scan_registry(&control);
        let start_menu = catalog::scan_start_menu(&control);
        let start_apps = catalog::start_apps::scan(&control);

        assert!(registry.apps.is_empty());
        assert_eq!(registry.stop, Some(StageStop::Cancelled));
        assert!(start_menu.apps.is_empty());
        assert_eq!(start_menu.stop, Some(StageStop::Cancelled));
        assert_eq!(start_apps, None);
    }

    #[test]
    fn an_incomplete_source_keeps_its_previous_snapshot() {
        let previous = vec![SourceSnapshot {
            key: SourceKey(catalog::source::START_MENU_SOURCE.into()),
            fingerprint: None,
            health: None,
            apps: vec![app("shortcut", "Editor")],
        }];
        let incomplete = StartMenuScan {
            apps: Vec::new(),
            stop: Some(StageStop::TimedOut),
            complete: false,
        };

        let mut updates = Vec::new();
        if incomplete.stop.is_none() {
            updates.push(SourceSnapshot {
                key: SourceKey(catalog::source::START_MENU_SOURCE.into()),
                fingerprint: None,
                health: None,
                apps: incomplete.apps,
            });
        }
        let merged = merge_sources(previous, updates);

        assert_eq!(merged.apps.len(), 1);
        assert_eq!(merged.apps[0].name, "Editor");
    }

    #[test]
    fn post_metadata_sanitize_collapses_shortcut_registry_duplicates() {
        let mut shortcut = app("firefox-shortcut", "Firefox");
        shortcut.path = r"C:\Menu\Firefox.lnk".into();
        shortcut.launch_kind = LaunchKind::Shortcut;
        shortcut.source_kind = SourceKind::StartMenu;
        shortcut.resolved_path = Some(r"C:\Program Files\Mozilla Firefox\firefox.exe".into());
        shortcut.publisher = Some("Mozilla Foundation".into());
        shortcut.install_location = Some(r"C:\Users\Example\Desktop".into());
        let mut registry = app("firefox-registry", "Mozilla Firefox (x64 ru)");
        registry.path = r"C:\Program Files\Mozilla Firefox\firefox.exe".into();
        registry.publisher = Some("Mozilla".into());
        registry.install_location = Some(r"C:\Program Files\Mozilla Firefox".into());

        let apps = catalog::sanitize(vec![shortcut, registry]);

        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].name, "Firefox");
    }
}
