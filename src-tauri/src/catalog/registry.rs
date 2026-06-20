use super::{
    classify, clean_display_icon, find_executable, stable_id, AppInfo, LaunchKind, SourceKind,
    UninstallTarget,
};
use winreg::RegKey;

pub(super) struct RegistryValues {
    pub display_name: String,
    pub display_icon: Option<String>,
    pub display_version: Option<String>,
    pub publisher: Option<String>,
    pub comments: Option<String>,
    pub install_location: Option<String>,
    pub uninstall_string: Option<String>,
    pub quiet_uninstall_string: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct RegistryMetadata {
    pub name: String,
    pub description: Option<String>,
    pub version: Option<String>,
    pub publisher: Option<String>,
    pub install_location: Option<String>,
    pub uninstall: UninstallTarget,
}

#[derive(Default)]
pub(super) struct RegistryScan {
    pub apps: Vec<AppInfo>,
    pub metadata: Vec<RegistryMetadata>,
}

pub(super) fn scan(hive: winreg::HKEY, subkey: &str) -> RegistryScan {
    let Ok(uninstall) = RegKey::predef(hive).open_subkey(subkey) else {
        return RegistryScan::default();
    };
    let mut result = RegistryScan::default();
    for key in uninstall
        .enum_keys()
        .filter_map(Result::ok)
        .filter_map(|name| uninstall.open_subkey(name).ok())
    {
        let Ok(display_name) = key.get_value("DisplayName") else {
            continue;
        };
        let values = RegistryValues {
            display_name,
            display_icon: key.get_value("DisplayIcon").ok(),
            display_version: key.get_value("DisplayVersion").ok(),
            publisher: key.get_value("Publisher").ok(),
            comments: key.get_value("Comments").ok(),
            install_location: key.get_value("InstallLocation").ok(),
            uninstall_string: key.get_value("UninstallString").ok(),
            quiet_uninstall_string: key.get_value("QuietUninstallString").ok(),
        };
        if let Some(metadata) = metadata_from_values(&values) {
            result.metadata.push(metadata);
        }
        if let Some(app) = from_values(values) {
            result.apps.push(app);
        }
    }
    result
}

pub(super) fn from_values(values: RegistryValues) -> Option<AppInfo> {
    let path = values
        .display_icon
        .as_deref()
        .and_then(clean_display_icon)
        .filter(|path| {
            super::is_launchable(path)
                && !super::is_noise(&values.display_name, &path.to_string_lossy())
        })
        .or_else(|| values.install_location.as_deref().and_then(find_executable))?;
    let path_text = path.to_string_lossy().trim().to_string();
    if values.display_name.trim().is_empty() || super::is_noise(&values.display_name, &path_text) {
        return None;
    }
    let uninstall = uninstall_from_values(&values);
    let can_uninstall = uninstall.is_some();
    let name = values.display_name.trim().to_string();
    Some(AppInfo {
        id: stable_id(&path_text),
        category: classify(&name, &path_text),
        name,
        path: path_text,
        icon_base64: None,
        launch_kind: if path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("lnk"))
        {
            LaunchKind::Shortcut
        } else {
            LaunchKind::Executable
        },
        source_kind: SourceKind::Registry,
        description: clean(values.comments),
        version: clean(values.display_version),
        publisher: clean(values.publisher),
        install_location: clean(values.install_location),
        can_uninstall,
        uninstall,
        resolved_path: None,
        shortcut_icon_path: None,
    })
}

fn metadata_from_values(values: &RegistryValues) -> Option<RegistryMetadata> {
    let name = values.display_name.trim().to_string();
    if name.is_empty() || super::is_invalid_display_name(&name) {
        return None;
    }
    Some(RegistryMetadata {
        name,
        description: clean(values.comments.clone()),
        version: clean(values.display_version.clone()),
        publisher: clean(values.publisher.clone()),
        install_location: clean(values.install_location.clone()),
        uninstall: uninstall_from_values(values)?,
    })
}

fn uninstall_from_values(values: &RegistryValues) -> Option<UninstallTarget> {
    values
        .quiet_uninstall_string
        .as_deref()
        .and_then(split_command)
        .or_else(|| values.uninstall_string.as_deref().and_then(split_command))
        .map(|(executable, arguments)| UninstallTarget::Command {
            executable,
            arguments,
        })
}

fn clean(value: Option<String>) -> Option<String> {
    value
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
}

