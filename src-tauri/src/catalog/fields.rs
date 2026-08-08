use crate::catalog::{AppInfo, LaunchKind};
use std::path::Path;

#[derive(Clone, Copy)]
pub(in crate::catalog) enum Field {
    Name,
    FileName,
    Path,
    Prose,
}

pub(in crate::catalog) struct MarkerFields {
    name: String,
    file_names: String,
    paths: String,
    prose: String,
}

impl MarkerFields {
    pub(in crate::catalog) fn from_app(app: &AppInfo) -> Self {
        let launch_path = (app.launch_kind != LaunchKind::AppUserModelId).then_some(&app.path);
        let resolved = app.resolved_path.as_deref().unwrap_or_default();
        let original = app.original_filename.as_deref().unwrap_or_default();
        let name = app.name.to_lowercase();
        let description = app
            .description
            .as_deref()
            .unwrap_or_default()
            .to_lowercase();
        let product = app
            .product_name
            .as_deref()
            .unwrap_or_default()
            .to_lowercase();

        let file_names = [
            launch_path.map(String::as_str).unwrap_or_default(),
            resolved,
            original,
        ]
        .iter()
        .map(|value| file_name_of(value))
        .collect::<Vec<_>>()
        .join(" ");

        let paths = [
            launch_path.map(String::as_str).unwrap_or_default(),
            resolved,
            app.install_location.as_deref().unwrap_or_default(),
        ]
        .join(" ")
        .to_lowercase()
        .replace('/', r"\");

        Self {
            prose: format!("{name} {description} {product}"),
            name,
            file_names,
            paths,
        }
    }

    pub(in crate::catalog) fn matches(&self, field: Field, needle: &str) -> bool {
        match field {
            Field::Name => self.name.contains(needle),
            Field::FileName => self.file_names.contains(needle),
            Field::Path => self.paths.contains(needle),
            Field::Prose => self.prose.contains(needle),
        }
    }

    pub(in crate::catalog) fn any(&self, field: Field, needles: &[&str]) -> bool {
        needles.iter().any(|needle| self.matches(field, needle))
    }
}

fn file_name_of(value: &str) -> String {
    let trimmed = value.trim().trim_matches('"');
    Path::new(trimmed)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(trimmed)
        .to_lowercase()
}
