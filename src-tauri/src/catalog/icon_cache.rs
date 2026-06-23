use sha2::{Digest, Sha256};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

fn icons_dir(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("icons")
}

fn safe_id(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric() || *character == '-')
        .collect()
}

fn icon_path(app_data_dir: &Path, app_id: &str, fingerprint: &str) -> PathBuf {
    icons_dir(app_data_dir).join(format!("{}-{fingerprint}.png", safe_id(app_id)))
}

pub fn read_icon(app_data_dir: &Path, app_id: &str, fingerprint: &str) -> Option<Vec<u8>> {
    fs::read(icon_path(app_data_dir, app_id, fingerprint)).ok()
}

pub fn write_icon(
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
    let prefix = format!("{}-", safe_id(app_id));
    for entry in fs::read_dir(directory)?.filter_map(Result::ok) {
        let path = entry.path();
        if path != destination
            && path
                .file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with(&prefix))
        {
            let _ = fs::remove_file(path);
        }
    }
    Ok(())
}

pub fn source_fingerprint(source: &str) -> String {
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

pub fn clear(app_data_dir: &Path) -> io::Result<()> {
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
    fn clear_removes_icon_directory() {
        let dir = tempfile::tempdir().unwrap();
        write_icon(dir.path(), "editor", "abc", b"png-data").unwrap();

        clear(dir.path()).unwrap();

        assert!(!icons_dir(dir.path()).exists());
    }
}
