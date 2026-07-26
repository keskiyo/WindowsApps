//! Portable-app display-name derivation: choosing a human name from the executable stem, parent
//! folder, and embedded product metadata, plus the version suffix split out of a versioned stem.

/// Chooses a portable app's display name. When the executable name carries no product
/// identity (`32.exe`, `x64.exe`, …) the embedded metadata is unreliable (a Yandex-derived
/// binary inside a "Крипто 4" folder reports ProductName "Yandex"), so the parent folder
/// wins. Otherwise use the metadata product name, falling back to the cleaned file stem.
pub(super) fn portable_display_name(
    stem: &str,
    parent: Option<&str>,
    product_name: Option<&str>,
) -> String {
    let folder = if is_generic_executable_stem(stem) {
        parent
            .map(clean_folder_name)
            .filter(|value| !value.is_empty() && !is_generic_executable_stem(value))
    } else {
        None
    };
    folder
        .or_else(|| {
            product_name
                .map(str::to_string)
                .filter(|value| !is_generic_product_name(value))
        })
        .unwrap_or_else(|| clean_portable_name(stem))
}

/// Light cleanup for a folder used as a display name: normalize separators and whitespace,
/// but keep trailing numbers ("Крипто 4" must not lose its "4", unlike version-suffix
/// stripping applied to executable stems).
fn clean_folder_name(value: &str) -> String {
    value
        .replace(['_', '-'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// An executable name that identifies the platform/arch, not the product (`32.exe`,
/// `x64.exe`, `win32.exe`, purely numeric). For these the parent folder is a better name.
fn is_generic_executable_stem(stem: &str) -> bool {
    let normalized = stem.trim().to_lowercase();
    if normalized.is_empty() {
        return true;
    }
    if normalized
        .chars()
        .all(|character| character.is_ascii_digit())
    {
        return true;
    }
    matches!(
        normalized.as_str(),
        "x86"
            | "x64"
            | "x86_64"
            | "amd64"
            | "ia64"
            | "arm64"
            | "win32"
            | "win64"
            | "app"
            | "application"
            | "launcher"
            | "main"
            | "run"
            | "start"
            | "program"
    )
}

fn is_generic_product_name(value: &str) -> bool {
    [
        "godot engine",
        "electron",
        "chromium",
        "application",
        "java",
        "python",
        "runtime",
        "launcher",
        "windows application",
    ]
    .iter()
    .any(|generic| value.trim().eq_ignore_ascii_case(generic))
}

pub(super) fn normalized_portable_name(value: &str) -> String {
    clean_portable_name(value)
        .chars()
        .filter(|character| character.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn clean_portable_name(value: &str) -> String {
    let trimmed = value.trim();
    let version_start = trimmed
        .char_indices()
        .find(|(index, character)| {
            *index > 0 && character.is_ascii_digit() && trimmed[..*index].ends_with(['-', '_', ' '])
        })
        .map(|(index, _)| index.saturating_sub(1));
    version_start
        .map_or(trimmed, |index| &trimmed[..index])
        .replace(['_', '-'], " ")
        .trim()
        .to_string()
}

pub(super) fn portable_version_from_stem(value: &str) -> Option<String> {
    let trimmed = value.trim();
    let start = trimmed.char_indices().find_map(|(index, character)| {
        (index > 0 && character.is_ascii_digit() && trimmed[..index].ends_with(['-', '_', ' ']))
            .then_some(index)
    })?;
    let version = trimmed[start..].trim();
    (!version.is_empty()).then(|| version.to_string())
}
