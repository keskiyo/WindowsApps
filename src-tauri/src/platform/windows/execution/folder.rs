use super::launcher::shell_execute;
use std::path::Path;

fn validate_local_directory(path: &Path) -> Result<(), ()> {
    let value = path.as_os_str().to_string_lossy();
    if value.starts_with(r"\\") || value.starts_with("//") || !path.is_absolute() || !path.is_dir()
    {
        return Err(());
    }
    Ok(())
}

/// Opens one already-resolved local folder through the Windows Shell. It accepts no shell
/// expression, URI, network path or caller-selected executable.
fn folder_shell_target(path: &Path) -> Result<String, ()> {
    validate_local_directory(path)?;
    Ok(path.to_string_lossy().into_owned())
}

pub(crate) fn open_trusted_folder(path: &Path) -> Result<(), ()> {
    let target = folder_shell_target(path)?;
    shell_execute(&target).map_err(|_| ())
}

#[cfg(test)]
mod tests {
    use super::{folder_shell_target, validate_local_directory};
    use std::path::Path;

    #[test]
    fn rejects_non_local_or_missing_directories() {
        let temporary = tempfile::tempdir().unwrap();
        let missing = temporary.path().join("missing");

        for candidate in [
            Path::new(""),
            Path::new("relative-folder"),
            Path::new(r"\\server\share"),
            Path::new(r"\\?\C:\directory"),
            missing.as_path(),
        ] {
            assert!(
                validate_local_directory(candidate).is_err(),
                "{candidate:?}"
            );
        }
    }

    #[test]
    fn accepts_an_existing_local_directory() {
        let temporary = tempfile::tempdir().unwrap();
        assert!(validate_local_directory(temporary.path()).is_ok());
    }

    #[test]
    fn creates_a_shell_target_for_an_existing_local_directory() {
        let temporary = tempfile::tempdir().unwrap();

        assert_eq!(
            folder_shell_target(temporary.path()).unwrap(),
            temporary.path().to_string_lossy(),
        );
    }
}
