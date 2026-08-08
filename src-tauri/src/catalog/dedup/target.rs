use super::arguments::squirrel_process_start;
use super::family::normalize_name;
use crate::catalog::{AppInfo, LaunchKind, SourceKind};
use std::path::{Component, Path};

pub(in crate::catalog) fn normalize_path(value: &str) -> String {
    let expanded = expand_windows_env(value.trim().trim_matches('"'));
    let separated = expanded.replace('/', "\\");
    let mut normalized = std::path::PathBuf::new();
    for component in Path::new(&separated).components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if !normalized.pop() {
                    normalized.push(component.as_os_str());
                }
            }
            _ => normalized.push(component.as_os_str()),
        }
    }
    normalized
        .to_string_lossy()
        .trim_end_matches('\\')
        .to_lowercase()
}

fn expand_windows_env(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    let mut rest = value;
    while let Some(start) = rest.find('%') {
        result.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        let Some(end) = after.find('%') else {
            result.push_str(&rest[start..]);
            return result;
        };
        let name = &after[..end];
        if let Ok(replacement) = std::env::var(name) {
            result.push_str(&replacement);
        } else {
            result.push('%');
            result.push_str(name);
            result.push('%');
        }
        rest = &after[end + 1..];
    }
    result.push_str(rest);
    result
}

pub(super) fn parent_path(path: &str) -> Option<String> {
    path.rsplit_once('\\').map(|(parent, _)| parent.to_string())
}
pub(super) fn launch_target(app: &AppInfo) -> Option<&str> {
    let target = app.resolved_path.as_deref()?;
    (!is_generic_interpreter_host(target)).then_some(target)
}

pub(super) fn squirrel_package(app: &AppInfo) -> Option<String> {
    let target = normalize_path(app.resolved_path.as_deref()?);
    let (parent, file) = target.rsplit_once('\\')?;
    if file == "update.exe" {
        let started = squirrel_process_start(app.launch_arguments.as_deref())?;
        return Some(format!("{parent}\\{started}"));
    }
    let (root, folder) = parent.rsplit_once('\\')?;
    is_squirrel_version_folder(folder).then(|| format!("{root}\\{file}"))
}

pub(super) fn is_squirrel_stub(app: &AppInfo) -> bool {
    app.resolved_path
        .as_deref()
        .is_some_and(|target| normalize_path(target).ends_with("\\update.exe"))
        && squirrel_process_start(app.launch_arguments.as_deref()).is_some()
}

fn is_squirrel_version_folder(folder: &str) -> bool {
    folder
        .strip_prefix("app-")
        .is_some_and(|version| version.starts_with(|value: char| value.is_ascii_digit()))
}

pub(super) fn is_generic_interpreter_host(path: &str) -> bool {
    const HOSTS: &[&str] = &[
        "cmd.exe",
        "powershell.exe",
        "pwsh.exe",
        "wscript.exe",
        "cscript.exe",
        "rundll32.exe",
        "mshta.exe",
        "conhost.exe",
        "control.exe",
        "explorer.exe",
        "mmc.exe",
        "python.exe",
        "pythonw.exe",
        "py.exe",
        "node.exe",
        "java.exe",
        "javaw.exe",
        "mysql.exe",
        "wsl.exe",
        "bash.exe",
        "sh.exe",
    ];
    let file = Path::new(path.trim().trim_matches('"'))
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(path)
        .to_ascii_lowercase();
    HOSTS.contains(&file.as_str())
}

pub(super) fn steam_app_id(app: &AppInfo) -> Option<&str> {
    if app.source_kind != SourceKind::Steam {
        return None;
    }
    app.path.strip_prefix("steam://rungameid/")
}

pub(super) fn system_tool_alias(app: &AppInfo) -> Option<&'static str> {
    if app.launch_kind == LaunchKind::AppUserModelId {
        match app.path.trim().to_lowercase().as_str() {
            "microsoft.windows.explorer" => return Some("windows:explorer"),
            "microsoft.windows.administrativetools" => return Some("windows:admintools"),
            "microsoft.windows.controlpanel" => return Some("windows:controlpanel"),
            "microsoft.windows.remotedesktop" => return Some("windows:remotedesktop"),
            "microsoft.windows.shell.rundialog" => return Some("windows:run"),
            _ => {}
        }
    }
    let target = app
        .resolved_path
        .as_deref()
        .unwrap_or_default()
        .to_lowercase();
    let args = app
        .launch_arguments
        .as_deref()
        .unwrap_or_default()
        .to_lowercase();
    if target.ends_with("control.exe") && args.contains("microsoft.administrativetools") {
        return Some("windows:admintools");
    }
    if target.is_empty() && normalize_name(&app.name) == "file explorer" {
        return Some("windows:explorer");
    }
    if target.is_empty() && normalize_name(&app.name) == "run" {
        return Some("windows:run");
    }
    None
}

pub(super) fn is_system_tool_target(app: &AppInfo) -> bool {
    let Some(target) = app.resolved_path.as_deref() else {
        return false;
    };
    let normalized = normalize_path(target);
    normalized.starts_with("::{")
        || normalized.ends_with(".msc")
        || normalized.ends_with(".cpl")
        || normalized.contains("\\windows\\system32\\")
        || normalized.contains("\\windows\\syswow64\\")
}
