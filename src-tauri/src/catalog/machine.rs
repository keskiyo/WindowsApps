//! What this machine says about its own executables, gathered once and handed to classification.
//!
//! Everything here is a *registration* Windows holds, not an inference the catalog draws, so it
//! generalizes by construction: it describes the machine being scanned rather than the machine the
//! rules were written on.

use super::place::{normalized_path, PlaceIndex};
use std::collections::HashSet;

/// Registrations that decide, in opposite directions, what an executable is.
pub(in crate::catalog) struct Registrations {
    launchable: HashSet<String>,
    installer_bundles: HashSet<String>,
}

impl Registrations {
    pub(in crate::catalog) fn current() -> Self {
        use crate::platform::windows::registered_targets;
        Self::from_paths(
            registered_targets::launchable_executables(),
            registered_targets::installer_bundles(),
        )
    }

    pub(in crate::catalog) fn from_paths(
        launchable: Vec<String>,
        installer_bundles: Vec<String>,
    ) -> Self {
        Self {
            launchable: launchable
                .iter()
                .map(|path| normalized_path(path))
                .collect(),
            installer_bundles: installer_bundles
                .iter()
                .map(|path| normalized_path(path))
                .collect(),
        }
    }

    #[cfg(test)]
    pub(in crate::catalog) fn empty() -> Self {
        Self::from_paths(Vec::new(), Vec::new())
    }

    /// Registered under `App Paths`: a vendor declared this executable as the one users start.
    pub(in crate::catalog) fn is_launchable(&self, path: &str) -> bool {
        self.launchable.contains(&normalized_path(path))
    }

    /// Registered as a product's own `BundleCachePath`: the file is that product's installer, said
    /// by the product itself.
    pub(in crate::catalog) fn is_installer_bundle(&self, path: &str) -> bool {
        self.installer_bundles.contains(&normalized_path(path))
    }
}

/// The machine facts artifact classification reads. Built once per scan or per cache load.
pub(in crate::catalog) struct MachineFacts {
    pub(in crate::catalog) places: PlaceIndex,
    pub(in crate::catalog) registrations: Registrations,
}

impl MachineFacts {
    pub(in crate::catalog) fn current() -> Self {
        Self {
            places: PlaceIndex::current(),
            registrations: Registrations::current(),
        }
    }

    /// Nothing resolved and nothing registered: the conservative baseline tests classify against,
    /// so an assertion cannot depend on the machine running the suite.
    #[cfg(test)]
    pub(in crate::catalog) fn empty() -> Self {
        Self {
            places: PlaceIndex::from_roots(Vec::new()),
            registrations: Registrations::empty(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registrations_compare_paths_case_and_separator_insensitively() {
        let registrations = Registrations::from_paths(
            vec![r"D:\разный хлам\7-Zip\7zFM.exe".into()],
            vec![r"C:\ProgramData\Package Cache\{guid}\AacSetup.exe".into()],
        );

        assert!(registrations.is_launchable(r"d:/РАЗНЫЙ ХЛАМ/7-Zip/7zfm.exe"));
        assert!(
            registrations.is_installer_bundle(r"c:\programdata\package cache\{guid}\aacsetup.exe")
        );
        assert!(!registrations.is_launchable(r"D:\разный хлам\7-Zip\7zG.exe"));
        assert!(!registrations.is_installer_bundle(r"C:\Program Files\Vendor\app.exe"));
    }

    #[test]
    fn an_empty_registration_set_claims_nothing() {
        let registrations = Registrations::empty();

        assert!(!registrations.is_launchable(r"C:\Program Files\Vendor\app.exe"));
        assert!(!registrations.is_installer_bundle(r"C:\Program Files\Vendor\app.exe"));
    }
}
