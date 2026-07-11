use std::path::{Path, PathBuf};
use winreg::enums::HKEY_CURRENT_USER;
use winreg::RegKey;

const UNINSTALL_ROOT: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall";

/// A newer version of the app is registered at a different directory than the running exe —
/// the user is launching an outdated leftover copy (e.g. after an update landed elsewhere).
pub struct StaleCopyInfo {
    pub installed_version: String,
    pub install_location: String,
}

/// Directory of the running executable, but only when it looks like an installed copy
/// (NSIS drops `uninstall.exe` next to the binary). Dev builds and loose portable copies
/// have no uninstaller and must never touch the install registry.
pub fn installed_copy_dir() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?.to_path_buf();
    has_uninstaller(&dir).then_some(dir)
}

fn has_uninstaller(dir: &Path) -> bool {
    dir.join("uninstall.exe").is_file()
}

/// Keep the NSIS "previous install location" key (`HKCU\Software\<publisher>\<product>`)
/// pointed at the directory this installed copy actually runs from. The installer reads
/// that key to reuse the user's chosen folder on update; if it was written under an older
/// publisher name (or lost), updates would silently land in the default directory instead.
pub fn sync_install_dir(publisher: &str, product: &str, dir: &Path) {
    if publisher.is_empty() || product.is_empty() {
        return;
    }
    let path = format!(r"Software\{publisher}\{product}");
    let Ok((key, _)) = RegKey::predef(HKEY_CURRENT_USER).create_subkey(&path) else {
        return;
    };
    let desired = dir.display().to_string();
    let current: Option<String> = key.get_value("").ok();
    if current.as_deref() != Some(desired.as_str()) {
        let _ = key.set_value("", &desired);
    }
}

/// Detect the "running an outdated leftover copy" situation: the uninstall registry says a
/// newer version is installed in a different directory than the running executable.
pub fn stale_copy_info(product: &str) -> Option<StaleCopyInfo> {
    let exe_dir = std::env::current_exe().ok()?.parent()?.to_path_buf();
    let key = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(format!(r"{UNINSTALL_ROOT}\{product}"))
        .ok()?;
    let version: String = key.get_value("DisplayVersion").ok()?;
    let location = clean_location(&key.get_value::<String, _>("InstallLocation").ok()?);
    if location.is_empty() || !version_newer(&version, env!("CARGO_PKG_VERSION")) {
        return None;
    }
    if same_dir(&exe_dir, Path::new(&location)) {
        return None;
    }
    Some(StaleCopyInfo {
        installed_version: version,
        install_location: location,
    })
}

/// NSIS writes InstallLocation wrapped in literal quotes; strip them and whitespace.
fn clean_location(value: &str) -> String {
    value.trim().trim_matches('"').trim().to_string()
}

fn same_dir(left: &Path, right: &Path) -> bool {
    normalized_dir(left) == normalized_dir(right)
}

fn normalized_dir(path: &Path) -> String {
    path.to_string_lossy()
        .replace('/', r"\")
        .trim_end_matches('\\')
        .to_lowercase()
}

fn version_newer(candidate: &str, current: &str) -> bool {
    version_key(candidate) > version_key(current)
}

fn version_key(version: &str) -> Vec<u64> {
    version
        .split(|character: char| !character.is_ascii_digit())
        .filter(|segment| !segment.is_empty())
        .filter_map(|segment| segment.parse().ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newer_versions_compare_numerically() {
        assert!(version_newer("0.2.3", "0.2.2"));
        assert!(version_newer("0.10.0", "0.9.9"));
        assert!(!version_newer("0.2.3", "0.2.3"));
        assert!(!version_newer("0.2.2", "0.2.3"));
    }

    #[test]
    fn install_location_quotes_are_stripped() {
        assert_eq!(
            clean_location("\"C:\\Users\\Maks\\AppData\\Local\\Windows Apps\""),
            r"C:\Users\Maks\AppData\Local\Windows Apps",
        );
        assert_eq!(clean_location("  D:\\Apps  "), r"D:\Apps");
    }

    #[test]
    fn directory_comparison_ignores_case_slashes_and_trailing_separator() {
        assert!(same_dir(
            Path::new(r"D:\Разный Хлам\Windows Apps\"),
            Path::new("d:/разный хлам/windows apps"),
        ));
        assert!(!same_dir(
            Path::new(r"D:\Apps\Windows Apps"),
            Path::new(r"C:\Apps\Windows Apps"),
        ));
    }

    #[test]
    fn only_directories_with_an_uninstaller_count_as_installed() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!has_uninstaller(dir.path()));
        std::fs::write(dir.path().join("uninstall.exe"), []).unwrap();
        assert!(has_uninstaller(dir.path()));
    }
}
