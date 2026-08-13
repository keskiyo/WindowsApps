use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
use winreg::RegKey;

pub(crate) struct RegistryEntry {
    pub(crate) display_name: String,
    pub(crate) display_icon: Option<String>,
    pub(crate) display_version: Option<String>,
    pub(crate) publisher: Option<String>,
    pub(crate) comments: Option<String>,
    pub(crate) install_location: Option<String>,
    pub(crate) uninstall_string: Option<String>,
    pub(crate) quiet_uninstall_string: Option<String>,
    pub(crate) system_component: bool,
}

pub(crate) struct RegistryEntries {
    pub(crate) entries: Vec<RegistryEntry>,
    pub(crate) complete: bool,
}

pub(crate) fn entries() -> RegistryEntries {
    let mut entries = Vec::new();
    let mut complete = true;
    for (hive, subkey) in [
        (
            HKEY_LOCAL_MACHINE,
            r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
        ),
        (
            HKEY_LOCAL_MACHINE,
            r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
        ),
        (
            HKEY_CURRENT_USER,
            r"Software\Microsoft\Windows\CurrentVersion\Uninstall",
        ),
    ] {
        match entries_from(hive, subkey) {
            Ok(root) => entries.extend(root),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => complete = false,
        }
    }
    RegistryEntries { entries, complete }
}

fn entries_from(hive: winreg::HKEY, subkey: &str) -> std::io::Result<Vec<RegistryEntry>> {
    let uninstall = RegKey::predef(hive).open_subkey(subkey)?;
    let names = uninstall.enum_keys().collect::<std::io::Result<Vec<_>>>()?;
    let keys = names
        .into_iter()
        .map(|name| uninstall.open_subkey(name))
        .collect::<std::io::Result<Vec<_>>>()?;
    Ok(keys
        .into_iter()
        .filter_map(|key| {
            let display_name = key.get_value("DisplayName").ok()?;
            Some(RegistryEntry {
                display_name,
                display_icon: key.get_value("DisplayIcon").ok(),
                display_version: key.get_value("DisplayVersion").ok(),
                publisher: key.get_value("Publisher").ok(),
                comments: key.get_value("Comments").ok(),
                install_location: key.get_value("InstallLocation").ok(),
                uninstall_string: key.get_value("UninstallString").ok(),
                quiet_uninstall_string: key.get_value("QuietUninstallString").ok(),
                system_component: key
                    .get_value::<u32, _>("SystemComponent")
                    .is_ok_and(|value| value == 1),
            })
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_registry_root_is_not_a_provider_failure() {
        let result = entries_from(
            HKEY_CURRENT_USER,
            r"Software\WindowsAppsLauncher\NoSuchUninstallRoot",
        );

        assert!(result.is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound));
    }

    #[test]
    fn a_reachable_root_is_still_enumerated() {
        let entries = entries_from(
            HKEY_LOCAL_MACHINE,
            r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
        )
        .unwrap();

        assert!(!entries.is_empty());
    }
}
