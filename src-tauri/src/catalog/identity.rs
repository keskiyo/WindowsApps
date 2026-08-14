use super::{filters, naming};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub(super) fn stable_id(identity: &str) -> String {
    format!(
        "{:x}",
        Sha256::digest(identity.trim().to_lowercase().as_bytes())
    )
}

pub(super) fn find_executable(location: &str) -> Option<PathBuf> {
    find_executable_named(location, None)
}

pub(super) fn find_executable_named(location: &str, name: Option<&str>) -> Option<PathBuf> {
    let root = PathBuf::from(location.trim().trim_matches('"'));
    if is_launchable(&root) {
        return Some(root);
    }
    if !root.is_dir() {
        return None;
    }
    let target = name
        .map(naming::normalized_portable_name)
        .filter(|key| !key.is_empty());
    WalkDir::new(&root)
        .max_depth(2)
        .into_iter()
        .filter_map(Result::ok)
        .map(|entry| entry.into_path())
        .filter(|path| {
            is_launchable(path) && !filters::is_maintenance_path(&path.to_string_lossy())
        })
        .min_by_key(|path| {
            let stem = path
                .file_stem()
                .map(|value| naming::normalized_portable_name(&value.to_string_lossy()))
                .unwrap_or_default();
            let name_score = match &target {
                Some(target) if stem == *target => 0u8,
                Some(target)
                    if !stem.is_empty() && (stem.contains(target) || target.contains(&stem)) =>
                {
                    1
                }
                Some(_) => 3,
                None => 2,
            };
            let depth = path
                .strip_prefix(&root)
                .map(|relative| relative.components().count())
                .unwrap_or(usize::MAX);
            (name_score, depth, path.to_string_lossy().into_owned())
        })
}

pub(crate) fn path_is_within(path: &str, root: &str) -> bool {
    let root = root.trim_end_matches(['\\', '/']);
    if root.is_empty() {
        return false;
    }
    let Some(rest) = path.strip_prefix(root) else {
        return false;
    };
    rest.is_empty() || rest.starts_with('\\') || rest.starts_with('/')
}

pub(super) fn is_launchable(path: &Path) -> bool {
    path.is_file()
        && path.extension().is_some_and(|extension| {
            extension.eq_ignore_ascii_case("exe") || extension.eq_ignore_ascii_case("lnk")
        })
}

#[cfg(test)]
mod tests {
    use super::{find_executable, find_executable_named, path_is_within, stable_id};

    #[test]
    fn stable_ids_ignore_windows_path_case() {
        assert_eq!(
            stable_id(r"C:\Apps\Codex.exe"),
            stable_id(r"c:\apps\CODEX.exe")
        );
    }

    #[test]
    fn path_containment_stops_at_component_boundaries() {
        assert!(path_is_within(r"c:\prog\app.exe", r"c:\prog"));
        assert!(path_is_within(r"c:\prog\app.exe", r"c:\prog\"));
        assert!(path_is_within(r"c:\prog", r"c:\prog"));
        assert!(path_is_within("c:/prog/app.exe", "c:/prog"));
        assert!(!path_is_within(r"c:\program files\app.exe", r"c:\prog"));
        assert!(!path_is_within(r"c:\progbackup", r"c:\prog"));
        assert!(!path_is_within(r"c:\prog\app.exe", ""));
        assert!(!path_is_within(r"c:\prog\app.exe", r"\"));
    }

    #[test]
    fn finds_executable_inside_registered_install_location() {
        let dir = tempfile::tempdir().unwrap();
        let executable = dir.path().join("Warhammer 40000 Space Marine 2.exe");
        std::fs::write(&executable, []).unwrap();

        assert_eq!(
            find_executable(&dir.path().to_string_lossy()),
            Some(executable)
        );
    }

    #[test]
    fn prefers_named_executable_over_bundled_helpers() {
        let dir = tempfile::tempdir().unwrap();
        let main = dir.path().join("Docker Desktop.exe");
        let bundled = dir.path().join("courgette64.exe");
        std::fs::write(&bundled, []).unwrap();
        std::fs::write(&main, []).unwrap();

        assert_eq!(
            find_executable_named(&dir.path().to_string_lossy(), Some("Docker Desktop")),
            Some(main)
        );
    }
}
