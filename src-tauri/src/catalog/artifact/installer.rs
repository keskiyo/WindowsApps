//! Installation-artifact evidence: a downloaded setup executable, a self-extracting stub, a shared
//! runtime redistributable.
//!
//! Evidence is bound to the field where it is honest. A vendor-authored file name
//! (`OriginalFilename`, `InternalName`, the executable's own name) reads the same on every machine;
//! a display name does not, so `Uninstall Foo` and `Advanced Installer` are deliberately *not*
//! decided here. Uninstall targets are excluded outright — they are maintenance entries, and
//! `visibility` demotes them to Auxiliary tools where they stay visible and restorable.

use crate::catalog::fields::{Field, MarkerFields};
use crate::catalog::filters::is_uninstall_target_path;
use crate::catalog::machine::MachineFacts;
use crate::catalog::place::Place;
use crate::catalog::{AppInfo, LaunchKind, SourceKind};
use std::path::Path;

pub(super) fn is_installation_artifact(
    app: &AppInfo,
    internal_name: Option<&str>,
    facts: &MachineFacts,
) -> bool {
    // A Steam entry launches through `steam://rungameid/...`, so it is a library item and never a
    // setup file the user runs. Steam's shared redistributables depot ("Steamworks Common
    // Redistributables") reads exactly like one otherwise; `visibility` files it under Auxiliary
    // tools instead, which is where a platform component belongs.
    if app.source_kind == SourceKind::Steam {
        return false;
    }
    // An AppUserModelId is an opaque identifier: without a resolved target there is nothing
    // trustworthy to read, and the identifier's own words never become evidence.
    let launch_path = if app.launch_kind == LaunchKind::AppUserModelId {
        let Some(target) = app.resolved_path.as_deref() else {
            return false;
        };
        target
    } else {
        app.resolved_path.as_deref().unwrap_or(&app.path)
    };
    if is_uninstall_target_path(Path::new(launch_path)) {
        return false;
    }
    // Proof rather than inference: some installed product registered this exact file as the setup
    // bundle it keeps for repair and uninstall. True for any vendor, wherever they cache it, and
    // it needs no name to read like an installer.
    if facts.registrations.is_installer_bundle(launch_path) {
        return true;
    }
    if has_installer_filename_evidence(launch_path, app.original_filename.as_deref(), internal_name)
    {
        return true;
    }
    let fields = MarkerFields::from_app(app);
    is_mysql_installer(launch_path)
        || has_installer_description_evidence(app, launch_path, facts)
        || is_runtime_redistributable(&fields)
        || is_self_extracting_stub(&fields)
}

/// A self-extracting installer stub, identified by the metadata its own vendor wrote into it.
///
/// `AdbeRdr11000_ru_RU.exe` bundled inside another product's `\support\` tree carries
/// `OriginalFilename = AdobeSelfExtractor.exe` and `ProductName = Adobe Self Extractor`. The file
/// name is the vendor's, not the machine's, so this recognizes the same stub shipped by any
/// product — where the previous `adberdr` needle only recognized Adobe's, on one PC.
fn is_self_extracting_stub(fields: &MarkerFields) -> bool {
    fields.any(Field::FileName, &["selfextract", "self-extract"])
        || fields.any(Field::Prose, &["self extractor", "self-extracting"])
}

/// A shared runtime shipped so other software can run, and the self-extracting stubs that install
/// one.
///
/// This used to reject the record outright from `visibility`, which meant a word could delete an
/// application on a machine nobody inspected. It now files the record under Installers & Docs,
/// where a wrong answer costs the user one click instead of the app.
fn is_runtime_redistributable(fields: &MarkerFields) -> bool {
    fields.any(Field::FileName, &["vcredist", "vc_redist", "dxsetup", "ndp48"])
        // "webview2 runtime" is spelled out so a real app named "WebView2 Sample" is not swept up;
        // the MSIX WebView2 framework package is handled by `visibility::is_framework_package`.
        || fields.any(
            Field::Prose,
            &["redistributable", "webview2 runtime", "bootstrapper"],
        )
}

