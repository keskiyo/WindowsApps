use crate::catalog::{AppInfo, LaunchKind, SourceKind};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub(crate) struct AppDetailsTarget {
    launch_kind: LaunchKind,
    source_kind: SourceKind,
    path: String,
    resolved_path: Option<String>,
    install_location: Option<String>,
}

impl AppDetailsTarget {
    pub(crate) fn from_app(app: &AppInfo) -> Self {
        Self {
            launch_kind: app.launch_kind,
            source_kind: app.source_kind,
            path: app.path.clone(),
            resolved_path: app.resolved_path.clone(),
            install_location: app.install_location.clone(),
        }
    }
}

fn is_local_path(path: &Path) -> bool {
    let value = path.as_os_str().to_string_lossy();
    !value.starts_with(r"\\") && !value.starts_with("//") && path.is_absolute()
}

fn local_canonical_path(path: PathBuf) -> Option<PathBuf> {
    let value = path.to_string_lossy();
    let local = Path::new(value.strip_prefix(r"\\?\").unwrap_or(&value));
    is_local_path(local).then(|| local.to_path_buf())
}

fn canonical_local_path(path: &Path) -> Option<PathBuf> {
    local_canonical_path(fs::canonicalize(path).ok()?)
}

fn local_executable(candidate: &str) -> Option<PathBuf> {
    let path = Path::new(candidate.trim());
    let is_executable = path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"));
    (is_local_path(path) && is_executable).then(|| path.to_path_buf())
}

pub(super) fn windows_directory() -> Option<PathBuf> {
    std::env::var_os("SystemRoot").map(PathBuf::from)
}

pub(super) fn executable_target(target: &AppDetailsTarget) -> Option<PathBuf> {
    if target.launch_kind == LaunchKind::AppUserModelId {
        return target.resolved_path.as_deref().and_then(local_executable);
    }
    target
        .resolved_path
        .as_deref()
        .and_then(local_executable)
        .or_else(|| local_executable(&target.path))
}

pub(super) fn trusted_system_file_target_in(
    target: &AppDetailsTarget,
    windows_root: &Path,
) -> Option<PathBuf> {
    if target.launch_kind != LaunchKind::AppUserModelId
        || target.source_kind != SourceKind::StartApps
    {
        return None;
    }
    let candidate = target.resolved_path.as_deref().map(str::trim)?;
    let candidate = Path::new(candidate);
    if !is_local_path(candidate) || !candidate.is_file() {
        return None;
    }
    let canonical_file = canonical_local_path(candidate)?;
    let canonical_root = canonical_local_path(windows_root)?;
    (canonical_file.starts_with(&canonical_root) && canonical_file.is_file())
        .then_some(canonical_file)
}

pub(super) fn details_file_target_with_system_root(
    target: &AppDetailsTarget,
    windows_root: Option<&Path>,
) -> Option<(PathBuf, bool)> {
    executable_target(target)
        .map(|path| (path, false))
        .or_else(|| {
            windows_root
                .and_then(|root| trusted_system_file_target_in(target, root))
                .map(|path| (path, true))
        })
}

pub(super) fn details_file_target(target: &AppDetailsTarget) -> Option<PathBuf> {
    details_file_target_with_system_root(target, windows_directory().as_deref())
        .map(|(path, _)| path)
}

pub(super) fn install_location_target(target: &AppDetailsTarget) -> Option<PathBuf> {
    (target.launch_kind != LaunchKind::AppUserModelId)
        .then_some(target.install_location.as_deref())
        .flatten()
        .map(Path::new)
        .filter(|path| is_local_path(path))
        .map(Path::to_path_buf)
}

/// The single folder rule, taking its Windows root the way `details_file_target_with_system_root`
/// does. `read_details` reports `can_open_folder` from this and `open_app_folder` authorizes from
/// it, so the button's enabled state and the permission check cannot drift apart.
pub(super) fn folder_target_in(
    target: &AppDetailsTarget,
    windows_root: Option<&Path>,
) -> Option<PathBuf> {
    if target.launch_kind == LaunchKind::AppUserModelId {
        return windows_root
            .and_then(|root| trusted_system_file_target_in(target, root))
            .and_then(|path| path.parent().map(Path::to_path_buf));
    }
    install_location_target(target)
        .filter(|path| path.is_dir())
        .or_else(|| {
            executable_target(target)
                .filter(|path| path.is_file())
                .and_then(|path| path.parent().map(Path::to_path_buf))
        })
}

pub(crate) fn folder_target(target: &AppDetailsTarget) -> Option<PathBuf> {
    folder_target_in(target, windows_directory().as_deref())
}

pub(crate) fn can_open_folder(target: &AppDetailsTarget) -> bool {
    folder_target(target).is_some()
}

pub(crate) fn details_fingerprint(target: &AppDetailsTarget) -> Option<String> {
    details_file_target(target)
        .map(|path| crate::catalog::icon_cache::source_fingerprint(&path.to_string_lossy()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packaged_executable_is_detail_evidence_but_not_a_folder_action_target() {
        let target = AppDetailsTarget {
            launch_kind: LaunchKind::AppUserModelId,
            source_kind: SourceKind::Msix,
            path: "Contoso.Package_123!App".into(),
            resolved_path: Some(r"C:\Program Files\WindowsApps\Contoso\app.exe".into()),
            install_location: Some(r"C:\Program Files\WindowsApps\Contoso".into()),
        };

        assert!(executable_target(&target).is_some());
        assert!(!can_open_folder(&target));
        assert_eq!(folder_target(&target), None);
    }

    #[test]
    fn normalizes_a_verbatim_local_canonical_path() {
        assert_eq!(
            local_canonical_path(PathBuf::from(r"\\?\C:\Windows\System32")),
            Some(PathBuf::from(r"C:\Windows\System32")),
        );
    }

    #[test]
    fn refuses_a_verbatim_network_canonical_path() {
        assert_eq!(
            local_canonical_path(PathBuf::from(r"\\?\UNC\server\share")),
            None,
        );
    }

    #[test]
    fn system_start_app_file_is_a_safe_details_and_folder_target() {
        let system_root = tempfile::tempdir().unwrap();
        let system_file = system_root.path().join("System32").join("taskschd.msc");
        fs::create_dir_all(system_file.parent().unwrap()).unwrap();
        fs::write(&system_file, b"console document").unwrap();
        let target = AppDetailsTarget {
            launch_kind: LaunchKind::AppUserModelId,
            path: "Microsoft.AutoGenerated.{TaskScheduler}".into(),
            resolved_path: Some(system_file.to_string_lossy().into_owned()),
            install_location: Some(system_file.parent().unwrap().to_string_lossy().into_owned()),
            source_kind: SourceKind::StartApps,
        };
        let canonical_file = local_canonical_path(fs::canonicalize(&system_file).unwrap()).unwrap();
        let canonical_folder = canonical_file.parent().unwrap().to_path_buf();

        assert_eq!(
            trusted_system_file_target_in(&target, system_root.path()),
            Some(canonical_file),
        );
        assert_eq!(
            folder_target_in(&target, Some(system_root.path())),
            Some(canonical_folder),
        );
    }

    // `can_open_folder` (via `read_details`) and `open_app_folder` used to be two copies of this
    // rule that obtained the Windows root differently, so a missing `SystemRoot` made the button
    // report "no folder" while the command would still have opened one. One implementation now
    // answers both, and a missing root only affects the AUMID branch that actually needs it.
    #[test]
    fn a_missing_windows_root_only_withholds_the_system_start_app_folder() {
        let install = tempfile::tempdir().unwrap();
        let executable = install.path().join("editor.exe");
        fs::write(&executable, b"binary").unwrap();
        let desktop = AppDetailsTarget {
            launch_kind: LaunchKind::Executable,
            source_kind: SourceKind::Registry,
            path: executable.to_string_lossy().into_owned(),
            resolved_path: None,
            install_location: Some(install.path().to_string_lossy().into_owned()),
        };
        let system_start_app = AppDetailsTarget {
            launch_kind: LaunchKind::AppUserModelId,
            source_kind: SourceKind::StartApps,
            path: "Microsoft.AutoGenerated.{TaskScheduler}".into(),
            resolved_path: Some(executable.to_string_lossy().into_owned()),
            install_location: None,
        };

        assert_eq!(
            folder_target_in(&desktop, None),
            Some(install.path().to_path_buf()),
        );
        assert_eq!(folder_target_in(&system_start_app, None), None);
    }

    #[test]
    fn package_or_outside_start_app_targets_are_not_trusted_system_files() {
        let system_root = tempfile::tempdir().unwrap();
        let outside_root = tempfile::tempdir().unwrap();
        let outside_file = outside_root.path().join("outside.msc");
        fs::write(&outside_file, b"outside").unwrap();
        let package = AppDetailsTarget {
            launch_kind: LaunchKind::AppUserModelId,
            path: "Contoso.Package_123!App".into(),
            resolved_path: Some(outside_file.to_string_lossy().into_owned()),
            install_location: None,
            source_kind: SourceKind::Msix,
        };

        assert_eq!(
            trusted_system_file_target_in(&package, system_root.path()),
            None
        );
    }
}
