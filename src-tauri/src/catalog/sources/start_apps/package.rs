use crate::catalog::{AppInfo, SourceKind, UninstallTarget};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Component, Path};

pub(super) const QUERY: &str = r#"
Get-AppxPackage | ForEach-Object {
    $package = $_
    $applications = @()
    try {
        $applications = @(
            (Get-AppxPackageManifest -Package $package.PackageFullName -ErrorAction Stop).Package.Applications.Application |
                ForEach-Object { [pscustomobject]@{ Id = [string]$_.Id; Executable = [string]$_.Executable } }
        )
    } catch {}
    [pscustomobject]@{
        PackageFullName = $package.PackageFullName
        PackageFamilyName = $package.PackageFamilyName
        Name = $package.Name
        Publisher = $package.Publisher
        Version = [string]$package.Version
        InstallLocation = $package.InstallLocation
        Applications = $applications
    }
} | ConvertTo-Json -Compress -Depth 5
"#;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct PackageApplicationRow {
    id: String,
    #[serde(default)]
    executable: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct PackageRow {
    package_full_name: String,
    package_family_name: String,
    name: String,
    publisher: Option<String>,
    version: Option<String>,
    install_location: Option<String>,
    #[serde(default)]
    applications: Vec<PackageApplicationRow>,
}

pub(super) fn enrich_packages(apps: &mut [AppInfo], json: &str) {
    let packages: Vec<PackageRow> = super::parse_rows(json).unwrap_or_default();
    let by_family = packages
        .iter()
        .map(|package| (package.package_family_name.to_lowercase(), package))
        .collect::<HashMap<_, _>>();

    for app in apps {
        let Some((family, application_id)) = app.path.rsplit_once('!') else {
            continue;
        };
        let family = family.to_lowercase();
        let application_id = application_id.to_string();
        let Some(package) = by_family.get(&family) else {
            continue;
        };
        apply_package_metadata(app, package, &application_id);
    }
}

fn apply_package_metadata(app: &mut AppInfo, package: &PackageRow, application_id: &str) {
    app.source_kind = SourceKind::Msix;
    app.publisher = display_publisher(&package.name, package.publisher.clone());
    app.version = non_empty(package.version.clone());
    app.install_location = non_empty(package.install_location.clone());
    app.description = Some(package.name.clone());
    app.product_name = Some(package.name.clone());
    app.can_uninstall = true;
    app.uninstall = Some(UninstallTarget::Msix {
        package_full_name: package.package_full_name.clone(),
    });

    let Some(install_location) = app.install_location.as_deref() else {
        return;
    };
    let Some(application) = package
        .applications
        .iter()
        .find(|application| application.id.eq_ignore_ascii_case(application_id))
    else {
        return;
    };
    if let Some(executable) = contained_executable(install_location, &application.executable) {
        app.resolved_path = Some(executable);
    }
}

fn non_empty(value: Option<String>) -> Option<String> {
    value.filter(|value| !value.trim().is_empty())
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

pub(super) fn display_publisher(package_name: &str, publisher: Option<String>) -> Option<String> {
    let publisher = non_empty(publisher.map(|value| value.trim().to_string()));
    if publisher
        .as_deref()
        .is_some_and(|value| value.starts_with("CN="))
    {
        return package_name
            .split('.')
            .next()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
    }
    publisher
}
