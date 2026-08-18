use crate::catalog::{AppInfo, SourceKind, UninstallTarget};
use crate::platform::windows::apps_folder::PackageIdentity;
use std::collections::HashMap;
use std::path::{Component, Path};

pub(super) fn apply(
    app: &mut AppInfo,
    package: &PackageIdentity,
    executables: &HashMap<String, String>,
) {
    let name = package_name(&package.full_name);
    app.source_kind = SourceKind::Msix;
    app.publisher = display_publisher(&name);
    app.version = package_version(&package.full_name);
    app.install_location = package.install_location.clone();
    app.description = Some(name.clone());
    app.product_name = Some(name);
    app.can_uninstall = true;
    app.uninstall = Some(UninstallTarget::Msix {
        package_full_name: package.full_name.clone(),
    });

    let Some(install_location) = app.install_location.as_deref() else {
        return;
    };
    let Some(executable) = executables.get(&app.path.to_lowercase()) else {
        return;
    };
    if let Some(path) = contained_executable(install_location, executable) {
        app.resolved_path = Some(path);
    }
}

fn package_name(full_name: &str) -> String {
    full_name
        .split('_')
        .next()
        .unwrap_or(full_name)
        .trim()
        .to_string()
}

fn package_version(full_name: &str) -> Option<String> {
    full_name
        .split('_')
        .nth(1)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

pub(super) fn display_publisher(package_name: &str) -> Option<String> {
    package_name
        .split('.')
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn contained_executable(install_location: &str, executable: &str) -> Option<String> {
    let root = Path::new(install_location.trim());
    let root_value = root.as_os_str().to_string_lossy();
    if !root.is_absolute() || root_value.starts_with(r"\\") || root_value.starts_with("//") {
        return None;
    }

    let relative = Path::new(executable.trim());
    if relative.as_os_str().is_empty()
        || relative.is_absolute()
        || !relative
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
    {
        return None;
    }

    let mut candidate = root.to_path_buf();
    for component in relative.components() {
        let Component::Normal(component) = component else {
            return None;
        };
        candidate.push(component);
    }
    Some(candidate.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_a_package_full_name_into_vendor_and_version() {
        let full_name = "OpenAI.Codex_26.727.4816.0_x64__2p2nqsd0c76g0";

        assert_eq!(package_name(full_name), "OpenAI.Codex");
        assert_eq!(package_version(full_name).as_deref(), Some("26.727.4816.0"));
        assert_eq!(display_publisher("OpenAI.Codex").as_deref(), Some("OpenAI"));
    }

    #[test]
    fn rejects_escaping_and_non_executable_manifest_targets() {
        for executable in [r"..\Outside.exe", r"C:\Outside.exe", "app/readme.txt", ""] {
            assert_eq!(
                contained_executable(r"C:\Program Files\WindowsApps\Codex", executable),
                None,
                "unsafe manifest target was accepted: {executable}"
            );
        }
    }

    #[test]
    fn joins_a_relative_manifest_target_onto_the_install_root() {
        assert_eq!(
            contained_executable(r"C:\Program Files\WindowsApps\Codex", "app/ChatGPT.exe")
                .as_deref(),
            Some(r"C:\Program Files\WindowsApps\Codex\app\ChatGPT.exe")
        );
    }

    #[test]
    fn rejects_a_network_install_root() {
        assert_eq!(
            contained_executable(r"\\server\share\Codex", "app/ChatGPT.exe"),
            None
        );
    }
}
