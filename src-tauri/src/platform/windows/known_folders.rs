use crate::platform::windows::com::CoTaskString;
use std::path::PathBuf;
use windows::core::GUID;
use windows::Win32::UI::Shell::{
    FOLDERID_Desktop, FOLDERID_Downloads, SHGetKnownFolderPath, KF_FLAG_DEFAULT,
};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct UserFolders {
    pub downloads: Option<PathBuf>,
    pub desktop: Option<PathBuf>,
    pub temp: Option<PathBuf>,
}

pub(crate) fn user_folders() -> UserFolders {
    UserFolders {
        downloads: known_folder(&FOLDERID_Downloads),
        desktop: known_folder(&FOLDERID_Desktop),
        temp: std::env::var_os("TEMP").map(PathBuf::from),
    }
}

fn known_folder(id: &GUID) -> Option<PathBuf> {
    // SAFETY: `id` points at a `FOLDERID_*` constant that lives for the whole program, so the
    // pointer is valid for the call. `KF_FLAG_DEFAULT` requests the current path without creating
    // anything, and passing no access token means "the calling user", which is the process owner.
    // On success the API allocates a null-terminated UTF-16 string that the caller owns;
    // `CoTaskString::own` takes that ownership immediately, so it is released exactly once on every
    // path out of this function. On failure nothing is allocated and there is nothing to release.
    let path = unsafe { SHGetKnownFolderPath(id, KF_FLAG_DEFAULT, None) }.ok()?;
    CoTaskString::own(path).to_trimmed().map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_user_folders_without_panicking() {
        let folders = user_folders();

        for folder in [folders.downloads, folders.desktop, folders.temp]
            .into_iter()
            .flatten()
        {
            assert!(folder.is_absolute(), "{}", folder.display());
            assert!(!folder.as_os_str().is_empty());
        }
    }
}
