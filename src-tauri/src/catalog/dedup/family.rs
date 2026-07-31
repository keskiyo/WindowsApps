//! Display-name and publisher normalization.
//!
//! Named `family` rather than `naming` on purpose: `catalog::naming` already exists one level up
//! and owns portable-executable naming, which is a different question.

use crate::catalog::AppInfo;

pub(in crate::catalog) fn normalize_name(name: &str) -> String {
    name.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// Architecture markers are noise for identity: the 64-bit and 32-bit build of one tool are a
/// single application to a launcher. Stripped as whole tokens (surrounding parentheses ignored)
/// from anywhere in the name, so `Windows PowerShell ISE (x86)`, `x64 Native Tools …` and
/// `Foo x64` all reduce to the same family as their 64-bit sibling.
fn strip_architecture_markers(name: &str) -> String {
    const MARKERS: &[&str] = &[
        "x86", "x64", "x86_x64", "x64_x86", "amd64", "arm64", "ia64", "win32", "win64", "32bit",
        "64bit", "32-bit", "64-bit", "wow", "wow64",
    ];
    name.split_whitespace()
        .filter(|token| {
            let cleaned = token.trim_matches(|character| character == '(' || character == ')');
            !MARKERS.contains(&cleaned)
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub(in crate::catalog) fn normalized_product_family(name: &str) -> String {
    let value = strip_architecture_markers(&normalize_name(name));
    if let Some((family, suffix)) = value.split_once(" - ") {
        let generic_suffix = ["proxy utility", "desktop app", "application"]
            .iter()
            .any(|marker| suffix.starts_with(marker));
        let has_version = suffix.chars().any(|character| character.is_ascii_digit());
        if generic_suffix && has_version {
            return family.trim().to_string();
        }
    }
    let mut family = version_family(&value).trim().to_string();
    if family.ends_with(" version") {
        family.truncate(family.len() - " version".len());
    }
    if let Some(rest) = family.strip_prefix("mozilla firefox") {
        let rest = rest.trim();
        if rest.is_empty() || rest.starts_with('(') {
            family = "firefox".into();
        }
    }
    canonical_windows_tool_family(&family)
}

fn canonical_windows_tool_family(family: &str) -> String {
    const ALIASES: [(&str, &[&str]); 10] = [
        ("task manager", &["task manager", "диспетчер задач"]),
        ("control panel", &["control panel", "панель управления"]),
        ("registry editor", &["registry editor", "редактор реестра"]),
        ("device manager", &["device manager", "диспетчер устройств"]),
        ("services", &["services", "службы"]),
        ("event viewer", &["event viewer", "просмотр событий"]),
        (
            "computer management",
            &["computer management", "управление компьютером"],
        ),
        (
            "disk management",
            &["disk management", "управление дисками"],
        ),
        (
            "system information",
            &["system information", "сведения о системе"],
        ),
        ("command prompt", &["command prompt", "командная строка"]),
    ];
    ALIASES
        .iter()
        .find_map(|(canonical, aliases)| aliases.contains(&family).then_some(*canonical))
        .unwrap_or(family)
        .to_string()
}

fn version_family(name: &str) -> &str {
    let Some((family, suffix)) = name.rsplit_once(' ') else {
        return name;
    };
    if !suffix.starts_with(|character: char| character.is_ascii_digit()) {
        return name;
    }
    let numeric_segments = suffix
        .split(|character: char| !character.is_ascii_digit())
        .filter(|segment| !segment.is_empty())
        .count();
    if numeric_segments >= 2 {
        family
    } else {
        name
    }
}

pub(super) fn launcher_product_family(name: &str) -> &str {
    name.strip_suffix(" launcher").unwrap_or(name).trim()
}

pub(super) fn helper_variant_family(value: &str) -> String {
    let mut family = launcher_product_family(value).to_string();
    for suffix in [
        " helper",
        " updater",
        " update",
        " crash reporter",
        " crashhandler",
        " service",
    ] {
        if family.ends_with(suffix) {
            family.truncate(family.len() - suffix.len());
            break;
        }
    }
    family.trim().to_string()
}

pub(super) fn normalized_publisher(value: Option<&str>) -> String {
    let value = value.unwrap_or_default().trim();
    // An X.500 certificate subject is not a display publisher. Packaged entries report the signing
    // certificate ("CN=AMD") while the desktop entry of the same product reports the marketing name
    // ("Advanced Micro Devices, Inc."); comparing them as strings manufactures a conflict between
    // two records of one vendor. `start_apps::display_publisher` already swaps it for the package
    // vendor where the AppX metadata is available — everywhere else it means "unknown", which
    // conflicts with nothing.
    // `get(..3)` rather than `value[..3]`: publisher strings come from the registry and from PE
    // metadata, so they are arbitrary UTF-8. Byte-slicing one that starts with a multi-byte
    // character — a Cyrillic vendor name, for instance — lands inside a code point and panics.
    if value
        .get(..3)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("CN="))
    {
        return String::new();
    }
    value
        .to_lowercase()
        .split(|character: char| !character.is_alphanumeric())
        .filter(|token| !token.is_empty() && !PUBLISHER_LEGAL_SUFFIXES.contains(token))
        .collect()
}

/// Legal-form suffixes stripped so "Foo" and "Foo Inc." compare equal. Matched as whole tokens,
/// never as substrings: the old `.replace("inc", "")` mangled real publisher names — "Vincent
/// Labs" became "vt labs", "Incredible" became "redible", and a publisher of literally "Inc."
/// collapsed to the empty string, which silently disabled `publishers_conflict` (the empty
/// publisher matches everything). Token matching keeps the intended stripping while leaving any
/// name that merely contains these letters intact.
const PUBLISHER_LEGAL_SUFFIXES: &[&str] = &[
    "corporation",
    "incorporated",
    "limited",
    "company",
    "corp",
    "inc",
    "llc",
];

/// A 32-bit build of a tool, recognized so a merged x86/x64 pair can surface the 64-bit one.
pub(super) fn is_32bit_variant(app: &AppInfo) -> bool {
    let name = app.name.to_lowercase();
    let arch_token = name
        .split(|character: char| !character.is_alphanumeric())
        .any(|token| matches!(token, "x86" | "wow" | "wow64"));
    let paths_are_32bit = [
        app.path.as_str(),
        app.resolved_path.as_deref().unwrap_or(""),
    ]
    .iter()
    .any(|value| {
        let lower = value.to_lowercase();
        lower.contains(r"\syswow64\") || lower.contains(r"\program files (x86)\")
    });
    arch_token || name.contains("32-bit") || paths_are_32bit
}

pub(super) fn version_key(version: Option<&str>) -> Vec<u64> {
    version
        .unwrap_or_default()
        .split(|character: char| !character.is_ascii_digit())
        .filter_map(|segment| segment.parse().ok())
        .collect()
}
