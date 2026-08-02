//! Install-tree dominance: which executable in a product folder is the product, and which ones
//! only ship with it.
//!
//! This replaces guessing from a vocabulary. `iconv.exe`, `intelliphp.ls`, `\git\mingw64\`,
//! `\.codeium\` are all one machine's answer to a question every machine asks differently, and a
//! vendor nobody enumerated ships components under names nobody listed. The relation that does
//! generalize is structural and costs no extra I/O: **an executable nested below a directory that
//! already holds a discovered executable is a component of whatever lives up there.**
//!
//! Real trees this was measured against:
//!
//! - `CrystalDiskInfo9_7_1Aoi\DiskInfo64A.exe` beside `CrystalDiskInfo9_7_1Aoi\CdiResource\AlertMail48.exe`;
//! - an extracted Office 2007 tree, `SETUP.EXE` at the root beside `ENTERPRISE.WW\OSE.EXE` and
//!   `OFFICE.RU-RU\DWTRIG20.EXE`.
//!
//! Depth alone is not enough, because a folder can be a shelf rather than a product: measured on
//! this machine, `test system\` holds a loose OCCT portable *and* a subfolder with AIDA64's, and
//! `Windhawk\` ships VSCodium beside its own binary. So the executable above must carry the **same
//! publisher**; two vendors under one folder are two products, not a product and its component.
//!
//! Three further escapes keep a real application out of this: an executable that names its own
//! folder is the product of its own subtree, anything Windows or another catalog record points at
//! is referenced software, and a file with no publisher at all is left alone rather than guessed
//! about.

use super::machine::Registrations;
use super::place::normalized_path as normalize;
use super::{AppInfo, LaunchKind, SourceKind, VisibilityClass, VisibilityReason};
use std::collections::{HashMap, HashSet};
use std::path::Path;

pub(super) fn demote_nested_components(apps: &mut [AppInfo], registrations: &Registrations) {
    let anchors = publishers_by_directory(apps);
    let referenced = referenced_executables(apps);

    for app in apps.iter_mut() {
        // Only a bare file found on disk is judged this way. Everything Windows registered has its
        // own evidence and is handled by the ordinary visibility rules.
        if app.source_kind != SourceKind::Portable || app.launch_kind == LaunchKind::AppUserModelId
        {
            continue;
        }
        let (Some(path), Some(publisher)) = (executable_path(app), publisher_of(app)) else {
            // Without a publisher there is nothing to prove the file belongs to the product above
            // it, and guessing here would hide an application. Left alone on purpose.
            continue;
        };
        // `App Paths` is a vendor declaring "this is the executable users start". It protects a
        // registered product that happens to live deep in a tree — `D:\...\7-Zip\7zFM.exe` — from
        // being read as a component of whatever sits above it.
        if referenced.contains(&normalize(path))
            || registrations.is_launchable(path)
            || names_its_own_folder(path)
        {
            continue;
        }
        if !has_ancestor_from_the_same_publisher(path, &publisher, &anchors) {
            continue;
        }
        app.visibility_class = VisibilityClass::Auxiliary;
        if !app
            .visibility_reasons
            .contains(&VisibilityReason::ProductComponent)
        {
            app.visibility_reasons
                .push(VisibilityReason::ProductComponent);
        }
    }
}

/// Every directory that directly holds a discovered executable, mapped to the publishers of the
/// executables in it.
fn publishers_by_directory(apps: &[AppInfo]) -> HashMap<String, HashSet<String>> {
    let mut directories: HashMap<String, HashSet<String>> = HashMap::new();
    for app in apps {
        let (Some(path), Some(publisher)) = (executable_path(app), publisher_of(app)) else {
            continue;
        };
        let Some(directory) = Path::new(path).parent() else {
            continue;
        };
        let directory = normalize(&directory.to_string_lossy());
        if !directory.is_empty() {
            directories.entry(directory).or_default().insert(publisher);
        }
    }
    directories
}

fn publisher_of(app: &AppInfo) -> Option<String> {
    app.publisher
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_lowercase)
}

