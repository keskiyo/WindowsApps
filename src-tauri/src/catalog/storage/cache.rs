use crate::catalog::incremental::FilesystemIndex;
use crate::catalog::source::{SourceHealth, SourceSnapshot};
use crate::catalog::target_availability::TargetAvailabilityDiff;
use crate::catalog::{AppCategory, AppDetails, AppInfo, ArtifactKind};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

const CACHE_FILE: &str = "apps-cache.json";
pub(crate) const CACHE_SCHEMA_VERSION: u32 = 9;

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CatalogDiagnostics {
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
    #[serde(default)]
    pub sources: Vec<SourceHealth>,
    #[serde(default)]
    pub target_availability: TargetAvailabilityDiff,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CachedAppDetails {
    pub fingerprint: String,
    pub details: AppDetails,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CatalogCache {
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
    #[serde(default)]
    pub app_details: BTreeMap<String, CachedAppDetails>,
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
            app_details: BTreeMap::new(),
        }
    }
}

pub(crate) fn read_document(app_data_dir: &Path) -> Option<CatalogCache> {
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
    let places = crate::catalog::machine::MachineFacts::current();
    if let Ok(mut document) = serde_json::from_slice::<CatalogCache>(bytes) {
        if document.schema_version == CACHE_SCHEMA_VERSION {
            promote_cached_artifacts(&mut document, &places);
            return Some(document);
        }
        if matches!(document.schema_version, 2..=8) {
            if document.schema_version < 4 {
                for app in &mut document.apps {
                    crate::catalog::visibility::apply_visibility(app);
                }
            }
            if document.schema_version < 5 {
                document.sources.retain(|snapshot| {
                    snapshot.key.0 != crate::catalog::source::LEGACY_COMBINED_SOURCE
                });
            }
            if document.schema_version < 8 {
                document.app_details.clear();
            }
            if document.schema_version < 9 {
                for app in &mut document.apps {
                    classify_artifact(app, &places);
                }
                document.sources.retain(|snapshot| {
                    !matches!(snapshot.key.0.as_str(), "portable" | "installer-cache")
                });
                for snapshot in &mut document.sources {
                    for app in &mut snapshot.apps {
                        classify_artifact(app, &places);
                    }
                }
                document.filesystem_index = FilesystemIndex::default();
            }
            document.schema_version = CACHE_SCHEMA_VERSION;
            return Some(document);
        }
        return None;
    }
    let mut apps = serde_json::from_slice::<Vec<AppInfo>>(bytes).ok()?;
    for app in &mut apps {
        app.icon_base64 = None;
        classify_artifact(app, &places);
    }
    Some(CatalogCache {
        apps,
        ..CatalogCache::default()
    })
}

fn classify_artifact(app: &mut AppInfo, places: &crate::catalog::machine::MachineFacts) {
    app.artifact_kind = crate::catalog::artifact::classify(app, None, places);
    if app.artifact_kind != ArtifactKind::Application {
        app.category = AppCategory::InstallersDocs;
    }
    crate::catalog::visibility::apply_visibility(app);
}

fn promote_cached_artifacts(
    document: &mut CatalogCache,
    places: &crate::catalog::machine::MachineFacts,
) {
    for app in &mut document.apps {
        promote_artifact(app, places);
    }
    for snapshot in &mut document.sources {
        for app in &mut snapshot.apps {
            promote_artifact(app, places);
        }
    }
    for directory in document.filesystem_index.directories.values_mut() {
        for app in &mut directory.apps {
            promote_artifact(app, places);
        }
    }
}

fn promote_artifact(app: &mut AppInfo, places: &crate::catalog::machine::MachineFacts) {
    if app.artifact_kind != ArtifactKind::Application {
        if app.category != AppCategory::InstallersDocs {
            app.category = AppCategory::InstallersDocs;
            crate::catalog::visibility::apply_visibility(app);
        }
        return;
    }
    let artifact_kind = crate::catalog::artifact::classify(app, None, places);
    if artifact_kind == ArtifactKind::Application {
        return;
    }
    app.artifact_kind = artifact_kind;
    app.category = AppCategory::InstallersDocs;
    crate::catalog::visibility::apply_visibility(app);
}

