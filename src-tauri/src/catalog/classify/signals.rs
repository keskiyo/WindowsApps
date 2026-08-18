use super::super::machine::Associations;
use super::super::AppInfo;
use std::collections::HashSet;
use std::path::Path;

#[derive(Clone, Copy)]
pub(super) enum Field {
    Association,
    Name,
    NameEq,
    Product,
    ExeEq,
    ExeContains,
    Path,
    Desc,
    Publisher,
}

impl Field {
    pub(super) fn id(self) -> &'static str {
        match self {
            Self::Association => "association",
            Self::Name | Self::NameEq => "name",
            Self::Product => "product",
            Self::ExeEq | Self::ExeContains => "exe",
            Self::Path => "path",
            Self::Desc => "description",
            Self::Publisher => "publisher",
        }
    }

    #[cfg(test)]
    pub(super) fn is_whole_value(self) -> bool {
        matches!(self, Self::NameEq | Self::ExeEq | Self::Association)
    }
}

pub(super) struct Signals {
    associations: Vec<String>,
    name_exact: String,
    name_base: String,
    name_tokens: HashSet<String>,
    product_tokens: HashSet<String>,
    exe_stem: String,
    path_blob: String,
    description: String,
    publisher: String,
}

impl Signals {
    pub(super) fn from_app(app: &AppInfo, associations: &Associations) -> Self {
        let resolved = app.resolved_path.as_deref().unwrap_or_default();
        let install = app.install_location.as_deref().unwrap_or_default();
        let path_blob = fold(&format!("{} {} {}", app.path, resolved, install));
        let exe_source = app
            .original_filename
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or(if resolved.is_empty() {
                &app.path
            } else {
                resolved
            });
        let exe_stem = product_evidence(exe_stem(exe_source));
        let product = product_evidence(fold(app.product_name.as_deref().unwrap_or_default()));
        Self {
            associations: associations.of(&exe_stem).to_vec(),
            name_exact: normalize(&app.name),
            name_base: normalize(strip_qualifiers(&app.name)),
            name_tokens: tokenize(&app.name),
            product_tokens: tokenize(&product),
            exe_stem,
            path_blob,
            description: product_evidence(fold(app.description.as_deref().unwrap_or_default())),
            publisher: product_evidence(fold(app.publisher.as_deref().unwrap_or_default())),
        }
    }

    pub(super) fn from_name_path(name: &str, path: &str) -> Self {
        Self {
            associations: Vec::new(),
            name_exact: normalize(name),
            name_base: normalize(strip_qualifiers(name)),
            name_tokens: tokenize(name),
            product_tokens: HashSet::new(),
            exe_stem: exe_stem(path),
            path_blob: fold(path),
            description: String::new(),
            publisher: String::new(),
        }
    }

    pub(super) fn matches(&self, field: Field, needle: &str) -> bool {
        match field {
            Field::Association => self.associations.iter().any(|token| token == needle),
            Field::Name => all_words_present(needle, &self.name_tokens),
            Field::NameEq => {
                !self.name_exact.is_empty()
                    && (self.name_exact == needle || self.name_base == needle)
            }
            Field::Product => all_words_present(needle, &self.product_tokens),
            Field::ExeEq => !self.exe_stem.is_empty() && self.exe_stem == needle,
            Field::ExeContains => !self.exe_stem.is_empty() && self.exe_stem.contains(needle),
            Field::Path => self.path_blob.contains(needle),
            Field::Desc => self.description.contains(needle),
            Field::Publisher => self.publisher.contains(needle),
        }
    }
}

fn fold(value: &str) -> String {
    value
        .to_lowercase()
        .chars()
        .map(|character| match character {
            '\u{b5}' | '\u{3bc}' => 'u',
            other => other,
        })
        .collect()
}

fn strip_qualifiers(value: &str) -> &str {
    let mut base = value.trim();
    while let Some(open) = base
        .strip_suffix(')')
        .and_then(|inner| inner.rfind('('))
        .filter(|open| *open > 0)
    {
        base = base[..open].trim_end();
    }
    base
}

fn normalize(value: &str) -> String {
    fold(&value.split_whitespace().collect::<Vec<_>>().join(" "))
}

fn tokenize(value: &str) -> HashSet<String> {
    fold(value)
        .split(|c: char| !c.is_alphanumeric())
        .filter(|token| !token.is_empty())
        .map(str::to_owned)
        .collect()
}

fn all_words_present(needle: &str, tokens: &HashSet<String>) -> bool {
    let mut parts = needle
        .split(|c: char| !c.is_alphanumeric())
        .filter(|part| !part.is_empty())
        .peekable();
    parts.peek().is_some() && parts.all(|part| tokens.contains(part))
}

const PACKAGING_METADATA: &[&str] = &[
    "installshield",
    "acresso",
    "flexera",
    "inno setup",
    "nullsoft",
    "nsis",
    "_isicores",
];

fn product_evidence(value: String) -> String {
    if PACKAGING_METADATA
        .iter()
        .any(|toolkit| value.contains(toolkit))
    {
        return String::new();
    }
    value
}

fn exe_stem(value: &str) -> String {
    let trimmed = value.trim().trim_matches('"');
    let carrier = if trimmed.len() > 4 && trimmed[trimmed.len() - 4..].eq_ignore_ascii_case(".mui")
    {
        &trimmed[..trimmed.len() - 4]
    } else {
        trimmed
    };
    fold(
        Path::new(carrier)
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or_default(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_state::cached_app;

    // Windows reports a localized resource stub as the original file name, and taking the last
    // extension off `MSPAINT.EXE.MUI` leaves `mspaint.exe`, which matches no executable rule.
    #[test]
    fn a_localized_resource_stub_still_names_its_executable() {
        assert_eq!(exe_stem("MSPAINT.EXE.MUI"), "mspaint");
        assert_eq!(exe_stem("sapisvr.exe.mui"), "sapisvr");
        assert_eq!(exe_stem("DPInst.exe.Mui"), "dpinst");
        assert_eq!(exe_stem("ClientConsole.EXE"), "clientconsole");
        assert_eq!(exe_stem(r"C:\Program Files\App\editor.exe"), "editor");
    }

    // An InstallShield shortcut reports the packaging toolkit as publisher, product and
    // description, so scoring it would file every such product under whatever the toolkit says.
    #[test]
    fn packaging_toolkit_metadata_is_not_product_evidence() {
        let mut app = cached_app("ABBYY FineReader PDF 15", r"C:\Menu\FineReader.lnk");
        app.publisher = Some("Acresso Software Inc.".into());
        app.product_name = Some("InstallShield".into());
        app.description = Some("InstallShield".into());
        app.original_filename = Some("_IsIcoRes.exe".into());

        let signals = Signals::from_app(&app, &Associations::empty());

        assert!(!signals.matches(Field::Publisher, "acresso"));
        assert!(!signals.matches(Field::Product, "installshield"));
        assert!(!signals.matches(Field::Desc, "installshield"));
        assert!(!signals.matches(Field::ExeContains, "isicores"));
    }

    #[test]
    fn ordinary_metadata_still_counts() {
        let mut app = cached_app("Editor", r"C:\Program Files\Editor\editor.exe");
        app.publisher = Some("Example Corp".into());
        app.product_name = Some("Example Editor".into());

        let signals = Signals::from_app(&app, &Associations::empty());

        assert!(signals.matches(Field::Publisher, "example corp"));
        assert!(signals.matches(Field::Product, "example editor"));
        assert!(signals.matches(Field::ExeEq, "editor"));
    }
}
