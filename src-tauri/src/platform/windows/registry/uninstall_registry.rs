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

pub(crate) fn entries() -> Vec<RegistryEntry> {
    [
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
    ]
    .into_iter()
    .flat_map(|(hive, subkey)| entries_from(hive, subkey))
    .collect()
}

fn entries_from(hive: winreg::HKEY, subkey: &str) -> Vec<RegistryEntry> {
    let Ok(uninstall) = RegKey::predef(hive).open_subkey(subkey) else {
        return Vec::new();
    };
    uninstall
        .enum_keys()
        .filter_map(Result::ok)
        .filter_map(|name| uninstall.open_subkey(name).ok())
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
        .collect()
}
