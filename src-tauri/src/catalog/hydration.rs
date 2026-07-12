use crate::catalog::{icon_cache, icon_source_candidates, AppInfo, LaunchKind, SourceKind};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde::Serialize;
use std::collections::{HashSet, VecDeque};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppHydrationPatch {
    pub id: String,
    pub generation: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_base64: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publisher: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_filename: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub install_location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub can_uninstall: Option<bool>,
}

pub fn hydrate_one(app_data_dir: &Path, app: &AppInfo, generation: u64) -> AppHydrationPatch {
    hydrate_app(app_data_dir, app, generation)
}

#[derive(Default)]
pub struct HydrationQueue {
    generation: u64,
    foreground: VecDeque<String>,
    background: VecDeque<String>,
    queued: HashSet<String>,
    running: bool,
}

impl HydrationQueue {
    pub fn enqueue(
        &mut self,
        generation: u64,
        ids: impl IntoIterator<Item = String>,
        priority: bool,
    ) -> bool {
        if self.generation != generation {
            self.generation = generation;
            self.foreground.clear();
            self.background.clear();
            self.queued.clear();
            self.running = false;
        }
        for id in ids {
            if !self.queued.insert(id.clone()) {
                if priority {
                    let was_background = self.background.iter().any(|queued| queued == &id);
                    self.background.retain(|queued| queued != &id);
                    if was_background && !self.foreground.iter().any(|queued| queued == &id) {
                        self.foreground.push_back(id);
                    }
                }
                continue;
            }
            if priority {
                self.foreground.push_back(id);
            } else {
                self.background.push_back(id);
            }
        }
        if self.running || self.queued.is_empty() {
            false
        } else {
            self.running = true;
            true
        }
    }

    pub fn pop(&mut self, generation: u64) -> Option<String> {
        if self.generation != generation {
            return None;
        }
        self.foreground
            .pop_front()
            .or_else(|| self.background.pop_front())
    }

    pub fn complete(&mut self, generation: u64, id: &str) {
        if self.generation == generation {
            self.queued.remove(id);
        }
    }

    pub fn finish(&mut self, generation: u64) {
        if self.generation == generation {
            self.running = false;
        }
    }
}

fn hydrate_app(app_data_dir: &Path, app: &AppInfo, generation: u64) -> AppHydrationPatch {
    let target = app.resolved_path.as_deref().unwrap_or(&app.path);
    let metadata = Path::new(target)
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
        .then(|| crate::platform::windows::executable_metadata::read(Path::new(target)));
    let icon_base64 = hydrate_icon(app_data_dir, app);
    AppHydrationPatch {
        id: app.id.clone(),
        generation,
        icon_base64,
        description: app.description.clone().or_else(|| {
            metadata
                .as_ref()
                .and_then(|value| value.description.clone())
        }),
        version: app
            .version
            .clone()
            .or_else(|| metadata.as_ref().and_then(|value| value.version.clone())),
        publisher: app
            .publisher
            .clone()
            .or_else(|| metadata.as_ref().and_then(|value| value.publisher.clone())),
        product_name: app.product_name.clone().or_else(|| {
            metadata
                .as_ref()
                .and_then(|value| value.product_name.clone())
        }),
        original_filename: app.original_filename.clone().or_else(|| {
            metadata
                .as_ref()
                .and_then(|value| value.original_filename.clone())
        }),
        install_location: app.install_location.clone().or_else(|| {
            Path::new(target)
                .parent()
                .map(|path| path.to_string_lossy().into_owned())
        }),
        can_uninstall: Some(app.uninstall.is_some()),
    }
}

fn hydrate_icon(app_data_dir: &Path, app: &AppInfo) -> Option<String> {
    // Walk the icon-source candidates (shortcut icon file → resolved target → path) until
    // one yields an icon; a shortcut whose declared .ico fails should still get the icon
    // embedded in its target executable.
    let mut candidates = icon_source_candidates(app);
    if candidates.is_empty() {
        candidates.push(app.path.clone());
    }
    for source in candidates {
        let fingerprint = icon_cache::source_fingerprint(&source);
        if let Some(bytes) = icon_cache::read_icon(app_data_dir, &app.id, &fingerprint) {
            return Some(format!("data:image/png;base64,{}", STANDARD.encode(bytes)));
        }
        let Some(data_url) = extract_icon_from_source(app, &source) else {
            continue;
        };
        if let Some((_, encoded)) = data_url.split_once(',') {
            if let Ok(bytes) = STANDARD.decode(encoded) {
                let _ = icon_cache::write_icon(app_data_dir, &app.id, &fingerprint, &bytes);
            }
        }
        return Some(data_url);
    }
    None
}

fn extract_icon_from_source(app: &AppInfo, source: &str) -> Option<String> {
    if app.launch_kind == LaunchKind::AppUserModelId {
        return crate::platform::windows::icon_extractor::extract_app_id_icon(&app.path);
    }
    if app.source_kind == SourceKind::Steam {
        // Steam game executables often lack an embedded icon; prefer Steam's own
        // library-cache icon and only fall back to the resolved executable.
        if let Some(icon) = steam_library_icon(app) {
            return Some(icon);
        }
    }
    let path = Path::new(source);
    // Shortcut icon locations frequently point at loose image files (.ico and friends).
    // SHGetFileInfoW returns the file-class icon for those, not their content — decode
    // the image directly instead.
    if is_image_file(path) {
        if let Some(icon) =
            crate::platform::windows::icon_extractor::image_file_to_png_data_url(path)
        {
            return Some(icon);
        }
    }
    crate::platform::windows::icon_extractor::extract_icon(path)
}

