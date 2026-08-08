use crate::catalog::{icon_cache, icon_source_candidates, AppInfo, LaunchKind, SourceKind};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde::Serialize;
use std::collections::{HashSet, VecDeque};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AppHydrationPatch {
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

pub(crate) struct HydrationOutcome {
    pub patch: AppHydrationPatch,
    pub written_icon: Option<(String, String)>,
}

pub(crate) fn hydrate_one(app_data_dir: &Path, app: &AppInfo, generation: u64) -> HydrationOutcome {
    hydrate_app(app_data_dir, app, generation)
}

#[derive(Default)]
pub(crate) struct HydrationQueue {
    generation: u64,
    foreground: VecDeque<String>,
    background: VecDeque<String>,
    queued: HashSet<String>,
    running: bool,
}

impl HydrationQueue {
    pub(crate) fn enqueue(
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

    pub(crate) fn pop(&mut self, generation: u64) -> Option<String> {
        if self.generation != generation {
            return None;
        }
        self.foreground
            .pop_front()
            .or_else(|| self.background.pop_front())
    }

    pub(crate) fn complete(&mut self, generation: u64, id: &str) {
        if self.generation == generation {
            self.queued.remove(id);
        }
    }

    pub(crate) fn finish(&mut self, generation: u64) {
        if self.generation == generation {
            self.running = false;
        }
    }
}

fn hydrate_app(app_data_dir: &Path, app: &AppInfo, generation: u64) -> HydrationOutcome {
    let target = app.resolved_path.as_deref().unwrap_or(&app.path);
    let metadata = Path::new(target)
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
        .then(|| crate::platform::windows::executable_metadata::read(Path::new(target)));
    let icon = hydrate_icon(app_data_dir, app);
    let icon_base64 = icon.data_url;
    let patch = AppHydrationPatch {
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
    };
    HydrationOutcome {
        patch,
        written_icon: icon
            .written_fingerprint
            .map(|fingerprint| (app.id.clone(), fingerprint)),
    }
}

#[derive(Default)]
struct HydratedIcon {
    data_url: Option<String>,
    written_fingerprint: Option<String>,
}

fn hydrate_icon(app_data_dir: &Path, app: &AppInfo) -> HydratedIcon {
    let mut candidates = icon_source_candidates(app);
    if candidates.is_empty() {
        candidates.push(app.path.clone());
    }
    for source in candidates {
        let fingerprint = icon_cache::source_fingerprint(&source);
        if let Some(bytes) = icon_cache::read_icon(app_data_dir, &app.id, &fingerprint) {
            let data_url = format!("data:image/png;base64,{}", STANDARD.encode(bytes));
            if !crate::platform::windows::icon_extractor::is_provably_not_this_files_icon(
                Path::new(&source),
                &data_url,
            ) {
                return HydratedIcon {
                    data_url: Some(data_url),
                    written_fingerprint: None,
                };
            }
        }
        let Some(data_url) = extract_icon_from_source(app, &source) else {
            continue;
        };
        let mut written_fingerprint = None;
        if let Some((_, encoded)) = data_url.split_once(',') {
            if let Ok(bytes) = STANDARD.decode(encoded) {
                if icon_cache::write_icon(app_data_dir, &app.id, &fingerprint, &bytes).is_ok() {
                    written_fingerprint = Some(fingerprint);
                }
            }
        }
        return HydratedIcon {
            data_url: Some(data_url),
            written_fingerprint,
        };
    }
    HydratedIcon::default()
}

fn extract_icon_from_source(app: &AppInfo, source: &str) -> Option<String> {
    if app.launch_kind == LaunchKind::AppUserModelId {
        return crate::platform::windows::icon_extractor::extract_app_id_icon(&app.path);
    }
    if app.source_kind == SourceKind::Steam {
        if let Some(icon) = steam_library_icon(app) {
            return Some(icon);
        }
    }
    let path = Path::new(source);
    if is_image_file(path) {
        if let Some(icon) =
            crate::platform::windows::icon_extractor::image_file_to_png_data_url(path)
        {
            return Some(icon);
        }
    }
    crate::platform::windows::icon_extractor::extract_icon(path)
}

fn is_image_file(path: &Path) -> bool {
    path.extension().is_some_and(|extension| {
        ["ico", "png", "jpg", "jpeg"]
            .iter()
            .any(|value| extension.eq_ignore_ascii_case(value))
    })
}

fn steam_library_icon(app: &AppInfo) -> Option<String> {
    let app_id = app.path.strip_prefix("steam://rungameid/")?;
    let cache = crate::platform::windows::steam_registry::install_root()?
        .join("appcache")
        .join("librarycache");
    let candidate = steam_icon_file(&cache, app_id)?;
    crate::platform::windows::icon_extractor::image_file_to_png_data_url(&candidate)
}

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
    fn a_cached_icon_the_shell_never_read_is_not_served_again() {
        let Some(placeholder) = crate::platform::windows::icon_extractor::unknown_type_icon()
        else {
            return;
        };
        for name in ["Editor.exe", "Editor.lnk"] {
            let dir = tempfile::tempdir().unwrap();
            let file = dir.path().join(name);
            std::fs::write(&file, b"a launch target that does exist").unwrap();
            let mut app = crate::app_state::cached_app("Editor", &file.to_string_lossy());
            app.id = "editor".into();
            let encoded = placeholder.split_once(',').expect("a data url").1;
            icon_cache::write_icon(
                dir.path(),
                &app.id,
                &icon_cache::source_fingerprint(&app.path),
                &STANDARD.decode(encoded).expect("the placeholder decodes"),
            )
            .expect("the icon cache is writable");

            let hydrated = hydrate_icon(dir.path(), &app);

            assert_ne!(
                hydrated.data_url.as_deref(),
                Some(placeholder.as_str()),
                "{name}"
            );
        }
    }

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
