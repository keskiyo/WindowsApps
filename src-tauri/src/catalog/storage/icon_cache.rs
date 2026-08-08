use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

fn icons_dir(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("icons")
}

fn cache_key(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}

const CACHE_KEY_LENGTH: usize = 64;

fn legacy_cache_key(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric() || *character == '-')
        .collect()
}

fn icon_path(app_data_dir: &Path, app_id: &str, fingerprint: &str) -> PathBuf {
    icons_dir(app_data_dir).join(format!("{}-{fingerprint}.png", cache_key(app_id)))
}

const MAX_ICON_BYTES: u64 = 4 * 1024 * 1024;

pub(crate) fn read_icon(app_data_dir: &Path, app_id: &str, fingerprint: &str) -> Option<Vec<u8>> {
    let path = icon_path(app_data_dir, app_id, fingerprint);
    let metadata = fs::metadata(&path).ok()?;
    if !metadata.is_file() || metadata.len() > MAX_ICON_BYTES {
        return None;
    }
    fs::read(path).ok()
}

pub(crate) fn write_icon(
    app_data_dir: &Path,
    app_id: &str,
    fingerprint: &str,
    bytes: &[u8],
) -> io::Result<()> {
    let directory = icons_dir(app_data_dir);
    fs::create_dir_all(&directory)?;
    let destination = icon_path(app_data_dir, app_id, fingerprint);
    let temporary = destination.with_extension("png.tmp");
    fs::write(&temporary, bytes)?;
    fs::rename(temporary, &destination)?;
    Ok(())
}

pub(crate) fn sweep_superseded(app_data_dir: &Path, written: &[(String, String)]) {
    if written.is_empty() {
        return;
    }
    let directory = icons_dir(app_data_dir);
    let Ok(entries) = fs::read_dir(&directory) else {
        return;
    };
    let mut current = std::collections::HashMap::<String, HashSet<String>>::new();
    let mut legacy_prefixes = Vec::new();
    for (app_id, fingerprint) in written {
        current
            .entry(cache_key(app_id))
            .or_default()
            .insert(format!("{fingerprint}.png"));
        let legacy = legacy_cache_key(app_id);
        if !legacy.is_empty() {
            legacy_prefixes.push(format!("{legacy}-"));
        }
    }
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        let Some(name) = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
        else {
            continue;
        };
        let Some((key, rest)) = name.split_once('-') else {
            continue;
        };
        let superseded = match current.get(key) {
            Some(live) => !live.contains(rest),
            None => legacy_prefixes
                .iter()
                .any(|prefix| name.starts_with(prefix.as_str())),
        };
        if superseded {
            let _ = fs::remove_file(path);
        }
    }
}

pub(crate) fn source_fingerprint(source: &str) -> String {
    let path = Path::new(source);
    let metadata = fs::metadata(path).ok();
    let size = metadata.as_ref().map_or(0, fs::Metadata::len);
    let modified = metadata
        .and_then(|metadata| metadata.modified().ok())
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |duration| duration.as_nanos());
    let identity = format!("{}|{size}|{modified}", source.to_lowercase());
    format!("{:x}", Sha256::digest(identity.as_bytes()))
}

pub(crate) fn retain_only(app_data_dir: &Path, live_ids: &[String]) {
    let directory = icons_dir(app_data_dir);
    let Ok(entries) = fs::read_dir(&directory) else {
        return;
    };
    let live = live_ids
        .iter()
        .map(|id| cache_key(id))
        .collect::<std::collections::HashSet<_>>();
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        let Some(name) = path.file_name().map(|name| name.to_string_lossy()) else {
            continue;
        };
        let Some((key, _)) = name.split_once('-') else {
            continue;
        };
        if key.len() == CACHE_KEY_LENGTH && !live.contains(key) {
            let _ = fs::remove_file(path);
        }
    }
}

