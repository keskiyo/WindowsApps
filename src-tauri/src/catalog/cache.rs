use crate::catalog::incremental::FilesystemIndex;
use crate::catalog::source::SourceSnapshot;
use crate::catalog::AppInfo;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;

const CACHE_FILE: &str = "apps-cache.json";
pub const CACHE_SCHEMA_VERSION: u32 = 4;

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogDiagnostics {
    pub completed_at: u64,
    pub duration_ms: u64,
    pub mode: String,
    pub total_apps: usize,
    pub source_counts: BTreeMap<String, usize>,
    #[serde(default)]
    pub visibility_counts: BTreeMap<String, usize>,
    pub added: usize,
    pub removed: usize,
    pub updated: usize,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogCache {
    pub schema_version: u32,
    pub generation: u64,
    pub apps: Vec<AppInfo>,
    #[serde(default)]
    pub sources: Vec<SourceSnapshot>,
    #[serde(default)]
    pub filesystem_index: FilesystemIndex,
    #[serde(default)]
    pub last_successful_sync: Option<u64>,
    #[serde(default)]
    pub diagnostics: Option<CatalogDiagnostics>,
}

impl Default for CatalogCache {
    fn default() -> Self {
        Self {
            schema_version: CACHE_SCHEMA_VERSION,
            generation: 0,
            apps: Vec::new(),
            sources: Vec::new(),
            filesystem_index: FilesystemIndex::default(),
            last_successful_sync: None,
            diagnostics: None,
        }
    }
}

pub fn read_document(app_data_dir: &Path) -> Option<CatalogCache> {
    let primary = app_data_dir.join(CACHE_FILE);
    let backup = app_data_dir.join("apps-cache.json.bak");
    fs::read(&primary)
        .ok()
        .and_then(|bytes| parse_document(&bytes))
        .or_else(|| {
            fs::read(backup)
                .ok()
                .and_then(|bytes| parse_document(&bytes))
        })
}

fn parse_document(bytes: &[u8]) -> Option<CatalogCache> {
    if let Ok(mut document) = serde_json::from_slice::<CatalogCache>(bytes) {
        if document.schema_version == CACHE_SCHEMA_VERSION {
            return Some(document);
        }
        if matches!(document.schema_version, 2 | 3) {
            for app in &mut document.apps {
                super::visibility::apply_visibility(app);
            }
            document.schema_version = CACHE_SCHEMA_VERSION;
            return Some(document);
        }
        return None;
    }
    let mut apps = serde_json::from_slice::<Vec<AppInfo>>(bytes).ok()?;
    for app in &mut apps {
        app.icon_base64 = None;
        super::visibility::apply_visibility(app);
    }
    Some(CatalogCache {
        apps,
        ..CatalogCache::default()
    })
}

pub fn write_document(app_data_dir: &Path, document: &CatalogCache) -> io::Result<()> {
    fs::create_dir_all(app_data_dir)?;
    let cache = app_data_dir.join(CACHE_FILE);
    let temporary = app_data_dir.join("apps-cache.json.tmp");
    let backup = app_data_dir.join("apps-cache.json.bak");
    let bytes = serde_json::to_vec(document).map_err(io::Error::other)?;
    fs::write(&temporary, bytes)?;
    if cache.exists() {
        if backup.exists() {
            fs::remove_file(&backup)?;
        }
        fs::rename(&cache, &backup)?;
    }
    if let Err(error) = fs::rename(&temporary, &cache) {
        if backup.exists() {
            let _ = fs::rename(&backup, &cache);
        }
        return Err(error);
    }
    if backup.exists() {
        fs::remove_file(backup)?;
    }
    Ok(())
}

pub fn reset(app_data_dir: &Path) -> io::Result<()> {
    let cache = app_data_dir.join(CACHE_FILE);
    let temporary = app_data_dir.join("apps-cache.json.tmp");
    let backup = app_data_dir.join("apps-cache.json.bak");
    for path in [cache, temporary, backup] {
        if path.exists() {
            fs::remove_file(path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::AppInfo;

    #[test]
    fn ignores_corrupt_cache() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("apps-cache.json"), "not json").unwrap();
        assert_eq!(read_document(dir.path()), None);
    }

    #[test]
    fn recovers_from_backup_after_interrupted_cache_replacement() {
        let dir = tempfile::tempdir().unwrap();
        let backup = CatalogCache {
            generation: 9,
            ..CatalogCache::default()
        };
        std::fs::write(
            dir.path().join("apps-cache.json.bak"),
            serde_json::to_vec(&backup).unwrap(),
        )
        .unwrap();

        let recovered = read_document(dir.path()).unwrap();

        assert_eq!(recovered.generation, 9);
    }

    #[test]
    fn recovers_from_backup_when_primary_cache_is_corrupt() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(CACHE_FILE), "{broken").unwrap();
        let backup = CatalogCache {
            generation: 11,
            ..CatalogCache::default()
        };
        std::fs::write(
            dir.path().join("apps-cache.json.bak"),
            serde_json::to_vec(&backup).unwrap(),
        )
        .unwrap();

        assert_eq!(read_document(dir.path()).unwrap().generation, 11);
    }

    #[test]
    fn migrates_legacy_array_to_lightweight_versioned_cache() {
        let dir = tempfile::tempdir().unwrap();
        let mut legacy = AppInfo {
            id: "editor".into(),
            name: "Editor".into(),
            path: r"C:\Editor.exe".into(),
            icon_base64: Some("data:image/png;base64,abc".into()),
            category: Default::default(),
            launch_kind: Default::default(),
            source_kind: Default::default(),
            description: Some("Editor description".into()),
            version: Some("1.0".into()),
            publisher: Some("Publisher".into()),
            product_name: Some("Editor".into()),
            original_filename: Some("editor.exe".into()),
            install_location: Some(r"C:\".into()),
            can_uninstall: false,
            uninstall: None,
            resolved_path: None,
            shortcut_icon_path: None,
            launch_arguments: Some("--profile-directory=Work".into()),
            canonical_identity: Some("identity:editor".into()),
            visibility_class: Default::default(),
            visibility_score: 0,
            visibility_reasons: Vec::new(),
        };
        std::fs::write(
            dir.path().join(CACHE_FILE),
            serde_json::to_vec(&vec![legacy.clone()]).unwrap(),
        )
        .unwrap();

        let document = read_document(dir.path()).unwrap();

        assert_eq!(document.schema_version, CACHE_SCHEMA_VERSION);
        assert_eq!(document.generation, 0);
        assert_eq!(document.apps.len(), 1);
        assert_eq!(document.apps[0].icon_base64, None);
        legacy.icon_base64 = None;
        super::super::visibility::apply_visibility(&mut legacy);
        assert_eq!(document.apps[0], legacy);
    }

    #[test]
    fn legacy_array_reapplies_current_visibility_rules() {
        let dir = tempfile::tempdir().unwrap();
        let legacy = AppInfo {
            id: "iconv".into(),
            name: "iconv".into(),
            path: r"C:\Git\usr\bin\iconv.exe".into(),
            icon_base64: None,
            category: Default::default(),
            launch_kind: Default::default(),
            source_kind: crate::catalog::SourceKind::Portable,
            description: None,
            version: None,
            publisher: None,
            product_name: None,
            original_filename: None,
            install_location: Some(r"C:\Git".into()),
            can_uninstall: false,
            uninstall: None,
            resolved_path: None,
            shortcut_icon_path: None,
            launch_arguments: None,
            canonical_identity: None,
            visibility_class: Default::default(),
            visibility_score: 0,
            visibility_reasons: Vec::new(),
        };
        std::fs::write(
            dir.path().join(CACHE_FILE),
            serde_json::to_vec(&vec![legacy]).unwrap(),
        )
        .unwrap();

        let document = read_document(dir.path()).unwrap();

        assert_eq!(
            document.apps[0].visibility_class,
            crate::catalog::VisibilityClass::Auxiliary
        );
    }

    #[test]
    fn preserves_shortcut_resolution_fields_in_versioned_cache() {
        let dir = tempfile::tempdir().unwrap();
        let mut app = AppInfo {
            id: "firefox".into(),
            name: "Firefox".into(),
            path: r"C:\Menu\Firefox.lnk".into(),
            icon_base64: None,
            category: Default::default(),
            launch_kind: Default::default(),
            source_kind: Default::default(),
            description: None,
            version: None,
            publisher: None,
            product_name: None,
            original_filename: None,
            install_location: None,
            can_uninstall: false,
            uninstall: None,
            resolved_path: Some(r"C:\Program Files\Mozilla Firefox\firefox.exe".into()),
            shortcut_icon_path: Some(r"C:\Program Files\Mozilla Firefox\firefox.exe".into()),
            launch_arguments: None,
            canonical_identity: None,
            visibility_class: Default::default(),
            visibility_score: 0,
            visibility_reasons: Vec::new(),
        };
        write_document(
            dir.path(),
            &CatalogCache {
                apps: vec![app.clone()],
                ..CatalogCache::default()
            },
        )
        .unwrap();

        let document = read_document(dir.path()).unwrap();

        app.icon_base64 = None;
        assert_eq!(document.apps[0], app);
    }

    #[test]
    fn migrates_v2_cache_by_reclassifying_visibility_without_rescan() {
        let dir = tempfile::tempdir().unwrap();
        let document = serde_json::json!({
            "schemaVersion": 2,
            "generation": 7,
            "apps": [{
                "id": "iconv",
                "name": "iconv",
                "path": "C:\\Git\\usr\\bin\\iconv.exe",
                "iconBase64": null,
                "category": "development",
                "launchKind": "executable",
                "sourceKind": "portable",
                "description": null,
                "version": null,
                "publisher": null,
                "installLocation": "C:\\Git",
                "canUninstall": false,
                "uninstall": null
            }],
            "sources": [],
            "filesystemIndex": { "directories": {} },
            "lastSuccessfulSync": null,
            "diagnostics": null
        });
        std::fs::write(
            dir.path().join(CACHE_FILE),
            serde_json::to_vec(&document).unwrap(),
        )
        .unwrap();

        let migrated = read_document(dir.path()).unwrap();

        assert_eq!(migrated.schema_version, CACHE_SCHEMA_VERSION);
        assert_eq!(migrated.generation, 7);
        assert_eq!(
            migrated.apps[0].visibility_class,
            crate::catalog::VisibilityClass::Auxiliary
        );
    }

    #[test]
    fn reset_removes_cache_files_without_touching_preferences() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(CACHE_FILE), "{}").unwrap();
        std::fs::write(dir.path().join("apps-cache.json.tmp"), "{}").unwrap();
        std::fs::write(dir.path().join("apps-cache.json.bak"), "{}").unwrap();
        std::fs::write(dir.path().join("scan-settings.json"), "{}").unwrap();

        reset(dir.path()).unwrap();

        assert!(!dir.path().join(CACHE_FILE).exists());
        assert!(!dir.path().join("apps-cache.json.tmp").exists());
        assert!(!dir.path().join("apps-cache.json.bak").exists());
        assert!(dir.path().join("scan-settings.json").exists());
    }
}
