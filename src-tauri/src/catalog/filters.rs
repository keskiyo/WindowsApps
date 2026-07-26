//! Noise/maintenance filtering: predicates that decide whether a discovered entry is a real,
//! launchable application versus an installer, uninstaller, documentation, helper, or runtime
//! component that should be dropped or demoted.

use std::path::{Path, PathBuf};
pub(super) fn clean_display_icon(value: &str) -> Option<PathBuf> {
    let trimmed = value.trim().trim_start_matches('\\').trim();
    let path = if let Some(rest) = trimmed.strip_prefix('"') {
        rest.split('"').next().unwrap_or(rest)
    } else {
        trimmed.split(',').next().unwrap_or(trimmed)
    };
    let path = path.trim().trim_matches('"');
    (!path.is_empty()).then(|| PathBuf::from(path))
}

pub(super) fn is_invalid_display_name(name: &str) -> bool {
    let name = name.trim();
    name.is_empty() || name.to_lowercase().starts_with("ms-resource:") || name.contains('\u{fffd}')
}

pub(super) fn is_noise(name: &str, path: &str) -> bool {
    is_maintenance_entry(name, path, None)
}

pub(super) fn is_maintenance_entry(name: &str, path: &str, resolved_path: Option<&str>) -> bool {
    if is_invalid_display_name(name) {
        return true;
    }
    is_documentation_name(name)
        || is_maintenance_path(path)
        || resolved_path.is_some_and(is_maintenance_path)
        || is_maintenance_text(name)
}

pub(super) fn is_maintenance_path(path: &str) -> bool {
    if is_runtime_internal_path(path) {
        return true;
    }
    let path_buf = Path::new(path);
    if path_buf
        .file_stem()
        .and_then(|stem| stem.to_str())
        .is_some_and(is_helper_executable_stem)
    {
        return true;
    }
    if path_buf.extension().is_some_and(|extension| {
        [
            "ico", "dll", "mui", "cpl", "chm", "pdf", "html", "htm", "txt", "rtf", "md", "url",
            "hlp", "xml", "log", "ini",
        ]
        .iter()
        .any(|value| extension.eq_ignore_ascii_case(value))
    }) {
        return true;
    }
    if path_buf
        .file_stem()
        .is_some_and(|stem| is_installer_file_name(&stem.to_string_lossy()))
    {
        return true;
    }
    is_maintenance_text(path)
}

pub(super) fn is_helper_executable_stem(stem: &str) -> bool {
    let normalized = stem.to_lowercase();
    matches!(
        normalized.as_str(),
        "git-lfs"
            | "git-credential-manager"
            | "gettext"
            | "printf_gettext"
            | "printf_ngettext"
            | "envsubst"
            | "msgfmt"
    )
}

pub(super) fn is_runtime_internal_path(path: &str) -> bool {
    let normalized = path.replace('/', r"\").to_lowercase();
    [
        r"\.cache\codex-runtimes\",
        r"\.codex\.sandbox-bin\",
        r"\appdata\local\openai\codex\runtimes\",
    ]
    .iter()
    .any(|marker| normalized.contains(marker))
}

/// Junk-detection for installer/updater executables by file name (stem, no extension).
/// Splits the stem into alphanumeric tokens to catch `setup-app`, `app-installer`,
/// `setup_x64`, and also matches glued names like `AppSetup` via prefix/suffix.
pub(crate) fn is_installer_file_name(stem: &str) -> bool {
    let lower = stem.to_lowercase();
    // 7-Zip installers are named like `7z2501-x64`; the real app is `7zFM`/`7zG`/`7z`.
    if lower.starts_with("7z")
        && lower[2..]
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_digit())
    {
        return true;
    }
    const TOKENS: [&str; 8] = [
        "setup",
        "unins",
        "unins000",
        "updater",
        "bootstrapper",
        "установщик",
        "деинсталляция",
        "удаление",
    ];
    let has_token = lower
        .split(|character: char| !character.is_alphanumeric())
        .filter(|token| !token.is_empty())
        // `contains` catches install/installer/instaler/installation, uninstall, and
        // vcredist2005_x64 / vc_redist / redistributables, including misspellings.
        .any(|token| {
            token.contains("instal")
                || token.contains("redist")
                || token.ends_with("setup")
                || TOKENS.contains(&token)
        });
    if has_token {
        return true;
    }
    ["setup", "install", "uninstall"]
        .iter()
        .any(|marker| lower.ends_with(marker))
}

/// Junk-detection for documentation / website shortcut display names.
/// Matches whole words at the start or end (so "HelpDesk Pro" survives) plus a few
/// multi-word phrases anywhere in the normalized name.
pub(super) fn is_documentation_name(name: &str) -> bool {
    const WORDS: [&str; 25] = [
        "documentation",
        "docs",
        "readme",
        "manual",
        "help",
        "faq",
        "license",
        "licence",
        "eula",
        "changelog",
        "tutorial",
        "website",
        "homepage",
        "support",
        "samples",
        "sample",
        "sdk",
        "example",
        "examples",
        "demo",
        "документация",
        "справка",
        "руководство",
        "лицензия",
        "сайт",
    ];
    const PHRASES: [&str; 9] = [
        "release notes",
        "what's new",
        "home page",
        "getting started",
        "visit website",
        "support center",
        "заметки о выпуске",
        "что нового",
        "веб сайт",
    ];
    let normalized = super::dedup::normalize_name(name);
    if PHRASES.iter().any(|phrase| normalized.contains(phrase)) {
        return true;
    }
    let words = normalized.split_whitespace().collect::<Vec<_>>();
    match (words.first(), words.last()) {
        (Some(first), Some(last)) => WORDS.contains(first) || WORDS.contains(last),
        _ => false,
    }
}

pub(super) fn is_maintenance_text(value: &str) -> bool {
    let value = value.to_lowercase();
    [
        "uninstall",
        "unins000",
        "installer",
        "installation notes",
        "setup.exe",
        "updater",
        "update.exe",
        "repair.exe",
        "bootstrap",
        "remove ",
        "delete ",
        "удалить",
        "деинстал",
        "microsoft visual c++ update",
        "hotfix",
        "security update",
        "redistributable",
        "subprocess",
        "kb[",
        "file://",
    ]
    .iter()
    .any(|needle| value.contains(needle))
}