/// Image formats the bundled `image` crate can decode (see Cargo.toml features).
fn is_image_file(path: &Path) -> bool {
    path.extension().is_some_and(|extension| {
        ["ico", "png", "jpg", "jpeg"]
            .iter()
            .any(|value| extension.eq_ignore_ascii_case(value))
    })
}

/// Resolve a Steam game's icon from `<SteamPath>/appcache/librarycache`.
fn steam_library_icon(app: &AppInfo) -> Option<String> {
    let app_id = app.path.strip_prefix("steam://rungameid/")?;
    let cache = super::steam::steam_root()?
        .join("appcache")
        .join("librarycache");
    let candidate = steam_icon_file(&cache, app_id)?;
    crate::platform::windows::icon_extractor::image_file_to_png_data_url(&candidate)
}

/// Locate the icon image for a Steam app inside `librarycache`.
/// Legacy Steam stored `<appid>_icon.jpg` directly; modern Steam stores per-app folders
/// `<appid>/<sha1hash>.jpg`, where the SHA1-named image is the client icon (the descriptive
/// `library_*.jpg`/`header.jpg`/`logo.png` files are large store art, not the icon).
fn steam_icon_file(librarycache: &Path, app_id: &str) -> Option<PathBuf> {
    let legacy = librarycache.join(format!("{app_id}_icon.jpg"));
    if legacy.is_file() {
        return Some(legacy);
    }
    let mut hashed = std::fs::read_dir(librarycache.join(app_id))
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| is_hashed_image(path))
        .collect::<Vec<_>>();
    hashed.sort();
    hashed.into_iter().next()
}

/// True for SHA1-hash-named image files (`<40 hex>.jpg|jpeg|png`) — Steam's icon naming.
fn is_hashed_image(path: &Path) -> bool {
    let extension_ok = path.extension().is_some_and(|extension| {
        ["jpg", "jpeg", "png"]
            .iter()
            .any(|value| extension.eq_ignore_ascii_case(value))
    });
    let stem_is_hash = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .is_some_and(|stem| {
            stem.len() == 40 && stem.chars().all(|character| character.is_ascii_hexdigit())
        });
    extension_ok && stem_is_hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_sha1_named_icon_from_modern_steam_layout() {
        let dir = tempfile::tempdir().unwrap();
        let cache = dir.path();
        let app_folder = cache.join("1623730");
        std::fs::create_dir_all(&app_folder).unwrap();
        let icon = app_folder.join("f5523077a8f4c923c2e8d8c17794b3319035fa73.jpg");
        for name in [
            "f5523077a8f4c923c2e8d8c17794b3319035fa73.jpg",
            "library_600x900.jpg",
            "library_header.jpg",
            "logo.png",
        ] {
            std::fs::write(app_folder.join(name), []).unwrap();
        }

        assert_eq!(steam_icon_file(cache, "1623730"), Some(icon));
    }

    #[test]
    fn prefers_legacy_appid_icon_when_present() {
        let dir = tempfile::tempdir().unwrap();
        let cache = dir.path();
        let legacy = cache.join("13180_icon.jpg");
        std::fs::write(&legacy, []).unwrap();

        assert_eq!(steam_icon_file(cache, "13180"), Some(legacy));
    }

    #[test]
    fn returns_none_when_only_store_art_exists() {
        let dir = tempfile::tempdir().unwrap();
        let cache = dir.path();
        let app_folder = cache.join("999");
        std::fs::create_dir_all(&app_folder).unwrap();
        for name in ["header.jpg", "library_hero.jpg", "logo.png"] {
            std::fs::write(app_folder.join(name), []).unwrap();
        }

        assert_eq!(steam_icon_file(cache, "999"), None);
    }

    #[test]
    fn queue_prioritizes_visible_ids_and_deduplicates_requests() {
        let mut queue = HydrationQueue::default();
        assert!(queue.enqueue(1, ["a".into(), "b".into(), "c".into()], false));
        assert!(!queue.enqueue(1, ["c".into(), "b".into()], true));
        assert_eq!(queue.pop(1).as_deref(), Some("c"));
        queue.complete(1, "c");
        assert_eq!(queue.pop(1).as_deref(), Some("b"));
        queue.complete(1, "b");
        assert_eq!(queue.pop(1).as_deref(), Some("a"));
    }

    #[test]
    fn new_generation_discards_stale_hydration_work() {
        let mut queue = HydrationQueue::default();
        assert!(queue.enqueue(1, ["old".into()], false));
        assert!(queue.enqueue(2, ["new".into()], true));
        assert_eq!(queue.pop(1), None);
        assert_eq!(queue.pop(2).as_deref(), Some("new"));
    }

    #[test]
    fn visible_request_does_not_duplicate_an_in_flight_id() {
        let mut queue = HydrationQueue::default();
        assert!(queue.enqueue(1, ["app".into()], false));
        assert_eq!(queue.pop(1).as_deref(), Some("app"));
        assert!(!queue.enqueue(1, ["app".into()], true));
        queue.complete(1, "app");
        assert_eq!(queue.pop(1), None);
    }
}
