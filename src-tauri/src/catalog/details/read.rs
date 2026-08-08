use super::target::{details_file_target_with_system_root, folder_target_in, windows_directory};
use super::AppDetailsTarget;
use crate::catalog::AppDetails;
use crate::platform::windows::{
    read_architecture, verify_signature, AppArchitecture, AppSignatureStatus,
};
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

fn unix_seconds(result: std::io::Result<SystemTime>) -> Option<u64> {
    result
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_secs())
}

pub(crate) fn read_details(target: &AppDetailsTarget) -> AppDetails {
    read_details_with_system_root(target, windows_directory().as_deref())
}

fn read_details_with_system_root(
    target: &AppDetailsTarget,
    windows_root: Option<&Path>,
) -> AppDetails {
    let target_file = details_file_target_with_system_root(target, windows_root);
    let metadata = target_file
        .as_ref()
        .map(|(path, _)| path.as_path())
        .and_then(|path| fs::metadata(path).ok());
    let target_exists = target_file.as_ref().map(|(path, _)| path.is_file());
    let folder = folder_target_in(target, windows_root);
    let install_location_exists = folder.as_deref().map(Path::is_dir);
    let readable_target = target_exists == Some(true);
    let architecture = target_file
        .as_ref()
        .filter(|_| readable_target)
        .map(|(path, is_system_file)| {
            let architecture = read_architecture(path);
            if *is_system_file && architecture == AppArchitecture::Unknown {
                AppArchitecture::NotApplicable
            } else {
                architecture
            }
        })
        .unwrap_or(AppArchitecture::Unknown);

    AppDetails {
        file_size_bytes: metadata.as_ref().map(fs::Metadata::len),
        file_created_at: metadata
            .as_ref()
            .and_then(|metadata| unix_seconds(metadata.created())),
        file_modified_at: metadata
            .as_ref()
            .and_then(|metadata| unix_seconds(metadata.modified())),
        architecture,
        signature: target_file
            .as_ref()
            .filter(|_| readable_target)
            .map(|(path, _)| verify_signature(path))
            .unwrap_or(AppSignatureStatus::Unavailable),
        executable_exists: target_exists,
        install_location_exists,
        can_open_folder: folder.is_some(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{AppCategory, AppInfo, LaunchKind, SourceKind};

    fn app(path: String, launch_kind: LaunchKind) -> AppInfo {
        AppInfo {
            id: "details-test".into(),
            name: "Details test".into(),
            path,
            icon_base64: None,
            artifact_kind: Default::default(),
            category: AppCategory::Other,
            launch_kind,
            source_kind: SourceKind::Registry,
            description: None,
            version: None,
            publisher: None,
            product_name: None,
            original_filename: None,
            install_location: None,
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
        }
    }

    #[test]
    fn reads_filesystem_details() {
        let temporary = tempfile::tempdir().unwrap();
        let executable = temporary.path().join("details.exe");
        std::fs::write(&executable, [1_u8, 2, 3]).unwrap();
        let mut executable_app = app(
            executable.to_string_lossy().into_owned(),
            LaunchKind::Executable,
        );
        executable_app.install_location = Some(temporary.path().to_string_lossy().into_owned());

        let details = read_details(&AppDetailsTarget::from_app(&executable_app));

        assert_eq!(details.file_size_bytes, Some(3));
        assert!(details.file_created_at.is_some());
        assert!(details.file_modified_at.is_some());
        assert_eq!(details.executable_exists, Some(true));
        assert_eq!(details.install_location_exists, Some(true));
    }

    #[test]
    fn reads_aumid_details_from_a_trusted_resolved_executable() {
        let temporary = tempfile::tempdir().unwrap();
        let executable = temporary.path().join("packaged.exe");
        std::fs::write(&executable, [1_u8, 2, 3, 4]).unwrap();
        let mut packaged = app("Contoso.Package_123!App".into(), LaunchKind::AppUserModelId);
        packaged.resolved_path = Some(executable.to_string_lossy().into_owned());

        let aumid = read_details(&AppDetailsTarget::from_app(&packaged));

        assert_eq!(aumid.file_size_bytes, Some(4));
        assert_eq!(aumid.executable_exists, Some(true));
    }

    #[test]
    fn reads_a_non_pe_system_start_app_target() {
        let system_root = tempfile::tempdir().unwrap();
        let target_file = system_root.path().join("System32").join("taskschd.msc");
        std::fs::create_dir_all(target_file.parent().unwrap()).unwrap();
        std::fs::write(&target_file, b"console document").unwrap();
        let mut system_app = app(
            "Microsoft.AutoGenerated.{TaskScheduler}".into(),
            LaunchKind::AppUserModelId,
        );
        system_app.source_kind = SourceKind::StartApps;
        system_app.resolved_path = Some(target_file.to_string_lossy().into_owned());

        let details = read_details_with_system_root(
            &AppDetailsTarget::from_app(&system_app),
            Some(system_root.path()),
        );

        assert_eq!(details.file_size_bytes, Some(16));
        assert_eq!(details.architecture, AppArchitecture::NotApplicable);
        assert_eq!(details.executable_exists, Some(true));
        assert_eq!(details.install_location_exists, Some(true));
        assert!(details.can_open_folder);
    }

    #[test]
    fn keeps_unresolved_aumid_availability_unknown() {
        let aumid = read_details(&AppDetailsTarget::from_app(&app(
            "Contoso.Package_123!App".into(),
            LaunchKind::AppUserModelId,
        )));

        assert_eq!(aumid.file_size_bytes, None);
        assert_eq!(aumid.executable_exists, None);
        assert_eq!(aumid.install_location_exists, None);
    }
}
