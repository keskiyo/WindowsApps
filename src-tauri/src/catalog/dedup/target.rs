//! What a catalog entry actually launches, normalized so two records can be compared.
//!
//! Path normalization lives here rather than in `identity` because every consumer of it is
//! asking the same question — "do these point at the same thing on disk" — and the answer has to
//! be identical for the blocking index, the evidence rules and the id derivation alike.

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
    // A generic interpreter host (cmd.exe, powershell.exe, python.exe, …) is not an identifying
    // target. Distinct tools that merely share an interpreter — a Node.js prompt and a VS
    // Developer Command Prompt, both `cmd.exe /k <different>.bat` — would otherwise resolve to
    // the same target, collide on one canonical identity, and over-merge into a single card.
    // Fall back to the shortcut's own path for those. Self-contained targets (`.msc`, `.cpl`,
    // real application exes) are unaffected.
    (!is_generic_interpreter_host(target)).then_some(target)
}

/// The application a Squirrel package launches, as `<package root>\<executable>`.
///
/// Squirrel installs put the program in a versioned folder beside their updater stub —
/// `<root>\Update.exe` and `<root>\app-1.0.9250\Discord.exe` — and register *both* in the Start
/// Menu under the product name. The two records share nothing the other evidence rules can use:
/// different targets, different versions, and not even a publisher (the stub is signed "GitHub",
/// not by the vendor), so Discord, Slack and GitHub Desktop each occupied two catalog cards. Both
/// records collapse to the same key here, which is an identity match — the version folder changes
/// with every update, so it must not be part of it.
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

/// The stub half of the pair above. It launches the application correctly and stays a usable card
/// on its own, but once it merges with the application's own record that record is the better
/// representative: the stub's target, version, publisher and command line all describe the updater.
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

/// Interpreter/host executables whose behaviour is defined by their arguments, so the host path
/// alone does not identify the tool. Start Apps retain these targets as launch evidence, but every
/// identity and merge consumer passes through this predicate.
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

/// Curated equivalence for built-in Windows shell items whose localized Start-App and English
/// shortcut resolve to different, non-comparable targets (a shell CLSID vs a PIDL-only shortcut,
/// or control.exe with an applet name). Returns a shared token so the pair collapses into one
/// card. Language-independent: keyed on stable AUMIDs and applet names, not display text.
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
    // "Administrative Tools" launches control.exe /name Microsoft.AdministrativeTools.
    if target.ends_with("control.exe") && args.contains("microsoft.administrativetools") {
        return Some("windows:admintools");
    }
    // "File Explorer" ships as a PIDL-only shortcut (no readable target); its .lnk file name
    // is English on every locale, so it is a safe key for this fixed shell item.
    if target.is_empty() && normalize_name(&app.name) == "file explorer" {
        return Some("windows:explorer");
    }
    // The Run dialog also ships as a PIDL-only "Run" shortcut with no readable target, matching the
    // localized `Microsoft.Windows.Shell.RunDialog` Start-App above.
    if target.is_empty() && normalize_name(&app.name) == "run" {
        return Some("windows:run");
    }
    None
}

/// True when the app's resolved launch target is a built-in Windows tool: a `.msc`/`.cpl`
/// snap-in, a binary under the Windows system directories, or a shell CLSID target
/// (`::{…}`, e.g. Control Panel). Used to scope localized-name preference to system tools.
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
