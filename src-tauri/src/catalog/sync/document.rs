//! Reading the on-disk catalog as a sanitized document, with a generation guard so a slow
//! sanitize write-back never clobbers a fresher catalog written by a concurrent scan.

use crate::catalog::cache::{self, CatalogCache};
use crate::catalog::{self, AppInfo};
use std::path::Path;

pub(crate) fn load_sanitized_document(app_data_dir: &Path) -> Option<CatalogCache> {
    let mut document = cache::read_document(app_data_dir)?;
    let original = document.apps.clone();
    document.apps = catalog::sanitize(document.apps);
    document.app_details =
        crate::catalog::details::retain_cached_details(&document.app_details, &document.apps);
    for app in &mut document.apps {
        app.can_uninstall = app.uninstall.is_some();
        app.icon_base64 = None;
    }
    if document.apps != original && !newer_generation_on_disk(app_data_dir, document.generation) {
        let _ = cache::write_document(app_data_dir, &document);
    }
    Some(document)
}

/// This read-modify-write cannot take `sync_lock`: `synchronize_catalog_once` already holds it
/// when it calls in, so acquiring it here would self-deadlock. Sanitizing a large catalog takes
/// long enough for a concurrent scan to finish and write a newer generation, and `get_apps`
/// reaches this path without the lock at all — so re-check the generation on disk and skip the
/// write rather than replacing a fresher catalog with a stale one.
fn newer_generation_on_disk(app_data_dir: &Path, generation: u64) -> bool {
    cache::read_document(app_data_dir).is_some_and(|current| current.generation > generation)
}

pub(crate) fn load_sanitized_cache(app_data_dir: &Path) -> Option<Vec<AppInfo>> {
    load_sanitized_document(app_data_dir).map(|document| document.apps)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_state::cached_app;
    use crate::catalog::cache::CachedAppDetails;
    use crate::catalog::AppDetails;
    use std::collections::BTreeMap;

    #[test]
    fn loads_and_persists_a_sanitized_cache() {
        let dir = tempfile::tempdir().unwrap();
        cache::write_document(
            dir.path(),
            &CatalogCache {
                apps: vec![
                    cached_app(
                        "Visual Studio Installer",
                        "Microsoft.VisualStudio.Installer",
                    ),
                    cached_app("Visual Studio Code", r"C:\Code.exe"),
                ],
                ..CatalogCache::default()
            },
        )
        .unwrap();
        let apps = load_sanitized_cache(dir.path()).unwrap();
        assert_eq!(apps.len(), 1);
        assert_eq!(cache::read_document(dir.path()).unwrap().apps.len(), 1);
    }

    // The sanitize write-back runs outside `sync_lock`, so a scan that finished meanwhile must
    // not be overwritten with the older catalog this call started from.
    #[test]
    fn sanitize_write_back_does_not_replace_a_newer_generation() {
        let dir = tempfile::tempdir().unwrap();
        cache::write_document(
            dir.path(),
            &CatalogCache {
                generation: 9,
                apps: vec![cached_app("Editor", r"C:\Editor.exe")],
                ..CatalogCache::default()
            },
        )
        .unwrap();

        assert!(newer_generation_on_disk(dir.path(), 8));
        assert!(!newer_generation_on_disk(dir.path(), 9));
        assert!(!newer_generation_on_disk(dir.path(), 10));
    }

    #[test]
    fn sanitize_write_back_is_skipped_when_the_cache_moved_ahead() {
        let dir = tempfile::tempdir().unwrap();
        // A document that sanitizing would change (the installer entry is dropped), but carrying
        // an older generation than what is already on disk.
        let stale = CatalogCache {
            generation: 1,
            apps: vec![
                cached_app(
                    "Visual Studio Installer",
                    "Microsoft.VisualStudio.Installer",
                ),
                cached_app("Editor", r"C:\Editor.exe"),
            ],
            ..CatalogCache::default()
        };
        cache::write_document(
            dir.path(),
            &CatalogCache {
                generation: 7,
                apps: vec![cached_app("Editor", r"C:\Editor.exe")],
                ..CatalogCache::default()
            },
        )
        .unwrap();

        assert!(newer_generation_on_disk(dir.path(), stale.generation));
        // The newer document on disk is still intact.
        assert_eq!(cache::read_document(dir.path()).unwrap().generation, 7);
    }

    #[test]
    fn disables_stale_uninstall_flag_without_a_cached_target() {
        let dir = tempfile::tempdir().unwrap();
        let mut app = cached_app("Editor", r"C:\Editor.exe");
        app.can_uninstall = true;
        cache::write_document(
            dir.path(),
            &CatalogCache {
                apps: vec![app],
                ..CatalogCache::default()
            },
        )
        .unwrap();
        let apps = load_sanitized_cache(dir.path()).unwrap();
        assert!(!apps[0].can_uninstall);
    }

    #[test]
    fn sanitize_write_back_removes_details_for_removed_cards_only() {
        let dir = tempfile::tempdir().unwrap();
        let mut removed = cached_app(
            "Visual Studio Installer",
            "Microsoft.VisualStudio.Installer",
        );
        removed.id = "removed".into();
        let mut kept = cached_app("Editor", r"C:\Editor.exe");
        let kept_id = crate::catalog::sanitize(vec![kept.clone()])
            .pop()
            .unwrap()
            .id;
        kept.id = kept_id.clone();
        let cached = |fingerprint: &str| CachedAppDetails {
            fingerprint: fingerprint.into(),
            details: AppDetails::default(),
        };
        cache::write_document(
            dir.path(),
            &CatalogCache {
                apps: vec![removed, kept],
                app_details: BTreeMap::from([
                    ("removed".into(), cached("old")),
                    (kept_id.clone(), cached("current")),
                ]),
                ..CatalogCache::default()
            },
        )
        .unwrap();

        let document = load_sanitized_document(dir.path()).unwrap();

        assert!(document.app_details.contains_key(&kept_id));
        assert!(!document.app_details.contains_key("removed"));
    }
}
