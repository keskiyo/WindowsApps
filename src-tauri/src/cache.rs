use crate::apps_scanner::AppInfo;
use std::fs;
use std::io;
use std::path::Path;

const CACHE_FILE: &str = "apps-cache.json";

pub fn read_cache(app_data_dir: &Path) -> Option<Vec<AppInfo>> {
    let bytes = fs::read(app_data_dir.join(CACHE_FILE)).ok()?;
    serde_json::from_slice(&bytes).ok()
}

pub fn write_cache(app_data_dir: &Path, apps: &[AppInfo]) -> io::Result<()> {
    fs::create_dir_all(app_data_dir)?;
    let cache = app_data_dir.join(CACHE_FILE);
    let temporary = app_data_dir.join("apps-cache.json.tmp");
    let bytes = serde_json::to_vec(apps).map_err(io::Error::other)?;
    fs::write(&temporary, bytes)?;
    if cache.exists() {
        fs::remove_file(&cache)?;
    }
    fs::rename(temporary, cache)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apps_scanner::{AppInfo, UninstallTarget};

    #[test]
    fn round_trips_cache() {
        let dir = tempfile::tempdir().unwrap();
        let apps = vec![AppInfo {
            id: "editor".into(),
            name: "Editor".into(),
            path: r"C:\Editor.exe".into(),
            icon_base64: None,
            category: Default::default(),
            launch_kind: Default::default(),
            source_kind: Default::default(),
            description: None,
            version: None,
            publisher: None,
            install_location: None,
            can_uninstall: true,
            uninstall: Some(UninstallTarget::Command {
                executable: r"C:\Editor\uninstall.exe".into(),
                arguments: "/quiet".into(),
            }),
            resolved_path: None,
            shortcut_icon_path: None,
        }];
        write_cache(dir.path(), &apps).unwrap();
        assert_eq!(read_cache(dir.path()), Some(apps));
    }

    #[test]
    fn ignores_corrupt_cache() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("apps-cache.json"), "not json").unwrap();
        assert_eq!(read_cache(dir.path()), None);
    }
}
