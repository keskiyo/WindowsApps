use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::catalog) enum Place {
    TransientDrop,
    PackageCache,
    Unknown,
}

pub(in crate::catalog) struct PlaceIndex {
    roots: Vec<(String, Place)>,
}

const UNIVERSAL_MARKERS: &[(&str, Place)] = &[
    (r"\package cache\", Place::PackageCache),
    (r"\microsoft\onedrive\", Place::PackageCache),
    (r"\microsoft visual studio\installer\", Place::PackageCache),
    (r"\downloads\", Place::TransientDrop),
];

impl PlaceIndex {
    pub(in crate::catalog) fn current() -> Self {
        let folders = crate::platform::windows::known_folders::user_folders();
        Self::from_roots(
            [folders.downloads, folders.desktop, folders.temp]
                .into_iter()
                .flatten()
                .map(|folder| (folder, Place::TransientDrop))
                .collect(),
        )
    }

    pub(in crate::catalog) fn from_roots(roots: Vec<(PathBuf, Place)>) -> Self {
        let mut roots = roots
            .into_iter()
            .filter_map(|(path, place)| {
                let normalized = normalized_path(&path.to_string_lossy());
                (!normalized.is_empty()).then_some((normalized, place))
            })
            .collect::<Vec<_>>();
        roots.sort_by_key(|(root, _)| std::cmp::Reverse(root.len()));
        Self { roots }
    }

    pub(in crate::catalog) fn classify(&self, path: &str) -> Place {
        let normalized = normalized_path(path);
        if let Some((_, place)) = self
            .roots
            .iter()
            .find(|(root, _)| is_within(&normalized, root))
        {
            return *place;
        }
        UNIVERSAL_MARKERS
            .iter()
            .find(|(marker, _)| normalized.contains(marker))
            .map(|(_, place)| *place)
            .unwrap_or(Place::Unknown)
    }
}

pub(in crate::catalog) fn normalized_path(path: &str) -> String {
    path.trim()
        .trim_matches('"')
        .trim_end_matches('\\')
        .replace('/', r"\")
        .to_lowercase()
}

fn is_within(path: &str, root: &str) -> bool {
    let root = root.trim_end_matches('\\');
    path.strip_prefix(root)
        .is_some_and(|rest| rest.starts_with('\\'))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn index() -> PlaceIndex {
        PlaceIndex::from_roots(vec![
            (PathBuf::from(r"D:\Stuff"), Place::TransientDrop),
            (
                PathBuf::from(r"C:\Users\Kowalski\AppData\Local\Temp"),
                Place::TransientDrop,
            ),
        ])
    }

    #[test]
    fn a_redirected_download_folder_is_a_transient_drop() {
        assert_eq!(
            index().classify(r"D:\Stuff\vendor-setup.exe"),
            Place::TransientDrop
        );
        assert_eq!(
            index().classify(r"C:/Users/Kowalski/AppData/Local/Temp/x/setup.exe"),
            Place::TransientDrop
        );
    }

    #[test]
    fn windows_own_constants_hold_without_any_resolved_folder() {
        let empty = PlaceIndex::from_roots(Vec::new());

        assert_eq!(
            empty.classify(r"C:\ProgramData\Package Cache\{guid}\winsdksetup.exe"),
            Place::PackageCache
        );
        assert_eq!(
            empty.classify(r"C:\Users\Any\Downloads\thing.exe"),
            Place::TransientDrop
        );
        assert_eq!(
            empty.classify(r"C:\Program Files\Vendor\app.exe"),
            Place::Unknown
        );
    }

    #[test]
    fn a_root_does_not_swallow_a_similarly_named_neighbour() {
        assert_eq!(index().classify(r"D:\StuffBackup\app.exe"), Place::Unknown);
        assert_eq!(index().classify(r"D:\Stuff"), Place::Unknown);
    }
}
