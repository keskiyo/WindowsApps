use serde::{Deserialize, Serialize};
use std::path::Path;
use std::{fs, io};

const SETTINGS_FILE: &str = "scan-settings.json";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanSettings {
    pub auto_scan_fixed_drives: bool,
    #[serde(default)]
    pub included_paths: Vec<String>,
    #[serde(default)]
    pub excluded_paths: Vec<String>,
}

impl Default for ScanSettings {
    fn default() -> Self {
        Self {
            auto_scan_fixed_drives: true,
            included_paths: Vec::new(),
            excluded_paths: Vec::new(),
        }
    }
}

pub fn read(app_data_dir: &Path) -> ScanSettings {
    fs::read(app_data_dir.join(SETTINGS_FILE))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

pub fn write(app_data_dir: &Path, settings: &ScanSettings) -> io::Result<()> {
    fs::create_dir_all(app_data_dir)?;
    let bytes = serde_json::to_vec_pretty(settings).map_err(io::Error::other)?;
    let temporary = app_data_dir.join("scan-settings.json.tmp");
    fs::write(&temporary, bytes)?;
    fs::rename(temporary, app_data_dir.join(SETTINGS_FILE))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_scan_settings() {
        let dir = tempfile::tempdir().unwrap();
        let settings = ScanSettings {
            auto_scan_fixed_drives: true,
            included_paths: vec![r"E:\Portable".into()],
            excluded_paths: vec![r"D:\Archives".into()],
        };

        write(dir.path(), &settings).unwrap();

        assert_eq!(read(dir.path()), settings);
    }

    #[test]
    fn missing_or_invalid_settings_use_safe_defaults() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(read(dir.path()), ScanSettings::default());
        std::fs::write(dir.path().join(SETTINGS_FILE), "not json").unwrap();
        assert_eq!(read(dir.path()), ScanSettings::default());
    }
}
