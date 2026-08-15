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
        let exe_stem = exe_stem(exe_source);
        Self {
            associations: associations.of(&exe_stem).to_vec(),
            name_exact: normalize(&app.name),
            name_base: normalize(strip_qualifiers(&app.name)),
            name_tokens: tokenize(&app.name),
            product_tokens: tokenize(app.product_name.as_deref().unwrap_or_default()),
            exe_stem,
            path_blob,
            description: fold(app.description.as_deref().unwrap_or_default()),
            publisher: fold(app.publisher.as_deref().unwrap_or_default()),
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

fn exe_stem(value: &str) -> String {
    fold(
        Path::new(value.trim().trim_matches('"'))
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or_default(),
    )
}
