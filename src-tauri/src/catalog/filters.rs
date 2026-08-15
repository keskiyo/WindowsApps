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
    is_invalid_display_name(name) || is_structural_path(path, false)
}

pub(super) fn is_maintenance_entry(name: &str, path: &str, _resolved_path: Option<&str>) -> bool {
    if is_invalid_display_name(name) {
        return true;
    }
    is_maintenance_path(path)
}

pub(super) fn is_maintenance_path(path: &str) -> bool {
    is_structural_path(path, true)
}

pub(super) fn is_uninstall_action_name(name: &str) -> bool {
    name.to_lowercase()
        .split(|character: char| !character.is_alphanumeric())
        .find(|token| !token.is_empty())
        .is_some_and(|first| matches!(first, "uninstall" | "удалить" | "деинсталляция"))
}

pub(super) fn carries_uninstall_switch(arguments: Option<&str>) -> bool {
    let Some(arguments) = arguments else {
        return false;
    };
    arguments
        .to_lowercase()
        .split(|character: char| character.is_whitespace() || character == '"')
        .map(|token| token.trim_start_matches(['/', '-']))
        .any(|token| {
            matches!(token, "x" | "uninstall" | "remove")
                || token.starts_with("x{")
                || token.starts_with("remove=all")
        })
}

pub(super) fn is_uninstall_target_path(path: &Path) -> bool {
    if !path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
    {
        return false;
    }
    let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
        return false;
    };
    let stem = stem.to_ascii_lowercase();
    matches!(stem.as_str(), "uninstall" | "uninstaller" | "unins")
        || stem.strip_prefix("unins").is_some_and(|suffix| {
            !suffix.is_empty() && suffix.chars().all(|value| value.is_ascii_digit())
        })
}

fn is_structural_path(path: &str, reject_installer_file: bool) -> bool {
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
    let is_executable = path_buf
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"));
    if reject_installer_file
        && is_executable
        && path_buf
            .file_stem()
            .is_some_and(|stem| is_installer_file_name(&stem.to_string_lossy()))
    {
        return true;
    }
    false
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

pub(crate) fn is_installer_file_name(stem: &str) -> bool {
    let lower = stem.to_lowercase();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_uninstall_action_is_named_by_its_first_word_never_by_a_product_name() {
        for name in [
            "Uninstall Go",
            "Удалить Ассистент",
            "Деинсталляция Foo",
            "uninstall example editor",
        ] {
            assert!(is_uninstall_action_name(name), "{name}");
        }
        for name in [
            "Revo Uninstaller",
            "IObit Uninstaller",
            "Uninstaller Pro",
            "Total Uninstall",
        ] {
            assert!(!is_uninstall_action_name(name), "{name}");
        }
    }

    #[test]
    fn an_uninstall_switch_is_read_as_a_whole_token() {
        for arguments in [
            "/x{8C0F4E5D-1B33-4C1B-9E4C-3C1A2F9E77A1}",
            "/X {8C0F4E5D-1B33-4C1B-9E4C-3C1A2F9E77A1}",
            "--uninstall --channel=stable",
            "-uninstall",
            "/uninstall /silent",
            "/qn REMOVE=ALL",
            "/remove",
        ] {
            assert!(carries_uninstall_switch(Some(arguments)), "{arguments}");
        }
        for arguments in [
            "--channel=stable",
            "/xyz",
            "-max-fps 60",
            "--extract-to \"C:\\Temp\"",
            "",
        ] {
            assert!(!carries_uninstall_switch(Some(arguments)), "{arguments}");
        }
        assert!(!carries_uninstall_switch(None));
    }
}