/// Executables something else points at: a Start Menu shortcut's target, a registry record's
/// executable, a Start App's launch target. A component is precisely what nothing points at.
fn referenced_executables(apps: &[AppInfo]) -> HashSet<String> {
    let mut referenced = HashSet::new();
    for app in apps {
        if let Some(target) = app.resolved_path.as_deref() {
            referenced.insert(normalize(target));
        }
        if app.source_kind != SourceKind::Portable {
            referenced.insert(normalize(&app.path));
        }
    }
    referenced
}

/// `Foo\Foo.exe` is the product of its own folder, however deep that folder sits. This is the same
/// test portable discovery already uses to accept an executable without metadata.
///
/// The architecture suffix is dropped first, because shipping `Foo\Foo_x64.exe` is the same claim
/// as shipping `Foo\Foo.exe`. Without that, GPU Shark — `FurMark_win64\gpushark\gpushark_x64.exe`,
/// a Geeks3D tool bundled beside Geeks3D's FurMark — read as a component of the product above it.
fn names_its_own_folder(path: &str) -> bool {
    let path = Path::new(path);
    let Some(stem) = path.file_stem().map(|value| value.to_string_lossy()) else {
        return false;
    };
    let stem = without_architecture_suffix(&stem);
    path.parent()
        .and_then(Path::file_name)
        .map(|folder| folder.to_string_lossy())
        .is_some_and(|folder| {
            super::naming::normalized_portable_name(&folder)
                == super::naming::normalized_portable_name(&stem)
        })
}

/// Drops trailing architecture tokens (`gpushark_x64` → `gpushark`). Only whole tokens count, so a
/// glued name such as `DiskInfo64A` keeps every character it has.
fn without_architecture_suffix(stem: &str) -> String {
    let mut tokens = stem
        .split(|character: char| !character.is_alphanumeric())
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();
    while tokens.len() > 1
        && tokens.last().is_some_and(|token| {
            matches!(
                token.to_lowercase().as_str(),
                "x64" | "x86" | "x32" | "64" | "32" | "win64" | "win32" | "amd64" | "arm64"
            )
        })
    {
        tokens.pop();
    }
    tokens.join(" ")
}

/// Walks strictly upward from the file's own directory — a sibling in the same directory is a peer,
/// not a parent product — and requires the executable up there to come from the same vendor.
///
/// The publisher is what separates a product's own component from an unrelated application that
/// merely lives in a subfolder. Measured: `CrystalDiskInfo9_7_1Aoi\CdiResource\AlertMail48.exe` and
/// the `DiskInfo64A.exe` above it are both Crystal Dew World, so the first is a component; but
/// `test system\OCCT 14.2.6 Portable.exe` is OCCT while the AIDA64 portable in the subfolder below
/// it is FinalWire, and a folder holding two vendors' tools is a shelf, not a product.
fn has_ancestor_from_the_same_publisher(
    path: &str,
    publisher: &str,
    anchors: &HashMap<String, HashSet<String>>,
) -> bool {
    Path::new(path)
        .parent()
        .into_iter()
        .flat_map(|directory| directory.ancestors().skip(1))
        .filter_map(|ancestor| anchors.get(&normalize(&ancestor.to_string_lossy())))
        .any(|publishers| publishers.contains(publisher))
}

