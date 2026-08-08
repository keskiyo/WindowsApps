use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
use winreg::RegKey;

const APP_PATHS: &[(winreg::HKEY, &str)] = &[
    (
        HKEY_LOCAL_MACHINE,
        r"SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths",
    ),
    (
        HKEY_LOCAL_MACHINE,
        r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\App Paths",
    ),
    (
        HKEY_CURRENT_USER,
        r"Software\Microsoft\Windows\CurrentVersion\App Paths",
    ),
];

const UNINSTALL: &[(winreg::HKEY, &str)] = &[
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
];

pub(crate) fn launchable_executables() -> Vec<String> {
    APP_PATHS
        .iter()
        .flat_map(|(hive, subkey)| subkey_default_values(*hive, subkey))
        .collect()
}

pub(crate) fn installer_bundles() -> Vec<String> {
    UNINSTALL
        .iter()
        .flat_map(|(hive, subkey)| values_named(*hive, subkey, "BundleCachePath"))
        .collect()
}

fn subkey_default_values(hive: winreg::HKEY, subkey: &str) -> Vec<String> {
    for_each_subkey(hive, subkey, |key| key.get_value("").ok())
}

fn values_named(hive: winreg::HKEY, subkey: &str, value: &'static str) -> Vec<String> {
    for_each_subkey(hive, subkey, move |key| key.get_value(value).ok())
}

fn for_each_subkey(
    hive: winreg::HKEY,
    subkey: &str,
    read: impl Fn(&RegKey) -> Option<String>,
) -> Vec<String> {
    let Ok(root) = RegKey::predef(hive).open_subkey(subkey) else {
        return Vec::new();
    };
    root.enum_keys()
        .filter_map(Result::ok)
        .filter_map(|name| root.open_subkey(name).ok())
        .filter_map(|key| read(&key))
        .map(|value| value.trim().trim_matches('"').to_string())
        .filter(|value| !value.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_reads_are_tolerant_and_normalized() {
        for value in launchable_executables()
            .into_iter()
            .chain(installer_bundles())
        {
            assert!(!value.is_empty());
            assert!(!value.starts_with('"'), "{value}");
            assert_eq!(value.trim(), value, "{value}");
        }

        assert!(
            subkey_default_values(HKEY_CURRENT_USER, r"Software\WindowsApps\NoSuchKey").is_empty()
        );
    }
}
