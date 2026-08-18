use crate::catalog::{icon_cache, icon_source_candidates, AppInfo, LaunchKind, SourceKind};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use std::path::{Path, PathBuf};

#[derive(Default)]
pub(super) struct HydratedIcon {
    pub(super) data_url: Option<String>,
    pub(super) written_fingerprint: Option<String>,
}

pub(super) fn hydrate_icon(app_data_dir: &Path, app: &AppInfo) -> HydratedIcon {
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
}
