use crate::catalog::sync::scan_control::{StageBudget, StageStop};
use crate::catalog::{AppInfo, ArtifactKind};
use std::path::PathBuf;
use walkdir::WalkDir;

pub(in crate::catalog) const MAX_DURATION: std::time::Duration = std::time::Duration::from_secs(5);
pub(in crate::catalog) const MAX_ENTRIES: usize = 25_000;
pub(in crate::catalog) const MAX_DEPTH: usize = 4;

pub(in crate::catalog) struct InstallerCacheScan {
    pub(in crate::catalog) apps: Vec<AppInfo>,
    pub(in crate::catalog) stop: Option<StageStop>,
}

pub(in crate::catalog) fn roots() -> Vec<PathBuf> {
    let mut roots = environment_roots(
        std::env::var_os("LOCALAPPDATA").map(PathBuf::from),
        std::env::var_os("ProgramData").map(PathBuf::from),
        std::env::var_os("ProgramFiles").map(PathBuf::from),
        std::env::var_os("ProgramFiles(x86)").map(PathBuf::from),
    );
    roots.retain(|root| root.is_dir());
    roots
}

fn environment_roots(
    local_app_data: Option<PathBuf>,
    program_data: Option<PathBuf>,
    program_files: Option<PathBuf>,
    program_files_x86: Option<PathBuf>,
) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(local_app_data) = local_app_data {
        roots.push(local_app_data.join("Package Cache"));
        roots.push(local_app_data.join(r"Microsoft\OneDrive"));
    }
    if let Some(program_data) = program_data {
        roots.push(program_data.join("Package Cache"));
    }
    if let Some(program_files) = program_files {
        roots.push(program_files.join(r"AMD\CIM"));
    }
    if let Some(program_files_x86) = program_files_x86 {
        roots.push(program_files_x86.join(r"Microsoft Visual Studio\Installer"));
    }
    roots
}

pub(in crate::catalog) fn scan_roots(
    roots: &[PathBuf],
    budget: &StageBudget<'_>,
) -> InstallerCacheScan {
    let facts = crate::catalog::machine::MachineFacts::current();
    let apps = roots
        .iter()
        .flat_map(|root| {
            WalkDir::new(root)
                .follow_links(false)
                .max_depth(budget.max_depth())
                .into_iter()
        })
        .take_while(|_| budget.charge_entry())
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter(|entry| {
            entry
                .path()
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
        })
        .filter_map(|entry| crate::catalog::portable_app(entry.into_path(), &facts))
        .filter(|app| app.artifact_kind == ArtifactKind::Installer)
        .collect();
    InstallerCacheScan {
        apps,
        stop: budget.stop(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::sync::scan_control::{ScanControl, StageStop};
    use crate::catalog::ArtifactKind;
    use std::fs;
    use std::time::Duration;

    #[test]
    fn environment_roots_include_only_bounded_known_installer_locations() {
        let roots = environment_roots(
            Some(PathBuf::from(r"C:\Users\Test\AppData\Local")),
            Some(PathBuf::from(r"C:\ProgramData")),
            Some(PathBuf::from(r"C:\Program Files")),
            Some(PathBuf::from(r"C:\Program Files (x86)")),
        );

        assert_eq!(
            roots,
            vec![
                PathBuf::from(r"C:\Users\Test\AppData\Local\Package Cache"),
                PathBuf::from(r"C:\Users\Test\AppData\Local\Microsoft\OneDrive"),
                PathBuf::from(r"C:\ProgramData\Package Cache"),
                PathBuf::from(r"C:\Program Files\AMD\CIM"),
                PathBuf::from(r"C:\Program Files (x86)\Microsoft Visual Studio\Installer"),
            ]
        );
    }

    #[test]
    fn scans_only_executables_and_classifies_setup_artifacts() {
        let directory = tempfile::tempdir().unwrap();
        fs::write(directory.path().join("setup.exe"), b"fixture").unwrap();
        fs::write(directory.path().join("notes.txt"), b"fixture").unwrap();
        let cancelled = || false;
        let control = ScanControl::new(&cancelled);
        let budget = control.stage_with(Duration::from_secs(1), 50, 4);

        let result = scan_roots(&[directory.path().to_path_buf()], &budget);

        assert_eq!(result.stop, None);
        assert_eq!(result.apps.len(), 1);
        assert_eq!(result.apps[0].artifact_kind, ArtifactKind::Installer);
    }

    #[test]
    fn entry_limit_marks_snapshot_incomplete() {
        let directory = tempfile::tempdir().unwrap();
        fs::create_dir(directory.path().join("nested")).unwrap();
        fs::write(
            directory.path().join("nested").join("setup.exe"),
            b"fixture",
        )
        .unwrap();
        let cancelled = || false;
        let control = ScanControl::new(&cancelled);
        let budget = control.stage_with(Duration::from_secs(1), 1, 4);

        let result = scan_roots(&[directory.path().to_path_buf()], &budget);

        assert_eq!(result.stop, Some(StageStop::EntryLimit));
    }
}