pub(in crate::catalog) fn has_installer_filename_evidence(
    path: &str,
    original_filename: Option<&str>,
    internal_name: Option<&str>,
) -> bool {
    let normalized_path = path.replace('/', r"\").to_lowercase();
    let filename = Path::new(path)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    if filename.eq_ignore_ascii_case("AMDSoftwareCompatibilityTool")
        && normalized_path.contains(r"\amd\cim\")
    {
        return true;
    }
    // No location gate. A setup executable is a setup executable in a folder called `Downloads`,
    // in one called `Загрузки`, and in one called `D:\Distr` — and the folder a user keeps
    // downloads in is not something any API can resolve. The file name the vendor wrote is the
    // evidence that travels between machines; requiring a recognizable folder on top of it only
    // meant installers went undetected everywhere the folder list had not been written for.
    internal_name.is_some_and(strong_installer_name)
        || strong_installer_name(filename)
        || original_filename.is_some_and(strong_installer_name)
}

fn is_mysql_installer(launch_path: &str) -> bool {
    let normalized_path = launch_path.replace('/', r"\").to_lowercase();
    let file_stem = Path::new(launch_path)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    normalized_path.contains(r"\mysql\mysql installer for windows\")
        && file_stem.eq_ignore_ascii_case("MySQLInstaller")
}

/// A `FileDescription` whose last word is "Setup" ("Attack Shark Software Setup"). On its own this
/// is weak — a configuration tool can describe itself the same way — so it needs one corroborator
/// that holds on any machine: the file sits where downloads or setup bundles land, or Windows has
/// no registration for it at all, which an installed product would have.
fn has_installer_description_evidence(app: &AppInfo, path: &str, facts: &MachineFacts) -> bool {
    let describes_setup = app.description.as_deref().is_some_and(|value| {
        value
            .split(|character: char| !character.is_alphanumeric())
            .rfind(|token| !token.is_empty())
            .is_some_and(|token| token.eq_ignore_ascii_case("setup"))
    });
    describes_setup
        && (facts.places.classify(path) != Place::Unknown || !is_registered_product(app))
}

/// Windows knows this record as installed software: it has an uninstall entry, or it arrived
/// through a packaging surface that only lists installed products.
fn is_registered_product(app: &AppInfo) -> bool {
    match app.source_kind {
        SourceKind::Registry => app.can_uninstall,
        SourceKind::Msix | SourceKind::StartApps | SourceKind::Steam => true,
        SourceKind::StartMenu | SourceKind::Portable => false,
    }
}

fn strong_installer_name(value: &str) -> bool {
    let stem = Path::new(value)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or(value)
        .to_lowercase();
    // The Windows Installer engine itself, by its Windows-wide name. A copy shipped inside another
    // product's tree — a WINE distribution, a portable runtime — is an installation tool, not an
    // application, on every machine that has one.
    stem == "msiexec"
        || stem == "setup"
        || stem == "installer"
        || stem.contains("_setup")
        || stem.contains("-setup")
        || stem.contains("_installer")
        || stem.contains("-installer")
        || stem.starts_with("setup_")
        || stem.starts_with("setup-")
        || stem.ends_with("setup")
}

#[cfg(test)]
mod tests {
    use super::super::testing::{candidate, start_menu_shortcut};
    use super::super::ArtifactKind;
    use crate::catalog::machine::MachineFacts;
    use crate::catalog::{AppInfo, LaunchKind, SourceKind};

    /// Classification with no resolved user folders, so only Windows' own path constants apply.
    /// That keeps every assertion here independent of the machine running the suite — and it is
    /// the conservative case, since a resolved folder can only add evidence.
    fn classify(app: &AppInfo, internal_name: Option<&str>) -> ArtifactKind {
        super::super::classify(app, internal_name, &MachineFacts::empty())
    }

    #[test]
    fn strong_executable_evidence_identifies_installers() {
        let cases = [
            (
                candidate(
                    "hidemy.name VPN",
                    r"D:\1MAIN\Downloads\hidemyname_vpn_2.1.915.exe",
                ),
                Some("hidemyname_vpn_setup_v2.1.915.exe"),
                None,
            ),
            (
                candidate("Yandex", r"E:\Programs\Browser\Yandex 32bit.exe"),
                None,
                Some("lite_installer"),
            ),
            (
                candidate(
                    "Visual Studio Installer",
                    r"C:\Program Files (x86)\Microsoft Visual Studio\Installer\setup.exe",
                ),
                Some("setup.exe"),
                None,
            ),
            (
                candidate(
                    "Python 3.14",
                    r"C:\Users\Maks\AppData\Local\Package Cache\{id}\python-3.14.0-amd64.exe",
                ),
                None,
                Some("setup"),
            ),
            (
                candidate(
                    "Microsoft OneDrive",
                    r"C:\Users\Maks\AppData\Local\Microsoft\OneDrive\26.1\OneDriveSetup.exe",
                ),
                Some("OneDriveSetup.exe"),
                None,
            ),
        ];

        for (mut app, original_filename, internal_name) in cases {
            app.original_filename = original_filename.map(str::to_string);
            assert_eq!(
                classify(&app, internal_name),
                ArtifactKind::Installer,
                "{}",
                app.path
            );
        }
    }

    #[test]
    fn product_names_do_not_turn_apps_into_installers() {
        for (name, path) in [
            ("Revo Uninstaller", r"C:\Program Files\Revo\RevoUninPro.exe"),
            (
                "Advanced Installer",
                r"C:\Program Files\Caphyon\Advanced Installer\AdvancedInstaller.exe",
            ),
        ] {
            assert_eq!(
                classify(&candidate(name, path), None),
                ArtifactKind::Application
            );
        }
    }

    #[test]
    fn amd_cim_compatibility_tool_is_installer_only_in_its_install_manager_path() {
        let amd_cim = candidate(
            "AMD Software Compatibility Tool",
            r"C:\Program Files\AMD\CIM\BIN64\AMDSoftwareCompatibilityTool.exe",
        );
        assert_eq!(classify(&amd_cim, None), ArtifactKind::Installer);

        let unrelated = candidate(
            "AMD Software Compatibility Tool",
            r"D:\Portable\AMDSoftwareCompatibilityTool.exe",
        );
        assert_eq!(classify(&unrelated, None), ArtifactKind::Application);
    }

    #[test]
    fn corroborated_mysql_attack_shark_and_windows_sdk_targets_are_installers() {
        let mysql_target =
            r"C:\Program Files (x86)\MySQL\MySQL Installer for Windows\MySQLInstaller.exe";
        let mysql_description = "The MySQL Installer is designed to provide a central point for installation and upgrade of all major MySQL products.";
        let mut mysql_shortcut = start_menu_shortcut(
            "MySQL Installer - Community",
            r"C:\ProgramData\Microsoft\Windows\Start Menu\Programs\MySQL\MySQL Installer - Community\MySQL Installer - Community.lnk",
            mysql_target,
        );
        mysql_shortcut.product_name = Some("MySQL Installer".into());
        mysql_shortcut.description = Some(mysql_description.into());
        mysql_shortcut.original_filename = Some("MySQLInstaller.exe".into());
        assert_eq!(
            classify(&mysql_shortcut, Some("MySQLInstaller.exe")),
            ArtifactKind::Installer
        );

        let mut mysql_aumid = candidate(
            "MySQL Installer - Community",
            "Microsoft.AutoGenerated.{2C7C9013-A7FD-5C12-D811-939AFE2A3592}",
        );
        mysql_aumid.source_kind = SourceKind::StartApps;
        mysql_aumid.launch_kind = LaunchKind::AppUserModelId;
        mysql_aumid.resolved_path = Some(mysql_target.into());
        mysql_aumid.product_name = Some("MySQL Installer".into());
        mysql_aumid.description = Some(mysql_description.into());
        mysql_aumid.original_filename = Some("MySQLInstaller.exe".into());
        assert_eq!(
            classify(&mysql_aumid, Some("MySQLInstaller.exe")),
            ArtifactKind::Installer
        );

        let mut mysql_aumid_without_metadata = candidate(
            "MySQL Installer - Community",
            "Microsoft.AutoGenerated.{2C7C9013-A7FD-5C12-D811-939AFE2A3592}",
        );
        mysql_aumid_without_metadata.source_kind = SourceKind::StartApps;
        mysql_aumid_without_metadata.launch_kind = LaunchKind::AppUserModelId;
        mysql_aumid_without_metadata.resolved_path = Some(mysql_target.into());
        assert_eq!(
            classify(&mysql_aumid_without_metadata, None),
            ArtifactKind::Installer
        );

        let mut attack_shark = candidate(
            "Attack Shark Software",
            r"D:\1MAIN\Загрузки\ATTACK SHARK X11 SOFT.exe",
        );
        attack_shark.description = Some("Attack Shark Software Setup".into());
        attack_shark.product_name = Some("Attack Shark Software".into());
        assert_eq!(classify(&attack_shark, None), ArtifactKind::Installer);

        let mut windows_sdk = candidate(
            "Windows Software Development Kit - Windows 10.0.19041.5609",
            r"C:\ProgramData\Package Cache\{5f4dc51d-f151-4325-8ba1-8b26169529a9}\winsdksetup.exe",
        );
        windows_sdk.original_filename = Some("winsdksetup.exe".into());
        assert_eq!(
            classify(&windows_sdk, Some("setup")),
            ArtifactKind::Installer
        );
    }

    #[test]
    fn uncorroborated_installer_words_do_not_reclassify_applications() {
        let mut unresolved_aumid = candidate(
            "Advanced Installer",
            "Caphyon.AdvancedInstaller!Application",
        );
        unresolved_aumid.source_kind = SourceKind::StartApps;
        unresolved_aumid.launch_kind = LaunchKind::AppUserModelId;
        unresolved_aumid.description = Some("Advanced Installer".into());
        assert_eq!(
            classify(&unresolved_aumid, Some("AdvancedInstaller.exe")),
            ArtifactKind::Application
        );

        let mut ordinary = candidate(
            "Profile Manager",
            r"C:\Program Files\Profile Manager\ProfileManager.exe",
        );
        ordinary.description = Some("Configure setup and manage user profiles".into());
        assert_eq!(classify(&ordinary, None), ArtifactKind::Application);
    }

    /// Redistributables and self-extracting stubs are installation artifacts, not applications —
    /// but they are no longer *deleted* for it. `visibility` used to reject them, so a word in a
    /// description removed the record from every view; they now land in Installers & Docs.
    #[test]
    fn redistributables_are_installers_rather_than_rejected_records() {
        let vc_redist = candidate(
            "Microsoft Visual C++ 2015-2022 Redistributable (x64)",
            r"C:\ProgramData\Package Cache\{guid}\VC_redist.x64.exe",
        );
        assert_eq!(classify(&vc_redist, None), ArtifactKind::Installer);

        // Real version-resource fields of the stub bundled in KOMPAS-3D's `\support\Adobe\` tree:
        // the vendor's own `OriginalFilename` is what identifies it, on any machine.
        let mut adobe = candidate(
            "Adobe Self Extractor",
            r"E:\Програмки\KOMPAS-3D V19\KOMPAS-Electric V19 x64\support\Adobe\AdbeRdr11000_ru_RU.exe",
        );
        adobe.publisher = Some("Adobe Systems Incorporated".into());
        adobe.product_name = Some("Adobe Self Extractor".into());
        adobe.description = Some("Adobe Self Extractor".into());
        adobe.original_filename = Some("AdobeSelfExtractor.exe".into());
        assert_eq!(
            classify(&adobe, Some("AdobeSelfExtractor.exe")),
            ArtifactKind::Installer
        );
    }

    /// A Steam library entry launches through `steam://rungameid/...`. Steam's shared
    /// redistributables depot describes itself as a redistributable, but it is a platform
    /// component the user cannot run as a setup file — `visibility` demotes it to Auxiliary tools.
    #[test]
    fn steam_library_entries_are_never_installation_artifacts() {
        let mut depot = candidate(
            "Steamworks Common Redistributables",
            "steam://rungameid/228980",
        );
        depot.source_kind = SourceKind::Steam;
        depot.product_name = Some("Steamworks Common Redistributables".into());
        depot.install_location =
            Some(r"C:\Program Files (x86)\Steam\steamapps\common\Steamworks Shared".into());

        assert_eq!(classify(&depot, None), ArtifactKind::Application);
    }

    /// An uninstaller is a maintenance entry, not an installer. `visibility` demotes it to
    /// Auxiliary tools, where it remains visible; classifying it here would move it into a
    /// category that promises the opposite of what it does.
    #[test]
    fn uninstall_targets_are_never_installation_artifacts() {
        for path in [
            r"C:\Program Files\Example\unins000.exe",
            r"C:\Program Files\Example\uninstall.exe",
            r"C:\Program Files\Example\Uninstaller.exe",
        ] {
            assert_eq!(
                classify(&candidate("Uninstall Example", path), None),
                ArtifactKind::Application,
                "{path}"
            );
        }
    }

    /// The rule that used to require a recognizable folder. A download folder is whatever the user
    /// named it — `Загрузки`, `Pobrane`, `D:\Distr` — and `SHGetKnownFolderPath` only resolves the
    /// one the shell knows about, which on the development machine is not the one downloads
    /// actually land in. The vendor's file name is what travels between machines.
    #[test]
    fn setup_executables_are_recognized_outside_any_known_folder() {
        for path in [
            r"F:\Pobrane\sterowniki-setup.exe",
            r"D:\Distr\contabilita_setup_2024.exe",
            r"E:\1MAIN\Загрузки\vendor-installer.exe",
            r"G:\Soft\setup.exe",
        ] {
            assert_eq!(
                classify(&candidate("Installer", path), None),
                ArtifactKind::Installer,
                "{path}"
            );
        }
    }

    /// The Windows Installer engine bundled inside another product's tree — here a WINE
    /// distribution under a PortProton install — is an installation tool wherever it is copied.
    #[test]
    fn a_bundled_windows_installer_engine_is_an_installation_artifact() {
        let mut wine = candidate(
            "Wine Installer",
            r"E:\!С ноута папки\Downloads\portproton\data\dist\WINE_LG_10-20\lib\wine\i386-windows\msiexec.exe",
        );
        wine.publisher = Some("Microsoft Corporation".into());

        assert_eq!(classify(&wine, None), ArtifactKind::Installer);
    }

    /// A folder the shell resolves still corroborates the weaker description-only evidence, which
    /// is what a redirected Downloads is for.
    #[test]
    fn a_resolved_download_folder_corroborates_description_only_evidence() {
        let mut registered = candidate("Vendor Tool", r"D:\Stuff\vendortool.exe");
        registered.source_kind = SourceKind::Registry;
        registered.can_uninstall = true;
        registered.description = Some("Vendor Tool Setup".into());

        // Registered software with a plain path: the description alone must not reclassify it.
        assert_eq!(
            super::super::classify(&registered, None, &MachineFacts::empty()),
            ArtifactKind::Application
        );

        // The same file in the folder this user actually downloads into.
        let resolved = MachineFacts {
            places: crate::catalog::place::PlaceIndex::from_roots(vec![(
                std::path::PathBuf::from(r"D:\Stuff"),
                crate::catalog::place::Place::TransientDrop,
            )]),
            registrations: crate::catalog::machine::Registrations::empty(),
        };
        assert_eq!(
            super::super::classify(&registered, None, &resolved),
            ArtifactKind::Installer
        );
    }

    /// The generic replacement for a vendor cache list: an installed product registered this exact
    /// file as its own setup bundle, so it is an installer whatever it is called and wherever the
    /// vendor chose to keep it.
    #[test]
    fn a_registered_bundle_cache_is_an_installer_without_any_name_evidence() {
        let bundle = r"C:\ProgramData\Package Cache\{5d3c3229}\AacHal.exe";
        let app = candidate("ENE External Device HAL", bundle);

        assert_eq!(classify(&app, None), ArtifactKind::Application);

        let registered = MachineFacts {
            places: crate::catalog::place::PlaceIndex::from_roots(Vec::new()),
            registrations: crate::catalog::machine::Registrations::from_paths(
                Vec::new(),
                vec![bundle.into()],
            ),
        };
        assert_eq!(
            super::super::classify(&app, None, &registered),
            ArtifactKind::Installer
        );
    }

    /// The display name is localized and user-editable, so it may never decide on its own. These
    /// are real products on machines nobody inspected; the old name needles removed them.
    #[test]
    fn products_named_after_installation_words_stay_applications() {
        for (name, path) in [
            (
                "Total Uninstall",
                r"D:\Tools\TotalUninstall\TotalUninst.exe",
            ),
            (
                "Universal USB Installer",
                r"F:\Narzedzia\UniversalUSB\uusb.exe",
            ),
            ("IObit Uninstaller", r"G:\Programme\IObit\IObitUnin.exe"),
            (
                "Sandboxie Plus",
                r"C:\Program Files\Sandboxie-Plus\SandMan.exe",
            ),
        ] {
            assert_eq!(
                classify(&candidate(name, path), None),
                ArtifactKind::Application,
                "{name}"
            );
        }
    }
}