fn stored_schema_version(app_data_dir: &Path) -> Option<u32> {
    let bytes = fs::read(app_data_dir.join(CACHE_FILE)).ok()?;
    serde_json::from_slice::<serde_json::Value>(&bytes)
        .ok()?
        .get("schemaVersion")?
        .as_u64()
        .and_then(|version| u32::try_from(version).ok())
}

pub(crate) fn has_newer_schema(app_data_dir: &Path) -> bool {
    stored_schema_version(app_data_dir).is_some_and(|version| version > CACHE_SCHEMA_VERSION)
}

pub(crate) fn write_document(app_data_dir: &Path, document: &CatalogCache) -> io::Result<()> {
    if has_newer_schema(app_data_dir) {
        return Ok(());
    }
    fs::create_dir_all(app_data_dir)?;
    let cache = app_data_dir.join(CACHE_FILE);
    let temporary = app_data_dir.join("apps-cache.json.tmp");
    let backup = app_data_dir.join("apps-cache.json.bak");
    let bytes = serde_json::to_vec(document).map_err(io::Error::other)?;
    {
        let mut file = fs::File::create(&temporary)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
    }
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
    Ok(())
}

pub(crate) fn reset(app_data_dir: &Path) -> io::Result<()> {
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
    use crate::app_state::cached_app;
    use crate::catalog::incremental::DirectoryRecord;
    use crate::catalog::source::{SourceKey, SourceSnapshot};
    use crate::catalog::{AppCategory, AppInfo, ArtifactKind, LaunchKind, SourceKind};

    #[test]
    fn ignores_corrupt_cache() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("apps-cache.json"), "not json").unwrap();
        assert_eq!(read_document(dir.path()), None);
    }

    // A downgrade must degrade to a rescan, never destroy the document a newer build wrote. The
    // preference store already refuses this; the catalog cache used to overwrite it silently.
    #[test]
    fn refuses_to_overwrite_a_cache_written_by_a_newer_schema() {
        let dir = tempfile::tempdir().unwrap();
        let newer = serde_json::json!({
            "schemaVersion": CACHE_SCHEMA_VERSION + 1,
            "generation": 42,
            "apps": [],
        });
        std::fs::write(
            dir.path().join(CACHE_FILE),
            serde_json::to_vec(&newer).unwrap(),
        )
        .unwrap();

        assert_eq!(read_document(dir.path()), None);
        write_document(dir.path(), &CatalogCache::default()).unwrap();

        let on_disk: serde_json::Value =
            serde_json::from_slice(&std::fs::read(dir.path().join(CACHE_FILE)).unwrap()).unwrap();
        assert_eq!(on_disk["schemaVersion"], CACHE_SCHEMA_VERSION + 1);
        assert_eq!(on_disk["generation"], 42);
        assert!(!dir.path().join("apps-cache.json.bak").exists());
    }

    #[test]
    fn writes_normally_when_the_stored_schema_is_current_or_older() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(CACHE_FILE),
            serde_json::to_vec(&serde_json::json!({
                "schemaVersion": CACHE_SCHEMA_VERSION - 1,
                "generation": 1,
                "apps": [],
            }))
            .unwrap(),
        )
        .unwrap();

        write_document(
            dir.path(),
            &CatalogCache {
                generation: 7,
                ..CatalogCache::default()
            },
        )
        .unwrap();

        assert_eq!(read_document(dir.path()).unwrap().generation, 7);
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
    fn retains_the_previous_valid_cache_as_a_backup_after_replacement() {
        let dir = tempfile::tempdir().unwrap();
        write_document(
            dir.path(),
            &CatalogCache {
                generation: 1,
                ..CatalogCache::default()
            },
        )
        .unwrap();
        write_document(
            dir.path(),
            &CatalogCache {
                generation: 2,
                ..CatalogCache::default()
            },
        )
        .unwrap();

        let backup = fs::read(dir.path().join("apps-cache.json.bak")).unwrap();

        assert_eq!(read_document(dir.path()).unwrap().generation, 2);
        assert_eq!(parse_document(&backup).unwrap().generation, 1);
    }

    #[test]
    fn migrates_v8_for_artifact_discovery_without_losing_user_data() {
        let dir = tempfile::tempdir().unwrap();
        let ordinary = cached_app("Editor", r"C:\Editor.exe");
        let mut docs = cached_app("Application Verifier Help", r"C:\Menu\Verifier Help.lnk");
        docs.launch_kind = LaunchKind::Shortcut;
        docs.source_kind = SourceKind::StartMenu;
        docs.canonical_identity = Some("docs:verifier".into());
        docs.preference_identity = Some("preference:verifier".into());
        let mut filesystem_index = FilesystemIndex::default();
        filesystem_index.directories.insert(
            r"D:\Apps".into(),
            DirectoryRecord {
                modified_nanos: 1,
                executables: Default::default(),
                child_directories: Vec::new(),
                apps: vec![ordinary.clone()],
            },
        );
        let mut app_details = BTreeMap::new();
        app_details.insert(
            docs.id.clone(),
            CachedAppDetails {
                fingerprint: "stable".into(),
                details: AppDetails::default(),
            },
        );
        let document = CatalogCache {
            schema_version: 8,
            generation: 17,
            apps: vec![ordinary, docs.clone()],
            sources: vec![
                SourceSnapshot {
                    key: SourceKey("start-menu".into()),
                    fingerprint: None,
                    health: None,
                    apps: vec![docs],
                },
                SourceSnapshot {
                    key: SourceKey("portable".into()),
                    fingerprint: None,
                    health: None,
                    apps: Vec::new(),
                },
                SourceSnapshot {
                    key: SourceKey("installer-cache".into()),
                    fingerprint: None,
                    health: None,
                    apps: Vec::new(),
                },
            ],
            filesystem_index,
            last_successful_sync: Some(10),
            diagnostics: None,
            app_details,
        };
        std::fs::write(
            dir.path().join(CACHE_FILE),
            serde_json::to_vec(&document).unwrap(),
        )
        .unwrap();

        let migrated = read_document(dir.path()).unwrap();

        assert_eq!(migrated.schema_version, 9);
        let docs = migrated
            .apps
            .iter()
            .find(|app| app.name == "Application Verifier Help")
            .unwrap();
        assert_eq!(docs.artifact_kind, ArtifactKind::Documentation);
        assert_eq!(docs.category, AppCategory::InstallersDocs);
        assert_eq!(docs.canonical_identity.as_deref(), Some("docs:verifier"));
        assert_eq!(
            docs.preference_identity.as_deref(),
            Some("preference:verifier")
        );
        assert!(migrated.filesystem_index.directories.is_empty());
        assert!(migrated.sources.iter().all(|snapshot| {
            !matches!(snapshot.key.0.as_str(), "portable" | "installer-cache")
        }));
        assert_eq!(migrated.app_details.len(), 1);
    }

    #[test]
    fn current_schema_promotes_new_structural_artifact_rules_without_rescan() {
        let dir = tempfile::tempdir().unwrap();
        let amd = cached_app(
            "AMD Software Compatibility Tool",
            r"C:\Program Files\AMD\CIM\BIN64\AMDSoftwareCompatibilityTool.exe",
        );
        let mut docs = cached_app(
            "Tools for Desktop Apps",
            r"C:\ProgramData\Microsoft\Windows\Start Menu\Programs\Windows Kits\Windows Software Development Kit\Tools for Desktop Apps.lnk",
        );
        docs.launch_kind = LaunchKind::Shortcut;
        docs.source_kind = SourceKind::StartMenu;
        docs.resolved_path = Some(
            r"C:\Program Files (x86)\Windows Kits\10\Shortcuts\DesktopDevCenterToolsDocumentation.url"
                .into(),
        );
        let mut existing_installer = cached_app("Yandex setup", r"E:\Apps\Yandex 32bit.exe");
        existing_installer.artifact_kind = ArtifactKind::Installer;
        existing_installer.category = AppCategory::Other;
        let mut filesystem_index = FilesystemIndex::default();
        filesystem_index.directories.insert(
            r"D:\Apps".into(),
            DirectoryRecord {
                modified_nanos: 1,
                executables: Default::default(),
                child_directories: Vec::new(),
                apps: vec![amd.clone()],
            },
        );
        let document = CatalogCache {
            schema_version: CACHE_SCHEMA_VERSION,
            apps: vec![amd.clone(), docs.clone(), existing_installer],
            sources: vec![SourceSnapshot {
                key: SourceKey("start-menu".into()),
                fingerprint: None,
                health: None,
                apps: vec![amd, docs],
            }],
            filesystem_index,
            ..CatalogCache::default()
        };
        std::fs::write(
            dir.path().join(CACHE_FILE),
            serde_json::to_vec(&document).unwrap(),
        )
        .unwrap();

        let loaded = read_document(dir.path()).unwrap();

        assert_eq!(loaded.apps[0].artifact_kind, ArtifactKind::Installer);
        assert_eq!(loaded.apps[0].category, AppCategory::InstallersDocs);
        assert_eq!(loaded.apps[1].artifact_kind, ArtifactKind::Documentation);
        assert_eq!(loaded.apps[1].category, AppCategory::InstallersDocs);
        assert_eq!(loaded.apps[2].artifact_kind, ArtifactKind::Installer);
        assert_eq!(loaded.apps[2].category, AppCategory::InstallersDocs);
        assert_eq!(
            loaded.sources[0].apps[0].artifact_kind,
            ArtifactKind::Installer
        );
        assert_eq!(
            loaded.sources[0].apps[1].artifact_kind,
            ArtifactKind::Documentation
        );
        assert_eq!(loaded.filesystem_index.directories.len(), 1);
        assert_eq!(
            loaded.filesystem_index.directories[r"D:\Apps"].apps[0].artifact_kind,
            ArtifactKind::Installer
        );
    }

    #[test]
    fn current_schema_promotes_url_backed_start_app_documentation() {
        let dir = tempfile::tempdir().unwrap();
        let mut website = cached_app("Node.js website", "https://nodejs.org/");
        website.source_kind = SourceKind::StartApps;
        website.launch_kind = LaunchKind::AppUserModelId;
        website.resolved_path = Some("https://nodejs.org/".into());
        let mut filesystem_index = FilesystemIndex::default();
        filesystem_index.directories.insert(
            r"D:\Apps".into(),
            DirectoryRecord {
                modified_nanos: 1,
                executables: Default::default(),
                child_directories: Vec::new(),
                apps: vec![cached_app("Editor", r"D:\Apps\Editor.exe")],
            },
        );
        let document = CatalogCache {
            schema_version: CACHE_SCHEMA_VERSION,
            apps: vec![website.clone()],
            sources: vec![SourceSnapshot {
                key: SourceKey("start-apps".into()),
                fingerprint: None,
                health: None,
                apps: vec![website],
            }],
            filesystem_index,
            ..CatalogCache::default()
        };
        std::fs::write(
            dir.path().join(CACHE_FILE),
            serde_json::to_vec(&document).unwrap(),
        )
        .unwrap();

        let loaded = read_document(dir.path()).unwrap();

        assert_eq!(loaded.apps[0].artifact_kind, ArtifactKind::Documentation);
        assert_eq!(loaded.apps[0].category, AppCategory::InstallersDocs);
        assert_eq!(
            loaded.sources[0].apps[0].artifact_kind,
            ArtifactKind::Documentation
        );
        assert_eq!(loaded.filesystem_index.directories.len(), 1);
    }

    #[test]
    fn migrates_legacy_array_to_lightweight_versioned_cache() {
        let dir = tempfile::tempdir().unwrap();
        let mut legacy = AppInfo {
            id: "editor".into(),
            name: "Editor".into(),
            path: r"C:\Editor.exe".into(),
            icon_base64: Some("data:image/png;base64,abc".into()),
            artifact_kind: Default::default(),
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
            preference_identity: None,
            visibility_class: Default::default(),
            visibility_score: 0,
            visibility_reasons: Vec::new(),
            target_availability: None,
            category_reasons: Vec::new(),
            close_risk: None,
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
        crate::catalog::visibility::apply_visibility(&mut legacy);
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
            artifact_kind: Default::default(),
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
            preference_identity: None,
            visibility_class: Default::default(),
            visibility_score: 0,
            visibility_reasons: Vec::new(),
            target_availability: None,
            category_reasons: Vec::new(),
            close_risk: None,
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
            artifact_kind: Default::default(),
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
            preference_identity: None,
            visibility_class: Default::default(),
            visibility_score: 0,
            visibility_reasons: Vec::new(),
            target_availability: None,
            category_reasons: Vec::new(),
            close_risk: None,
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
    fn migrates_v4_by_dropping_the_combined_windows_source() {
        let dir = tempfile::tempdir().unwrap();
        let document = serde_json::json!({
            "schemaVersion": 4,
            "generation": 12,
            "apps": [],
            "sources": [
                { "key": "windows", "fingerprint": null, "apps": [] },
                { "key": "steam", "fingerprint": null, "apps": [] },
                { "key": "portable", "fingerprint": null, "apps": [] }
            ],
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
        assert_eq!(migrated.generation, 12);
        let keys = migrated
            .sources
            .iter()
            .map(|snapshot| snapshot.key.0.as_str())
            .collect::<Vec<_>>();
        assert_eq!(keys, vec!["steam"]);
    }

    #[test]
    fn migrates_v5_without_details_and_preserves_document_data() {
        let dir = tempfile::tempdir().unwrap();
        let document = serde_json::json!({
            "schemaVersion": 5,
            "generation": 21,
            "apps": [],
            "sources": [{ "key": "portable", "fingerprint": null, "apps": [] }],
            "filesystemIndex": { "directories": {} },
            "lastSuccessfulSync": 123,
            "diagnostics": null
        });
        std::fs::write(
            dir.path().join(CACHE_FILE),
            serde_json::to_vec(&document).unwrap(),
        )
        .unwrap();

        let migrated = read_document(dir.path()).unwrap();

        assert_eq!(migrated.schema_version, CACHE_SCHEMA_VERSION);
        assert_eq!(migrated.generation, 21);
        assert!(migrated.sources.is_empty());
        assert!(migrated.app_details.is_empty());
    }

    #[test]
    fn migrates_v6_by_recomputing_cached_folder_availability() {
        let dir = tempfile::tempdir().unwrap();
        let document = serde_json::json!({
            "schemaVersion": 6,
            "generation": 22,
            "apps": [],
            "appDetails": {
                "task-scheduler": {
                    "fingerprint": "system-file",
                    "details": {
                        "fileSizeBytes": 145059,
                        "fileCreatedAt": 1,
                        "fileModifiedAt": 2,
                        "architecture": "notApplicable",
                        "signature": "verified",
                        "executableExists": true,
                        "installLocationExists": true
                    }
                }
            }
        });
        std::fs::write(
            dir.path().join(CACHE_FILE),
            serde_json::to_vec(&document).unwrap(),
        )
        .unwrap();

        let migrated = read_document(dir.path()).unwrap();

        assert_eq!(migrated.schema_version, CACHE_SCHEMA_VERSION);
        assert!(migrated.app_details.is_empty());
    }

    #[test]
    fn migrates_v7_by_recomputing_cached_folder_availability() {
        let dir = tempfile::tempdir().unwrap();
        let document = serde_json::json!({
            "schemaVersion": 7,
            "generation": 23,
            "apps": [],
            "appDetails": {
                "battle-net": {
                    "fingerprint": "shortcut-target",
                    "details": {
                        "fileSizeBytes": 216784,
                        "fileCreatedAt": 1,
                        "fileModifiedAt": 2,
                        "architecture": "x86",
                        "signature": "verified",
                        "executableExists": true,
                        "installLocationExists": true,
                        "canOpenFolder": false
                    }
                }
            }
        });
        std::fs::write(
            dir.path().join(CACHE_FILE),
            serde_json::to_vec(&document).unwrap(),
        )
        .unwrap();

        let migrated = read_document(dir.path()).unwrap();

        assert_eq!(migrated.schema_version, CACHE_SCHEMA_VERSION);
        assert!(migrated.app_details.is_empty());
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