pub(super) fn split_command(value: &str) -> Option<(String, String)> {
    let value = value.trim();
    if let Some(rest) = value.strip_prefix('"') {
        let end = rest.find('"')?;
        return Some((rest[..end].to_string(), rest[end + 1..].trim().to_string()));
    }
    let lower = value.to_ascii_lowercase();
    let end = lower.find(".exe").map(|index| index + 4)?;
    Some((
        value[..end].trim().to_string(),
        value[end..].trim().to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn values(display_name: &str, display_icon: Option<String>) -> RegistryValues {
        RegistryValues {
            display_name: display_name.into(),
            display_icon,
            display_version: None,
            publisher: None,
            comments: None,
            install_location: None,
            uninstall_string: None,
            quiet_uninstall_string: None,
        }
    }

    #[test]
    fn rejects_resource_only_display_icon() {
        let dir = tempfile::tempdir().unwrap();
        let icon = dir.path().join("app.ico");
        std::fs::write(&icon, []).unwrap();
        assert!(from_values(values(
            "Icon Resource",
            Some(icon.to_string_lossy().into_owned())
        ))
        .is_none());
    }

    #[test]
    fn registry_record_preserves_metadata_and_uninstall_data() {
        let dir = tempfile::tempdir().unwrap();
        let executable = dir.path().join("Codex.exe");
        std::fs::write(&executable, []).unwrap();
        let app = from_values(RegistryValues {
            display_name: "Codex".into(),
            display_icon: Some(format!("{},0", executable.display())),
            display_version: Some("1.2.3".into()),
            publisher: Some("OpenAI".into()),
            comments: Some("Coding agent".into()),
            install_location: Some(r"C:\Apps".into()),
            uninstall_string: Some(r"C:\Apps\uninstall.exe /remove".into()),
            quiet_uninstall_string: None,
        })
        .unwrap();
        assert_eq!(app.version.as_deref(), Some("1.2.3"));
        assert_eq!(app.publisher.as_deref(), Some("OpenAI"));
        assert!(app.uninstall.is_some());
    }

    #[test]
    fn quiet_uninstall_command_has_priority() {
        let dir = tempfile::tempdir().unwrap();
        let executable = dir.path().join("App.exe");
        std::fs::write(&executable, []).unwrap();
        let app = from_values(RegistryValues {
            display_name: "App".into(),
            display_icon: Some(executable.to_string_lossy().into_owned()),
            display_version: None,
            publisher: None,
            comments: None,
            install_location: None,
            uninstall_string: Some(r"C:\Apps\uninstall.exe".into()),
            quiet_uninstall_string: Some(r"C:\Apps\uninstall.exe /quiet".into()),
        })
        .unwrap();
        assert_eq!(
            app.uninstall,
            Some(UninstallTarget::Command {
                executable: r"C:\Apps\uninstall.exe".into(),
                arguments: "/quiet".into(),
            })
        );
    }

    #[test]
    fn uninstall_is_unavailable_without_a_parsable_command() {
        let dir = tempfile::tempdir().unwrap();
        let executable = dir.path().join("App.exe");
        std::fs::write(&executable, []).unwrap();
        let app = from_values(RegistryValues {
            display_name: "App".into(),
            display_icon: Some(executable.to_string_lossy().into_owned()),
            display_version: None,
            publisher: None,
            comments: None,
            install_location: None,
            uninstall_string: None,
            quiet_uninstall_string: Some("not a command".into()),
        })
        .unwrap();
        assert!(!app.can_uninstall);
        assert!(app.uninstall.is_none());
    }

    #[test]
    fn splits_quoted_uninstall_command() {
        assert_eq!(
            split_command(r#""C:\Program Files\App\uninstall.exe" /remove"#),
            Some((
                r"C:\Program Files\App\uninstall.exe".into(),
                "/remove".into()
            ))
        );
    }

    #[test]
    fn preserves_uninstall_metadata_when_display_icon_is_the_uninstaller() {
        let values = RegistryValues {
            display_name: "Steam".into(),
            display_icon: Some(r"C:\Program Files (x86)\Steam\uninstall.exe".into()),
            display_version: None,
            publisher: Some("Valve".into()),
            comments: None,
            install_location: None,
            uninstall_string: Some(r"C:\Program Files (x86)\Steam\uninstall.exe".into()),
            quiet_uninstall_string: None,
        };
        let metadata = metadata_from_values(&values).unwrap();
        assert_eq!(metadata.name, "Steam");
        assert_eq!(metadata.publisher.as_deref(), Some("Valve"));
        assert_eq!(
            metadata.uninstall,
            UninstallTarget::Command {
                executable: r"C:\Program Files (x86)\Steam\uninstall.exe".into(),
                arguments: String::new(),
            }
        );
        assert!(from_values(values).is_none());
    }
}
