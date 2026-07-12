use crate::platform::windows::uninstaller::UninstallMechanism;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::cmp::Reverse;
use std::fs;
use std::io;
use std::path::Path;

const HISTORY_FILE: &str = "uninstall-history.json";
const HISTORY_TEMP_FILE: &str = "uninstall-history.json.tmp";
const MAX_HISTORY_ENTRIES: usize = 100;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UninstallResult {
    Succeeded,
    Failed,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UninstallHistoryEntry {
    pub id: String,
    pub timestamp: u64,
    pub app_name: String,
    pub publisher: Option<String>,
    pub mechanism: UninstallMechanism,
    pub result: UninstallResult,
}

pub fn read(app_data_dir: &Path) -> Vec<UninstallHistoryEntry> {
    let path = app_data_dir.join(HISTORY_FILE);
    let Ok(text) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let Ok(mut entries) = serde_json::from_str::<Vec<UninstallHistoryEntry>>(&text) else {
        return Vec::new();
    };
    sort_and_limit(&mut entries);
    entries
}

pub fn append(app_data_dir: &Path, mut entry: UninstallHistoryEntry) -> io::Result<()> {
    let mut entries = read(app_data_dir);
    if entry.id.trim().is_empty() {
        entry.id = entry_id(&entry);
    }
    entries.push(entry);
    sort_and_limit(&mut entries);
    write(app_data_dir, &entries)
}

pub fn clear(app_data_dir: &Path) -> io::Result<()> {
    write(app_data_dir, &[])
}

fn write(app_data_dir: &Path, entries: &[UninstallHistoryEntry]) -> io::Result<()> {
    fs::create_dir_all(app_data_dir)?;
    let path = app_data_dir.join(HISTORY_FILE);
    let temp_path = app_data_dir.join(HISTORY_TEMP_FILE);
    let json = serde_json::to_string_pretty(entries)?;
    fs::write(&temp_path, json)?;
    fs::rename(temp_path, path)
}

fn sort_and_limit(entries: &mut Vec<UninstallHistoryEntry>) {
    entries.sort_by_key(|entry| Reverse(entry.timestamp));
    entries.truncate(MAX_HISTORY_ENTRIES);
}

fn entry_id(entry: &UninstallHistoryEntry) -> String {
    let mut hasher = Sha256::new();
    hasher.update(entry.timestamp.to_le_bytes());
    hasher.update(entry.app_name.as_bytes());
    if let Some(publisher) = &entry.publisher {
        hasher.update(publisher.as_bytes());
    }
    hasher.update(format!("{:?}", entry.mechanism).as_bytes());
    hasher.update(format!("{:?}", entry.result).as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use crate::platform::windows::uninstaller::UninstallMechanism;
    use serde_json::Value;

    fn entry(index: u64) -> super::UninstallHistoryEntry {
        super::UninstallHistoryEntry {
            id: String::new(),
            timestamp: index,
            app_name: format!("App {index}"),
            publisher: Some("Publisher".into()),
            mechanism: UninstallMechanism::RegisteredCommand,
            result: super::UninstallResult::Succeeded,
        }
    }

    #[test]
    fn uninstall_history_retains_only_newest_100_entries() {
        let dir = tempfile::tempdir().unwrap();
        for index in 0..101 {
            super::append(dir.path(), entry(index)).unwrap();
        }
        let history = super::read(dir.path());
        assert_eq!(history.len(), 100);
        assert_eq!(history[0].timestamp, 100);
        assert_eq!(history[99].timestamp, 1);
    }

    #[test]
    fn uninstall_history_ignores_corrupt_json() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("uninstall-history.json"), "{broken").unwrap();
        assert!(super::read(dir.path()).is_empty());
    }

    #[test]
    fn uninstall_history_excludes_sensitive_fields() {
        let serialized = serde_json::to_value(entry(1)).unwrap();
        let object = serialized.as_object().unwrap();
        for key in [
            "command",
            "path",
            "arguments",
            "package",
            "error",
            "username",
        ] {
            assert!(!object.contains_key(key), "{key} must not be persisted");
        }
        assert_eq!(serialized["result"], Value::String("succeeded".into()));
    }
}
