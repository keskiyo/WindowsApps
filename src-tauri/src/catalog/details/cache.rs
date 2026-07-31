use crate::catalog::cache::{CachedAppDetails, CatalogCache};
use crate::catalog::{AppDetails, AppInfo};
use std::collections::{BTreeMap, HashSet};

pub(crate) fn read_cached_details(
    cache: &CatalogCache,
    app_id: &str,
    fingerprint: &str,
) -> Option<AppDetails> {
    cache
        .app_details
        .get(app_id)
        .filter(|cached| cached.fingerprint == fingerprint)
        .map(|cached| cached.details.clone())
}

pub(crate) fn retain_cached_details(
    cached: &BTreeMap<String, CachedAppDetails>,
    apps: &[AppInfo],
) -> BTreeMap<String, CachedAppDetails> {
    let live_ids = apps
        .iter()
        .map(|app| app.id.as_str())
        .collect::<HashSet<_>>();
    cached
        .iter()
        .filter(|(id, _)| live_ids.contains(id.as_str()))
        .map(|(id, details)| (id.clone(), details.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cached_details_are_used_only_for_the_current_file_fingerprint() {
        let details = AppDetails {
            file_size_bytes: Some(42),
            ..AppDetails::default()
        };
        let cache = CatalogCache {
            app_details: BTreeMap::from([(
                "editor".into(),
                CachedAppDetails {
                    fingerprint: "current".into(),
                    details: details.clone(),
                },
            )]),
            ..CatalogCache::default()
        };

        assert_eq!(
            read_cached_details(&cache, "editor", "current"),
            Some(details)
        );
        assert_eq!(read_cached_details(&cache, "editor", "changed"), None);
    }
}
