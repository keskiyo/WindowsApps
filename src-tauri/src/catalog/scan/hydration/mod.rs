mod icon;
mod queue;

pub(crate) use queue::HydrationQueue;

use crate::catalog::AppInfo;
use icon::hydrate_icon;
use serde::Serialize;
use std::path::Path;

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AppHydrationPatch {
    pub id: String,
    pub generation: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_base64: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publisher: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_filename: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub install_location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub can_uninstall: Option<bool>,
}

pub(crate) struct HydrationOutcome {
    pub patch: AppHydrationPatch,
    pub written_icon: Option<(String, String)>,
}

pub(crate) fn hydrate_one(app_data_dir: &Path, app: &AppInfo, generation: u64) -> HydrationOutcome {
    let target = app.resolved_path.as_deref().unwrap_or(&app.path);
    let metadata = Path::new(target)
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
        .then(|| crate::platform::windows::executable_metadata::read(Path::new(target)));
    let icon = hydrate_icon(app_data_dir, app);
    let patch = AppHydrationPatch {
        id: app.id.clone(),
        generation,
        icon_base64: icon.data_url,
        description: app.description.clone().or_else(|| {
            metadata
                .as_ref()
                .and_then(|value| value.description.clone())
        }),
        version: app
            .version
            .clone()
            .or_else(|| metadata.as_ref().and_then(|value| value.version.clone())),
        publisher: app
            .publisher
            .clone()
            .or_else(|| metadata.as_ref().and_then(|value| value.publisher.clone())),
        product_name: app.product_name.clone().or_else(|| {
            metadata
                .as_ref()
                .and_then(|value| value.product_name.clone())
        }),
        original_filename: app.original_filename.clone().or_else(|| {
            metadata
                .as_ref()
                .and_then(|value| value.original_filename.clone())
        }),
        install_location: app.install_location.clone().or_else(|| {
            Path::new(target)
                .parent()
                .map(|path| path.to_string_lossy().into_owned())
        }),
        can_uninstall: Some(app.uninstall.is_some()),
    };
    HydrationOutcome {
        patch,
        written_icon: icon
            .written_fingerprint
            .map(|fingerprint| (app.id.clone(), fingerprint)),
    }
}
