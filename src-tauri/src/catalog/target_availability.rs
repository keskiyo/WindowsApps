use crate::catalog::{AppInfo, LaunchKind};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::ErrorKind;
use std::path::Path;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TargetUnverifiableReason {
    UnmountedVolume,
    NetworkPathUnavailable,
    AccessDenied,
    TemporaryIoError,
    RelativePathWithoutBase,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TargetNonFileReason {
    AppUserModelId,
    SteamUri,
    ShellProtocol,
    ShellLocation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TargetAvailability {
    Present,
    Missing,
    Unverifiable(TargetUnverifiableReason),
    NotApplicable(TargetNonFileReason),
}

impl TargetAvailability {
    pub(crate) fn reason_id(self) -> &'static str {
        match self {
            Self::Present => "target.present",
            Self::Missing => "target.missing",
            Self::Unverifiable(reason) => match reason {
                TargetUnverifiableReason::UnmountedVolume => "target.unverifiable.unmounted_volume",
                TargetUnverifiableReason::NetworkPathUnavailable => "target.unverifiable.network",
                TargetUnverifiableReason::AccessDenied => "target.unverifiable.access_denied",
                TargetUnverifiableReason::TemporaryIoError => "target.unverifiable.io_error",
                TargetUnverifiableReason::RelativePathWithoutBase => {
                    "target.unverifiable.relative_path"
                }
            },
            Self::NotApplicable(reason) => match reason {
                TargetNonFileReason::AppUserModelId => "target.not_applicable.aumid",
                TargetNonFileReason::SteamUri => "target.not_applicable.steam_uri",
                TargetNonFileReason::ShellProtocol => "target.not_applicable.protocol",
                TargetNonFileReason::ShellLocation => "target.not_applicable.shell_location",
            },
        }
    }
}

pub(crate) fn would_legacy_keep(availability: TargetAvailability) -> bool {
    !matches!(
        availability,
        TargetAvailability::Missing
            | TargetAvailability::Unverifiable(TargetUnverifiableReason::AccessDenied)
            | TargetAvailability::Unverifiable(TargetUnverifiableReason::TemporaryIoError)
    )
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TargetAvailabilityDiff {
    pub by_reason: BTreeMap<String, usize>,
    pub kept_by_new_rule: usize,
}

impl TargetAvailabilityDiff {
    pub(crate) fn record(&mut self, availability: TargetAvailability, new_rule_active: bool) {
        *self
            .by_reason
            .entry(availability.reason_id().to_owned())
            .or_insert(0) += 1;
        let kept_here = availability != TargetAvailability::Missing;
        if new_rule_active && kept_here && !would_legacy_keep(availability) {
            self.kept_by_new_rule += 1;
        }
    }
}

pub(crate) fn availability(app: &AppInfo) -> TargetAvailability {
    if app.launch_kind == LaunchKind::AppUserModelId {
        return TargetAvailability::NotApplicable(TargetNonFileReason::AppUserModelId);
    }
    let target = app.resolved_path.as_deref().unwrap_or(&app.path).trim();
    if target.is_empty() {
        return TargetAvailability::NotApplicable(TargetNonFileReason::ShellLocation);
    }
    if target.starts_with("steam://") {
        return TargetAvailability::NotApplicable(TargetNonFileReason::SteamUri);
    }
    if target.contains("://") {
        return TargetAvailability::NotApplicable(TargetNonFileReason::ShellProtocol);
    }
    if target.starts_with(r"\\") {
        return TargetAvailability::Unverifiable(TargetUnverifiableReason::NetworkPathUnavailable);
    }
    let path = Path::new(target);
    if !path.is_absolute() {
        return TargetAvailability::Unverifiable(TargetUnverifiableReason::RelativePathWithoutBase);
    }
    let volume_mounted = path
        .ancestors()
        .last()
        .is_some_and(|root| root.metadata().is_ok());
    if !volume_mounted {
        return TargetAvailability::Unverifiable(TargetUnverifiableReason::UnmountedVolume);
    }
    match path.metadata() {
        Ok(_) => TargetAvailability::Present,
        Err(error) => from_metadata_error(error.kind()),
    }
}

fn from_metadata_error(kind: ErrorKind) -> TargetAvailability {
    match kind {
        ErrorKind::NotFound => TargetAvailability::Missing,
        ErrorKind::PermissionDenied => {
            TargetAvailability::Unverifiable(TargetUnverifiableReason::AccessDenied)
        }
        _ => TargetAvailability::Unverifiable(TargetUnverifiableReason::TemporaryIoError),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{AppCategory, SourceKind};

    fn app(path: &str, launch_kind: LaunchKind) -> AppInfo {
        AppInfo {
            id: "test".into(),
            name: "Test".into(),
            path: path.into(),
            icon_base64: None,
            artifact_kind: Default::default(),
            category: AppCategory::Other,
            launch_kind,
            source_kind: SourceKind::Registry,
            description: None,
            version: None,
            publisher: None,
            product_name: None,
            original_filename: None,
            install_location: None,
            can_uninstall: false,
            uninstall: None,
            resolved_path: None,
            shortcut_icon_path: None,
            launch_arguments: None,
            canonical_identity: None,
            preference_identity: None,
            visibility_class: Default::default(),
            visibility_score: 0,
            visibility_reasons: Vec::new(),
            target_availability: None,
            category_reasons: Vec::new(),
            close_risk: None,
        }
    }

    #[test]
    fn an_existing_file_is_present() {
        let directory = tempfile::tempdir().unwrap();
        let file = directory.path().join("app.exe");
        std::fs::write(&file, b"binary").unwrap();

        let availability = availability(&app(&file.to_string_lossy(), LaunchKind::Executable));

        assert_eq!(availability, TargetAvailability::Present);
        assert_eq!(availability.reason_id(), "target.present");
    }

    #[test]
    fn a_deleted_file_on_a_mounted_volume_is_missing() {
        let directory = tempfile::tempdir().unwrap();
        let file = directory.path().join("gone.exe");

        let target = app(&file.to_string_lossy(), LaunchKind::Executable);

        assert_eq!(availability(&target), TargetAvailability::Missing);
        assert_eq!(availability(&target), TargetAvailability::Missing);
    }

    #[test]
    fn an_unmounted_volume_is_unverifiable_and_keeps_the_record() {
        let target = app(r"Q:\Portable\App\app.exe", LaunchKind::Executable);

        assert_eq!(
            availability(&target),
            TargetAvailability::Unverifiable(TargetUnverifiableReason::UnmountedVolume)
        );
        assert_ne!(availability(&target), TargetAvailability::Missing);
    }

    #[test]
    fn a_network_path_is_never_touched() {
        let target = app(r"\\fileserver\share\app.exe", LaunchKind::Executable);

        assert_eq!(
            availability(&target),
            TargetAvailability::Unverifiable(TargetUnverifiableReason::NetworkPathUnavailable)
        );
        assert_ne!(availability(&target), TargetAvailability::Missing);
    }

    #[test]
    fn launch_mechanisms_that_are_not_files_are_not_checked_as_files() {
        for (target, expected) in [
            (
                app(
                    "Microsoft.WindowsCalculator_8wekyb!App",
                    LaunchKind::AppUserModelId,
                ),
                TargetNonFileReason::AppUserModelId,
            ),
            (
                app("steam://rungameid/220", LaunchKind::Executable),
                TargetNonFileReason::SteamUri,
            ),
            (
                app("http://example.invalid/help", LaunchKind::Shortcut),
                TargetNonFileReason::ShellProtocol,
            ),
            (
                app("", LaunchKind::Shortcut),
                TargetNonFileReason::ShellLocation,
            ),
        ] {
            assert_eq!(
                availability(&target),
                TargetAvailability::NotApplicable(expected),
                "{}",
                target.path
            );
            assert_ne!(availability(&target), TargetAvailability::Missing);
        }
    }

    #[test]
    fn a_path_with_no_base_to_resolve_against_is_kept() {
        for path in [r"bin\app.exe", "ms-settings:appsfeatures"] {
            let target = app(path, LaunchKind::Executable);

            assert_eq!(
                availability(&target),
                TargetAvailability::Unverifiable(TargetUnverifiableReason::RelativePathWithoutBase),
                "{path}"
            );
            assert_ne!(availability(&target), TargetAvailability::Missing);
        }
    }

    #[test]
    fn only_a_definite_not_found_can_remove_an_application() {
        assert_eq!(
            from_metadata_error(ErrorKind::NotFound),
            TargetAvailability::Missing
        );
        assert_eq!(
            from_metadata_error(ErrorKind::PermissionDenied),
            TargetAvailability::Unverifiable(TargetUnverifiableReason::AccessDenied)
        );
        for kind in [
            ErrorKind::TimedOut,
            ErrorKind::Interrupted,
            ErrorKind::WouldBlock,
            ErrorKind::Other,
        ] {
            assert_eq!(
                from_metadata_error(kind),
                TargetAvailability::Unverifiable(TargetUnverifiableReason::TemporaryIoError),
                "{kind:?}"
            );
        }
    }

    #[test]
    fn a_shortcut_is_judged_by_its_resolved_target() {
        let directory = tempfile::tempdir().unwrap();
        let shortcut = directory.path().join("App.lnk");
        std::fs::write(&shortcut, b"shortcut").unwrap();
        let mut target = app(&shortcut.to_string_lossy(), LaunchKind::Shortcut);
        target.resolved_path = Some(
            directory
                .path()
                .join("uninstalled.exe")
                .to_string_lossy()
                .into_owned(),
        );

        assert_eq!(availability(&target), TargetAvailability::Missing);
    }

    #[test]
    fn the_two_rules_agree() {
        let directory = tempfile::tempdir().unwrap();
        let present = directory.path().join("present.exe");
        std::fs::write(&present, b"binary").unwrap();
        let absent = directory.path().join("gone.exe");
        let mut shortcut = app(&present.to_string_lossy(), LaunchKind::Shortcut);
        shortcut.resolved_path = Some(absent.to_string_lossy().into_owned());

        for target in [
            app(&present.to_string_lossy(), LaunchKind::Executable),
            app(&absent.to_string_lossy(), LaunchKind::Executable),
            app(r"Q:\Portable\App\app.exe", LaunchKind::Executable),
            app(r"\\fileserver\share\app.exe", LaunchKind::Executable),
            app(r"bin\app.exe", LaunchKind::Executable),
            app("ms-settings:appsfeatures", LaunchKind::Executable),
            app("steam://rungameid/220", LaunchKind::Executable),
            app("", LaunchKind::Shortcut),
            app("Vendor.Package!App", LaunchKind::AppUserModelId),
            shortcut,
        ] {
            assert_eq!(
                would_legacy_keep(availability(&target)),
                crate::catalog::legacy_target_is_present(&target),
                "{}",
                target.path
            );
        }
    }

    #[test]
    fn only_a_check_that_failed_counts_as_kept_by_the_new_rule() {
        let mut diff = TargetAvailabilityDiff::default();
        for availability in [
            TargetAvailability::Present,
            TargetAvailability::Missing,
            TargetAvailability::Unverifiable(TargetUnverifiableReason::UnmountedVolume),
            TargetAvailability::Unverifiable(TargetUnverifiableReason::NetworkPathUnavailable),
            TargetAvailability::Unverifiable(TargetUnverifiableReason::RelativePathWithoutBase),
            TargetAvailability::NotApplicable(TargetNonFileReason::AppUserModelId),
            TargetAvailability::Unverifiable(TargetUnverifiableReason::AccessDenied),
            TargetAvailability::Unverifiable(TargetUnverifiableReason::TemporaryIoError),
        ] {
            diff.record(availability, true);
        }

        assert_eq!(diff.kept_by_new_rule, 2);
        assert_eq!(diff.by_reason.values().sum::<usize>(), 8);
        assert_eq!(diff.by_reason.get("target.missing"), Some(&1));
    }

    #[test]
    fn the_rollback_rule_reports_no_records_saved() {
        let mut diff = TargetAvailabilityDiff::default();
        diff.record(
            TargetAvailability::Unverifiable(TargetUnverifiableReason::AccessDenied),
            false,
        );

        assert_eq!(diff.kept_by_new_rule, 0);
        assert_eq!(
            diff.by_reason.get("target.unverifiable.access_denied"),
            Some(&1)
        );
    }

    #[test]
    fn every_outcome_has_a_distinct_stable_identifier() {
        let identifiers = [
            TargetAvailability::Present,
            TargetAvailability::Missing,
            TargetAvailability::Unverifiable(TargetUnverifiableReason::UnmountedVolume),
            TargetAvailability::Unverifiable(TargetUnverifiableReason::NetworkPathUnavailable),
            TargetAvailability::Unverifiable(TargetUnverifiableReason::AccessDenied),
            TargetAvailability::Unverifiable(TargetUnverifiableReason::TemporaryIoError),
            TargetAvailability::Unverifiable(TargetUnverifiableReason::RelativePathWithoutBase),
            TargetAvailability::NotApplicable(TargetNonFileReason::AppUserModelId),
            TargetAvailability::NotApplicable(TargetNonFileReason::SteamUri),
            TargetAvailability::NotApplicable(TargetNonFileReason::ShellProtocol),
            TargetAvailability::NotApplicable(TargetNonFileReason::ShellLocation),
        ]
        .map(TargetAvailability::reason_id);
        let unique = identifiers.iter().collect::<std::collections::HashSet<_>>();

        assert_eq!(unique.len(), identifiers.len());
        assert!(identifiers
            .iter()
            .all(|identifier| identifier.starts_with("target.")));
    }
}
