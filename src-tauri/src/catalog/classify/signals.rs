use super::super::AppInfo;
use std::collections::HashSet;
use std::path::Path;

#[derive(Clone, Copy)]
pub(super) enum Field {
    Name,
    Product,
    ExeEq,
    ExeContains,
    Path,
    Desc,
    Publisher,
    Feature,
}

impl Field {
    pub(super) fn id(self) -> &'static str {
        match self {
            Self::Name => "name",
            Self::Product => "product",
            Self::ExeEq | Self::ExeContains => "exe",
            Self::Path => "path",
            Self::Desc => "description",
            Self::Publisher => "publisher",
            Self::Feature => "feature",
        }
    }
}

pub(super) struct Signals {
    name_tokens: HashSet<String>,
    product_tokens: HashSet<String>,
    exe_stem: String,
    path_blob: String,
    description: String,
    publisher: String,
    name_path_blob: String,
}

impl Signals {
    pub(super) fn from_app(app: &AppInfo) -> Self {
        let resolved = app.resolved_path.as_deref().unwrap_or_default();
        let install = app.install_location.as_deref().unwrap_or_default();
        let path_blob = format!("{} {} {}", app.path, resolved, install).to_lowercase();
        let exe_source = app
            .original_filename
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or(if resolved.is_empty() {
                &app.path
            } else {
                resolved
            });
        Self {
            name_tokens: tokenize(&app.name),
            product_tokens: tokenize(app.product_name.as_deref().unwrap_or_default()),
            exe_stem: exe_stem(exe_source),
            path_blob,
            description: app
                .description
                .as_deref()
                .unwrap_or_default()
                .to_lowercase(),
            publisher: app.publisher.as_deref().unwrap_or_default().to_lowercase(),
            name_path_blob: format!("{} {} {}", app.name, app.path, resolved).to_lowercase(),
        }
    }

    pub(super) fn from_name_path(name: &str, path: &str) -> Self {
        Self {
            name_tokens: tokenize(name),
            product_tokens: HashSet::new(),
            exe_stem: exe_stem(path),
            path_blob: path.to_lowercase(),
            description: String::new(),
            publisher: String::new(),
            name_path_blob: format!("{name} {path}").to_lowercase(),
        }
    }

    pub(super) fn matches(&self, field: Field, needle: &str) -> bool {
        match field {
            Field::Name => all_words_present(needle, &self.name_tokens),
            Field::Product => all_words_present(needle, &self.product_tokens),
            Field::ExeEq => !self.exe_stem.is_empty() && self.exe_stem == needle,
            Field::ExeContains => !self.exe_stem.is_empty() && self.exe_stem.contains(needle),
            Field::Path => self.path_blob.contains(needle),
            Field::Desc => self.description.contains(needle),
            Field::Publisher => self.publisher.contains(needle),
            Field::Feature => self.name_path_blob.contains(needle),
        }
    }
}

fn tokenize(value: &str) -> HashSet<String> {
    value
        .to_lowercase()
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
    Path::new(value.trim().trim_matches('"'))
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default()
        .to_lowercase()
}