fn executable_path(app: &AppInfo) -> Option<&str> {
    [app.resolved_path.as_deref(), Some(app.path.as_str())]
        .into_iter()
        .flatten()
        .map(str::trim)
        .find(|value| {
            Path::new(value)
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{AppCategory, ArtifactKind};

    fn published(name: &str, path: &str, publisher: &str) -> AppInfo {
        let mut app = portable(name, path);
        app.publisher = Some(publisher.into());
        app
    }

    fn portable(name: &str, path: &str) -> AppInfo {
        AppInfo {
            id: path.into(),
            name: name.into(),
            path: path.into(),
            icon_base64: None,
            artifact_kind: ArtifactKind::Application,
            category: AppCategory::Other,
            launch_kind: LaunchKind::Executable,
            source_kind: SourceKind::Portable,
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
            visibility_class: VisibilityClass::Primary,
            visibility_score: 0,
            visibility_reasons: Vec::new(),
        }
    }

    fn classes(apps: &[AppInfo]) -> Vec<(&str, VisibilityClass)> {
        apps.iter()
            .map(|app| (app.name.as_str(), app.visibility_class))
            .collect()
    }

    /// The two trees this rule was measured against, with no vocabulary involved in either.
    /// Publishers are the real `CompanyName` values read from those files.
    #[test]
    fn nested_executables_below_a_products_own_folder_are_components() {
        let mut apps = vec![
            published(
                "CrystalDiskInfo",
                r"E:\Програмки\SSD&HDD\CrystalDiskInfo9_7_1Aoi\DiskInfo64A.exe",
                "Crystal Dew World",
            ),
            published(
                "AlertMail48",
                r"E:\Програмки\SSD&HDD\CrystalDiskInfo9_7_1Aoi\CdiResource\AlertMail48.exe",
                "Crystal Dew World",
            ),
            published(
                "Setup",
                r"E:\Програмки\Office\MICROSOFT.OFFICE.2007\SETUP.EXE",
                "Microsoft Corporation",
            ),
            published(
                "OSE",
                r"E:\Програмки\Office\MICROSOFT.OFFICE.2007\ENTERPRISE.WW\OSE.EXE",
                "Microsoft Corporation",
            ),
            published(
                "DWTRIG20",
                r"E:\Програмки\Office\MICROSOFT.OFFICE.2007\OFFICE.RU-RU\DWTRIG20.EXE",
                "Microsoft Corporation",
            ),
        ];

        demote_nested_components(&mut apps, &Registrations::empty());

        assert_eq!(
            classes(&apps),
            vec![
                ("CrystalDiskInfo", VisibilityClass::Primary),
                ("AlertMail48", VisibilityClass::Auxiliary),
                ("Setup", VisibilityClass::Primary),
                ("OSE", VisibilityClass::Auxiliary),
                ("DWTRIG20", VisibilityClass::Auxiliary),
            ]
        );
        assert!(apps[1]
            .visibility_reasons
            .contains(&VisibilityReason::ProductComponent));
    }

    /// A folder of independent portable applications must survive a loose executable sitting beside
    /// them: each application names its own folder, which is what tells it apart from a component.
    #[test]
    fn independent_applications_in_their_own_folders_are_kept() {
        let mut apps = vec![
            portable("Loose tool", r"D:\Portable\tool.exe"),
            portable("Notepad3", r"D:\Portable\Notepad3\Notepad3.exe"),
            portable("HxD", r"D:\Portable\HxD\HxD.exe"),
        ];

        demote_nested_components(&mut apps, &Registrations::empty());

        assert!(apps
            .iter()
            .all(|app| app.visibility_class == VisibilityClass::Primary));
    }

    /// A shelf holding several vendors' tools is not a product tree. Measured on two real folders
    /// where the earlier depth-only rule would have hidden the application the user was looking
    /// for: an AIDA64 portable under a folder whose loose executable is OCCT's, and the VSCodium
    /// that Windhawk ships beside its own binary.
    #[test]
    fn a_different_vendors_application_in_a_subfolder_is_not_a_component() {
        let mut apps = vec![
            published(
                "OCCT",
                r"E:\Програмки\test system\OCCT 14.2.6 Portable.exe",
                "OCCT",
            ),
            published(
                "AIDA64 Extreme",
                r"E:\Програмки\test system\AIDA64 Extreme Edition 6.85.6305 Beta Portable\aida64.exe",
                "FinalWire Ltd.",
            ),
            published(
                "Windhawk",
                r"D:\разный хлам\Windhawk\windhawk.exe",
                "Ramen Software",
            ),
            published(
                "VSCodium",
                r"D:\разный хлам\Windhawk\UI\VSCodium.exe",
                "VSCodium",
            ),
        ];

        demote_nested_components(&mut apps, &Registrations::empty());

        assert_eq!(
            classes(&apps),
            vec![
                ("OCCT", VisibilityClass::Primary),
                ("AIDA64 Extreme", VisibilityClass::Primary),
                ("Windhawk", VisibilityClass::Primary),
                ("VSCodium", VisibilityClass::Primary),
            ]
        );
    }

    /// Shipping `Foo\Foo_x64.exe` claims the folder as much as `Foo\Foo.exe` does. GPU Shark is a
    /// Geeks3D tool bundled beside Geeks3D's FurMark, so the publisher gate cannot separate them —
    /// naming its own folder is what does.
    #[test]
    fn an_executable_naming_its_folder_with_an_architecture_suffix_is_kept() {
        let mut apps = vec![
            published(
                "FurMark(GUI)",
                r"D:\разный хлам\FurMark_2.10.2_win64\FurMark_win64\FurMark_GUI.exe",
                "Geeks3D",
            ),
            published(
                "gpushark",
                r"D:\разный хлам\FurMark_2.10.2_win64\FurMark_win64\gpushark\gpushark_x64.exe",
                "Geeks3D",
            ),
        ];

        demote_nested_components(&mut apps, &Registrations::empty());

        assert_eq!(apps[1].visibility_class, VisibilityClass::Primary);
    }

    /// The suffix is dropped only as a whole token, so a glued name keeps every character and the
    /// measured components stay components.
    #[test]
    fn an_architecture_suffix_is_only_dropped_as_a_whole_token() {
        assert_eq!(without_architecture_suffix("gpushark_x64"), "gpushark");
        assert_eq!(without_architecture_suffix("WinMTR-x86"), "WinMTR");
        assert_eq!(without_architecture_suffix("DiskInfo64A"), "DiskInfo64A");
        assert_eq!(without_architecture_suffix("AlertMail48"), "AlertMail48");
        assert_eq!(without_architecture_suffix("x64"), "x64");
    }

    /// Without a publisher there is nothing proving the file belongs to the product above it, and
    /// guessing would hide an application. The conservative answer is to leave it alone.
    #[test]
    fn an_executable_without_a_publisher_is_left_alone() {
        let mut apps = vec![
            published("Anchor", r"D:\Vendor\anchor.exe", "Vendor"),
            portable("Unknown", r"D:\Vendor\extras\unknown.exe"),
        ];

        demote_nested_components(&mut apps, &Registrations::empty());

        assert_eq!(apps[1].visibility_class, VisibilityClass::Primary);
    }

    /// A shortcut or a registered product pointing at the executable makes it referenced software,
    /// whatever its depth. Without this, the portable twin of a Start Menu entry would be demoted
    /// and the sticky component reason would follow the merged card into Auxiliary tools.
    #[test]
    fn a_referenced_executable_is_never_a_component() {
        let mut shortcut = portable(
            "Vendor App",
            r"C:\ProgramData\Microsoft\Windows\Start Menu\Programs\Vendor App.lnk",
        );
        shortcut.source_kind = SourceKind::StartMenu;
        shortcut.launch_kind = LaunchKind::Shortcut;
        shortcut.resolved_path = Some(r"D:\Vendor\runtime\vendorapp.exe".into());

        let mut apps = vec![
            published("Vendor Launcher", r"D:\Vendor\launcher.exe", "Vendor Ltd"),
            published(
                "Vendor App",
                r"D:\Vendor\runtime\vendorapp.exe",
                "Vendor Ltd",
            ),
            shortcut,
        ];

        demote_nested_components(&mut apps, &Registrations::empty());

        assert_eq!(apps[1].visibility_class, VisibilityClass::Primary);
    }

    /// Siblings in one directory are peers. Only nesting demotes.
    #[test]
    fn executables_sharing_one_directory_are_not_demoted() {
        let mut apps = vec![
            published("First", r"D:\Tools\first.exe", "Vendor Ltd"),
            published("Second", r"D:\Tools\second.exe", "Vendor Ltd"),
        ];

        demote_nested_components(&mut apps, &Registrations::empty());

        assert!(apps
            .iter()
            .all(|app| app.visibility_class == VisibilityClass::Primary));
    }
}
