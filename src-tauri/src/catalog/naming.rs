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

fn clean_folder_name(value: &str) -> String {
    value
        .replace(['_', '-'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

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
