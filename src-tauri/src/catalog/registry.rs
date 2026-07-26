use super::{
    classify, clean_display_icon, find_executable_named, stable_id, AppInfo, LaunchKind,
    SourceKind, UninstallTarget,
};
use crate::platform::windows::uninstall_registry;
pub(super) use crate::platform::windows::uninstall_registry::RegistryEntry as RegistryValues;

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

pub(super) fn scan() -> RegistryScan {
    let mut result = RegistryScan::default();
    for values in uninstall_registry::entries()
        .into_iter()
        .map(expand_registry_paths)
    {
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
    if values.system_component {
        return None;
    }
    let path = values
        .display_icon
        .as_deref()
        .and_then(clean_display_icon)
        .filter(|path| {
            super::is_launchable(path)
                && !super::is_noise(&values.display_name, &path.to_string_lossy())
        })
        .or_else(|| {
            values
                .install_location
                .as_deref()
                .and_then(|location| find_executable_named(location, Some(&values.display_name)))
        })?;
    let path_text = path.to_string_lossy().trim().to_string();
    if values.display_name.trim().is_empty() || super::is_noise(&values.display_name, &path_text) {
        return None;
    }
    let uninstall = uninstall_from_values(&values);
    let can_uninstall = uninstall.is_some();
    let name = values.display_name.trim().to_string();
    let executable_metadata = crate::platform::windows::executable_metadata::read(&path);
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
        description: clean(values.comments).or(executable_metadata.description),
        version: clean(values.display_version).or(executable_metadata.version),
        publisher: clean(values.publisher).or(executable_metadata.publisher),
        product_name: executable_metadata.product_name,
        original_filename: executable_metadata.original_filename,
        install_location: clean(values.install_location),
        can_uninstall,
        uninstall,
        resolved_path: None,
        shortcut_icon_path: None,
        launch_arguments: None,
        canonical_identity: None,
        visibility_class: Default::default(),
        visibility_score: 0,
        visibility_reasons: Vec::new(),
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

/// Registry paths routinely arrive as `REG_EXPAND_SZ` (`%ProgramFiles%\App\app.exe`), which
/// `winreg` hands back unexpanded. Nothing downstream expands them, so an unexpanded value
/// fails `is_launchable` (`Path::is_file`) and the whole entry is dropped — the application is
/// registered with Windows but silently absent from the catalog. Expand once, here, before any
/// on-disk resolution is attempted.
fn expand_registry_paths(mut values: RegistryValues) -> RegistryValues {
    values.display_icon = values
        .display_icon
        .map(|value| crate::platform::windows::exec_target::expand_env(&value));
    values.install_location = values
        .install_location
        .map(|value| crate::platform::windows::exec_target::expand_env(&value));
    values
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
            system_component: false,
        }
    }

    // `REG_EXPAND_SZ` values arrive unexpanded. Before expansion these entries failed the
    // `is_file` check and vanished from the catalog even though Windows had them registered.
    // The variable name is unique to this test: the test binary is multi-threaded and the
    // process environment is shared.
    #[test]
    fn expands_environment_variables_in_registry_paths() {
        let dir = tempfile::tempdir().unwrap();
        std::env::set_var("WINAPPS_TEST_REG_ROOT", dir.path());
        let raw = values("Editor", Some(r"%WINAPPS_TEST_REG_ROOT%\Editor.exe".into()));

        let expanded = expand_registry_paths(raw);

        assert_eq!(
            expanded.display_icon.as_deref(),
            Some(dir.path().join("Editor.exe").to_string_lossy().as_ref())
        );
    }

    #[test]
    fn an_expanded_registry_path_produces_a_catalog_entry() {
        let dir = tempfile::tempdir().unwrap();
        let executable = dir.path().join("Editor.exe");
        std::fs::write(&executable, []).unwrap();
        std::env::set_var("WINAPPS_TEST_REG_ENTRY_ROOT", dir.path());
        let raw = values(
            "Editor",
            Some(r"%WINAPPS_TEST_REG_ENTRY_ROOT%\Editor.exe".into()),
        );

        // Unexpanded the same value resolves to nothing and the entry is dropped.
        assert!(from_values(values(
            "Editor",
            Some(r"%WINAPPS_TEST_REG_ENTRY_ROOT%\Editor.exe".into()),
        ))
        .is_none());

        let app = from_values(expand_registry_paths(raw)).expect("expanded path is launchable");
        assert_eq!(app.name, "Editor");
        assert_eq!(app.path, executable.to_string_lossy());
    }

    #[test]
    fn system_component_is_metadata_only() {
        let dir = tempfile::tempdir().unwrap();
        let executable = dir.path().join("Runtime.exe");
        std::fs::write(&executable, []).unwrap();
        let mut values = values(
            "Runtime component",
            Some(executable.to_string_lossy().into_owned()),
        );
        values.system_component = true;

        assert!(from_values(values).is_none());
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
            system_component: false,
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
            system_component: false,
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
            system_component: false,
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
            system_component: false,
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