pub(crate) fn clear(app_data_dir: &Path) -> io::Result<()> {
    let directory = icons_dir(app_data_dir);
    if directory.exists() {
        fs::remove_dir_all(directory)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_png_bytes_by_app_and_fingerprint() {
        let dir = tempfile::tempdir().unwrap();

        write_icon(dir.path(), "editor", "abc", b"png-data").unwrap();

        assert_eq!(
            read_icon(dir.path(), "editor", "abc"),
            Some(b"png-data".to_vec())
        );
        assert_eq!(read_icon(dir.path(), "editor", "different"), None);
    }

    #[test]
    fn applications_under_non_latin_paths_keep_separate_icons() {
        let dir = tempfile::tempdir().unwrap();
        let first = r"D:\Программы\Редактор.exe";
        let second = r"D:\Файлы\Просмотр.exe";
        assert_eq!(legacy_cache_key(first), legacy_cache_key(second));

        write_icon(dir.path(), first, "fp1", b"first").unwrap();
        write_icon(dir.path(), second, "fp2", b"second").unwrap();

        assert_eq!(read_icon(dir.path(), first, "fp1"), Some(b"first".to_vec()));
        assert_eq!(
            read_icon(dir.path(), second, "fp2"),
            Some(b"second".to_vec())
        );
    }

    #[test]
    fn separator_only_differences_do_not_share_a_key() {
        assert_eq!(
            legacy_cache_key(r"C:\Tools\Git\bin\git.exe"),
            legacy_cache_key(r"C:\Tools\Gitbin\git.exe")
        );
        assert_ne!(
            cache_key(r"C:\Tools\Git\bin\git.exe"),
            cache_key(r"C:\Tools\Gitbin\git.exe")
        );
    }

    #[test]
    fn a_new_icon_replaces_the_file_left_by_the_previous_naming_scheme() {
        let dir = tempfile::tempdir().unwrap();
        let app_id = r"C:\Editor.exe";
        let icons = icons_dir(dir.path());
        std::fs::create_dir_all(&icons).unwrap();
        let stale = icons.join(format!("{}-old.png", legacy_cache_key(app_id)));
        std::fs::write(&stale, b"legacy").unwrap();

        write_icon(dir.path(), app_id, "fp", b"current").unwrap();
        sweep_superseded(dir.path(), &[(app_id.to_string(), "fp".to_string())]);

        assert!(!stale.exists());
        assert_eq!(
            read_icon(dir.path(), app_id, "fp"),
            Some(b"current".to_vec())
        );
    }

    #[test]
    fn one_batch_sweep_drops_every_superseded_fingerprint() {
        let dir = tempfile::tempdir().unwrap();
        let app_id = r"C:\Editor.exe";
        write_icon(dir.path(), app_id, "old", b"old").unwrap();
        write_icon(dir.path(), app_id, "new", b"new").unwrap();

        sweep_superseded(dir.path(), &[(app_id.to_string(), "new".to_string())]);

        assert_eq!(read_icon(dir.path(), app_id, "old"), None);
        assert_eq!(read_icon(dir.path(), app_id, "new"), Some(b"new".to_vec()));
    }

    #[test]
    fn a_batch_sweep_leaves_applications_outside_the_batch_alone() {
        let dir = tempfile::tempdir().unwrap();
        let swept = r"C:\Swept.exe";
        let untouched = r"C:\Untouched.exe";
        write_icon(dir.path(), swept, "old", b"old").unwrap();
        write_icon(dir.path(), swept, "new", b"new").unwrap();
        write_icon(dir.path(), untouched, "fp", b"kept").unwrap();

        sweep_superseded(dir.path(), &[(swept.to_string(), "new".to_string())]);

        assert_eq!(read_icon(dir.path(), swept, "old"), None);
        assert_eq!(
            read_icon(dir.path(), untouched, "fp"),
            Some(b"kept".to_vec())
        );
    }

    #[test]
    fn an_oversized_cache_file_is_skipped_instead_of_being_read() {
        let dir = tempfile::tempdir().unwrap();
        let app_id = r"C:\Editor.exe";
        let icons = icons_dir(dir.path());
        std::fs::create_dir_all(&icons).unwrap();
        let path = icon_path(dir.path(), app_id, "fp");
        let file = std::fs::File::create(&path).unwrap();
        file.set_len(MAX_ICON_BYTES + 1).unwrap();
        drop(file);

        assert_eq!(read_icon(dir.path(), app_id, "fp"), None);
        assert_eq!(std::fs::metadata(&path).unwrap().len(), MAX_ICON_BYTES + 1);
    }

    #[test]
    fn a_cache_entry_that_is_a_directory_is_not_read() {
        let dir = tempfile::tempdir().unwrap();
        let app_id = r"C:\Editor.exe";
        std::fs::create_dir_all(icon_path(dir.path(), app_id, "fp")).unwrap();

        assert_eq!(read_icon(dir.path(), app_id, "fp"), None);
    }

    #[test]
    fn an_icon_at_the_size_limit_is_still_read() {
        let dir = tempfile::tempdir().unwrap();
        let app_id = r"C:\Editor.exe";
        write_icon(dir.path(), app_id, "fp", b"small").unwrap();

        assert_eq!(read_icon(dir.path(), app_id, "fp"), Some(b"small".to_vec()));
    }

    #[test]
    fn source_fingerprint_changes_with_file_size() {
        let dir = tempfile::tempdir().unwrap();
        let executable = dir.path().join("Editor.exe");
        std::fs::write(&executable, [1]).unwrap();
        let first = source_fingerprint(&executable.to_string_lossy());
        std::fs::write(&executable, [1, 2]).unwrap();
        let second = source_fingerprint(&executable.to_string_lossy());

        assert_ne!(first, second);
    }

    #[test]
    fn icons_of_applications_no_longer_in_the_catalog_are_dropped() {
        let dir = tempfile::tempdir().unwrap();
        let kept = r"C:\Kept.exe";
        let removed = r"C:\Removed.exe";
        write_icon(dir.path(), kept, "fp1", b"kept").unwrap();
        write_icon(dir.path(), removed, "fp2", b"removed").unwrap();

        retain_only(dir.path(), &[kept.to_string()]);

        assert_eq!(read_icon(dir.path(), kept, "fp1"), Some(b"kept".to_vec()));
        assert_eq!(read_icon(dir.path(), removed, "fp2"), None);
    }

    #[test]
    fn retaining_leaves_unrecognized_file_names_alone() {
        let dir = tempfile::tempdir().unwrap();
        let icons = icons_dir(dir.path());
        std::fs::create_dir_all(&icons).unwrap();
        let legacy = icons.join("CEditorexe-old.png");
        std::fs::write(&legacy, b"legacy").unwrap();

        retain_only(dir.path(), &[]);

        assert!(legacy.exists());
    }

    #[test]
    fn clear_removes_icon_directory() {
        let dir = tempfile::tempdir().unwrap();
        write_icon(dir.path(), "editor", "abc", b"png-data").unwrap();

        clear(dir.path()).unwrap();

        assert!(!icons_dir(dir.path()).exists());
    }
}
