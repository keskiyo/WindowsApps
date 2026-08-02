//! Executables Windows itself has a registration for, read straight from the registry.
//!
//! Two registrations, two opposite meanings, and both are proof rather than inference:
//!
//! - **`App Paths`** — an executable a vendor registered so `start <name>` and the Run dialog find
//!   it. That is a declaration of user-facing software, and it is what tells a portable-looking
//!   `D:\...\7-Zip\7zFM.exe` apart from a bundled component sitting at the same depth.
//! - **`BundleCachePath`** — the setup bundle an installed product keeps so it can repair or
//!   uninstall itself. The file *is* an installer, said by the product that put it there, which
//!   works for vendors no needle list mentions and wherever they chose to cache it.

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

/// Executable paths registered under `App Paths`. Values are untrusted registry input and are
/// returned verbatim; the catalog compares them, it never executes them.
pub(crate) fn launchable_executables() -> Vec<String> {
    APP_PATHS
        .iter()
        .flat_map(|(hive, subkey)| subkey_default_values(*hive, subkey))
        .collect()
}

/// Setup bundles registered by the products they installed (`BundleCachePath`).
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

    /// Reads the real hives, so it asserts shape rather than content: a missing key must yield an
    /// empty list instead of failing, and no entry may be blank or quoted.
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
