//! Text/path marker predicates used by `classify_visibility`. Each answers one narrow "is this a
//! …" question about an app's name, path, and metadata; keeping them here leaves the parent module
//! to the scoring flow.

use crate::catalog::AppInfo;
use std::path::Path;

pub(super) fn executable_matches_product(app: &AppInfo) -> bool {
    let stem = Path::new(&app.path)
        .file_stem()
        .and_then(|value| value.to_str())
        .map(normalized_identity_text)
        .unwrap_or_default();
    if stem.len() < 3 {
        return false;
    }
    [&app.name, app.product_name.as_deref().unwrap_or_default()]
        .iter()
        .map(|value| normalized_identity_text(value))
        .any(|value| value == stem || value.starts_with(&stem))
}

fn normalized_identity_text(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

pub(super) fn is_bundled_toolchain_path(path: &str) -> bool {
    contains_any(
        path,
        &[
            r"\.vscode\extensions\",
            r"\git\usr\bin\",
            r"\git\mingw32\",
            r"\git\mingw64\",
            r"\codex-runtimes\",
            r"\sdk\samples\",
        ],
    )
}

/// A command environment opens a configured interpreter shell (a VS developer prompt, a
/// Node.js prompt) rather than an application. A regular user launching software does not pick
/// these; they belong in Auxiliary tools. Plain `Windows PowerShell` / `Command Prompt` — an
/// interpreter with no arguments — stay primary.
pub(super) fn is_command_environment(app: &AppInfo) -> bool {
    let name = app.name.to_lowercase();
    const NAME_MARKERS: &[&str] = &[
        "developer command prompt",
        "developer powershell",
        "native tools command prompt",
        "cross tools command prompt",
        "command prompt for vs",
        "command line client",
        // A setup/toolchain action ("Install Additional Tools for Node.js") is not an app.
        "additional tools for node",
        // A diagnostic "safe mode" launcher opens the real app in a recovery mode — not the way
        // a user launches it day to day.
        "safe mode",
        "безопасный режим",
    ];
    if NAME_MARKERS.iter().any(|marker| name.contains(marker)) {
        return true;
    }
    let has_arguments = app
        .launch_arguments
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty());
    has_arguments
        && app
            .resolved_path
            .as_deref()
            .is_some_and(is_interpreter_host_path)
}

fn is_interpreter_host_path(path: &str) -> bool {
    let file = Path::new(path.trim().trim_matches('"'))
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(path)
        .to_ascii_lowercase();
    matches!(
        file.as_str(),
        "cmd.exe"
            | "powershell.exe"
            | "pwsh.exe"
            | "wscript.exe"
            | "cscript.exe"
            | "mshta.exe"
            | "rundll32.exe"
            | "python.exe"
            | "pythonw.exe"
            | "py.exe"
            | "mysql.exe"
            | "node.exe"
            | "wsl.exe"
    )
}

fn contains_any(value: &str, markers: &[&str]) -> bool {
    markers.iter().any(|marker| value.contains(marker))
}

pub(super) fn is_installer_or_uninstaller(value: &str) -> bool {
    contains_any(
        value,
        &[
            " uninstall",
            "uninstall ",
            "unins000",
            " setup",
            "setup.exe",
            "installer",
            "tsetup",
            "vcredist",
            "redist",
        ],
    )
}

pub(super) fn is_documentation(value: &str) -> bool {
    contains_any(
        value,
        &[
            " faq",
            "faqs",
            "documentation",
            "installation notes",
            "release notes",
            "readme",
            "manual",
            "документация",
            "справка",
            "руководство",
        ],
    )
}

pub(super) fn is_maintenance_executable(value: &str) -> bool {
    contains_any(
        value,
        &[
            "update-service",
            "update_service",
            " updater",
            "update.exe",
            "crashhandler",
            "crash handler",
            "crashpad",
            "uninstall.exe",
            // "Reset preferences and cache files"-style shortcuts run the app in a wipe mode
            // instead of opening it; they belong with maintenance, not the player itself.
            "reset preferences",
            "reset cache",
            "reset config",
            "reset settings",
            "сброс настроек",
            "сброс кэш",
        ],
    )
}

pub(super) fn has_runtime_path(path: &str) -> bool {
    contains_any(
        path,
        &[
            r"\bin\",
            r"\lib\",
            r"\runtime\",
            r"\jre\",
            r"\sdk\",
            r"\plugins\",
            r"\resources\",
            r"\node_modules\",
        ],
    )
}

pub(super) fn is_product_component(value: &str) -> bool {
    contains_any(
        value,
        &[
            "iconv.exe",
            "intelliphp.ls",
            "language server",
            "openjdk platform binary",
            // Oracle Java runtime entries ("Java(TM) Platform SE", "Java(TM) SE") are the JRE, a
            // runtime component, not an application a user launches.
            "java(tm)",
            "the curl executable",
            "openssl command",
            "credential manager",
            "gettext",
            "git-lfs",
            "git large file storage",
            "sandbox",
            "compiler",
            " helper",
            "_helper",
            "-helper",
            " service.exe",
            " daemon",
            // A bundled utility executable (`AstUtil.exe`, `*Util.exe`) is a maintenance/config
            // helper of its product, not the application a user launches.
            "util.exe",
        ],
    )
}
