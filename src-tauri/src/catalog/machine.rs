use super::place::{normalized_path, PlaceIndex};
use std::collections::{HashMap, HashSet};

pub(in crate::catalog) struct Associations {
    by_executable: HashMap<String, Vec<String>>,
}

impl Associations {
    pub(in crate::catalog) fn current() -> Self {
        Self::from_pairs(crate::platform::windows::associations::registered_associations())
    }

    pub(in crate::catalog) fn from_pairs(pairs: Vec<(String, Vec<String>)>) -> Self {
        Self {
            by_executable: pairs
                .into_iter()
                .map(|(executable, tokens)| (executable.to_lowercase(), tokens))
                .collect(),
        }
    }

    #[cfg(test)]
    pub(in crate::catalog) fn empty() -> Self {
        Self {
            by_executable: HashMap::new(),
        }
    }

    pub(in crate::catalog) fn of(&self, executable_stem: &str) -> &[String] {
        self.by_executable
            .get(executable_stem)
            .map_or(&[], Vec::as_slice)
    }
}

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

    pub(in crate::catalog) fn is_launchable(&self, path: &str) -> bool {
        self.launchable.contains(&normalized_path(path))
    }

    pub(in crate::catalog) fn is_installer_bundle(&self, path: &str) -> bool {
        self.installer_bundles.contains(&normalized_path(path))
    }
}

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
